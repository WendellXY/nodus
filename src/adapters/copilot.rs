use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::adapters::{
    ArtifactKind, ManagedArtifactNames, ManagedFile, managed_artifact_path, managed_skill_id,
    managed_skill_root,
};
use crate::agent_format::markdown_from_codex_agent_toml;
use crate::manifest::{AgentEntry, SkillEntry};
use crate::paths::strip_path_prefix;
use crate::resolver::ResolvedPackage;

pub fn skill_files(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    skill: &SkillEntry,
) -> Result<Vec<ManagedFile>> {
    let source_root = snapshot_root.join(&skill.path);
    let managed_skill_id = managed_skill_id(names, package, &skill.id);
    let target_root = managed_skill_root(
        names,
        project_root,
        crate::adapters::Adapter::Copilot,
        package,
        &skill.id,
    );
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(&source_root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative = entry.path();
            let relative = strip_path_prefix(relative, &source_root)
                .with_context(|| format!("failed to make {} relative", entry.path().display()))?;
            let contents = fs::read(entry.path()).with_context(|| {
                format!("failed to read snapshot file {}", entry.path().display())
            })?;
            let contents = if relative == Path::new("SKILL.md") {
                rewrite_skill_name(&contents, &managed_skill_id)?
            } else {
                contents
            };
            files.push(ManagedFile {
                path: target_root.join(relative),
                contents,
            });
        }
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub fn agent_file(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    agent: &AgentEntry,
) -> Result<ManagedFile> {
    let target_path = managed_artifact_path(
        names,
        project_root,
        crate::adapters::Adapter::Copilot,
        ArtifactKind::Agent,
        package,
        &agent.id,
    )
    .expect("copilot agent path");
    let source_path = snapshot_root.join(&agent.path);
    let contents = fs::read(&source_path)
        .with_context(|| format!("failed to read snapshot file {}", source_path.display()))?;
    let contents = if agent.is_toml() {
        markdown_from_codex_agent_toml(
            &contents,
            &format!("GitHub Copilot agent source {}", source_path.display()),
        )?
    } else {
        contents
    };
    Ok(ManagedFile {
        path: target_path,
        contents,
    })
}

pub(crate) fn rewrite_skill_name(contents: &[u8], skill_id: &str) -> Result<Vec<u8>> {
    let contents =
        String::from_utf8(contents.to_vec()).context("GitHub Copilot skills must be UTF-8")?;
    let mut lines = split_lines_preserving_endings(&contents);

    if lines.first().map(|line| trim_line_ending(line)) != Some("---") {
        bail!(
            "GitHub Copilot skill {} is missing YAML frontmatter",
            skill_id
        );
    }

    let Some(frontmatter_end) = lines
        .iter()
        .skip(1)
        .position(|line| trim_line_ending(line) == "---")
    else {
        bail!(
            "GitHub Copilot skill {} is missing a closing frontmatter fence",
            skill_id
        );
    };
    let frontmatter_end = frontmatter_end + 1;

    if let Some(name_index) = lines
        .iter()
        .take(frontmatter_end)
        .position(|line| trim_line_ending(line).trim_start().starts_with("name:"))
    {
        lines[name_index] = rewrite_frontmatter_name_line(&lines[name_index], skill_id);
    } else {
        lines.insert(
            frontmatter_end,
            inserted_frontmatter_name_line(&lines, frontmatter_end, skill_id),
        );
    }
    Ok(lines.concat().into_bytes())
}

fn split_lines_preserving_endings(contents: &str) -> Vec<String> {
    if contents.is_empty() {
        Vec::new()
    } else {
        contents.split_inclusive('\n').map(str::to_string).collect()
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn inserted_frontmatter_name_line(lines: &[String], frontmatter_end: usize, name: &str) -> String {
    format!(
        "name: {name}{}",
        preferred_line_ending(lines, frontmatter_end)
    )
}

fn preferred_line_ending(lines: &[String], anchor: usize) -> &str {
    line_ending(lines.get(anchor).map(String::as_str).unwrap_or_default())
        .or_else(|| {
            anchor
                .checked_sub(1)
                .and_then(|index| lines.get(index))
                .and_then(|line| line_ending(line))
        })
        .unwrap_or("\n")
}

fn line_ending(line: &str) -> Option<&str> {
    if line.ends_with("\r\n") {
        Some("\r\n")
    } else if line.ends_with('\n') {
        Some("\n")
    } else {
        None
    }
}

fn rewrite_frontmatter_name_line(line: &str, name: &str) -> String {
    let leading = line
        .chars()
        .take_while(|character| character.is_ascii_whitespace())
        .collect::<String>();
    let newline = if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    };

    format!("{leading}name: {name}{newline}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_skill_name_to_match_runtime_id() {
        let contents = b"---\nname: Review\ndescription: Example\n---\n# Review\n".as_slice();
        let rewritten = rewrite_skill_name(contents, "review").unwrap();
        let rewritten = String::from_utf8(rewritten).unwrap();
        assert!(rewritten.contains("name: review"));
        assert!(rewritten.contains("description: Example"));
        assert!(rewritten.ends_with('\n'));
    }

    #[test]
    fn preserves_crlf_when_rewriting_skill_name() {
        let contents =
            b"---\r\nname: Review\r\ndescription: Example\r\n---\r\n# Review\r\n".as_slice();
        let rewritten = rewrite_skill_name(contents, "review").unwrap();
        let rewritten = String::from_utf8(rewritten).unwrap();

        assert!(rewritten.contains("name: review\r\n"));
        assert!(rewritten.contains("description: Example\r\n"));
        assert!(rewritten.ends_with("\r\n"));
    }

    #[test]
    fn inserts_missing_skill_name_into_frontmatter() {
        let contents = b"---\ndescription: Example\n---\n# Review\n".as_slice();
        let rewritten = rewrite_skill_name(contents, "review").unwrap();
        let rewritten = String::from_utf8(rewritten).unwrap();

        assert!(rewritten.contains("name: review\n"));
        assert!(rewritten.contains("description: Example\n"));
    }
}
