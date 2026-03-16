use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::adapters::ManagedFile;
use crate::resolver::ResolvedPackage;

pub fn managed_files(
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
) -> Result<Vec<ManagedFile>> {
    let mut files = Vec::new();

    for skill in &package.manifest.manifest.exports.skills {
        let source_root = snapshot_root.join(&skill.path);
        for entry in walkdir::WalkDir::new(&source_root) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let relative = entry.path().strip_prefix(&source_root).with_context(|| {
                    format!("failed to make {} relative", entry.path().display())
                })?;
                files.push(ManagedFile {
                    path: project_root
                        .join(".codex/skills")
                        .join(&skill.id)
                        .join(relative),
                    contents: fs::read(entry.path()).with_context(|| {
                        format!("failed to read snapshot file {}", entry.path().display())
                    })?,
                });
            }
        }
    }

    for rule in &package.manifest.manifest.exports.rules {
        let mut codex_sources = rule
            .sources
            .iter()
            .filter(|source| source.kind == "codex.ruleset");
        let Some(source) = codex_sources.next() else {
            continue;
        };
        if codex_sources.next().is_some() {
            bail!(
                "rule export `{}` in package `{}` has multiple codex.ruleset sources",
                rule.id,
                package.manifest.manifest.name
            );
        }

        let source_path = snapshot_root.join(&source.path);
        files.push(ManagedFile {
            path: project_root
                .join(".codex/rules")
                .join(format!("{}.rules", rule.id)),
            contents: fs::read(&source_path).with_context(|| {
                format!("failed to read snapshot file {}", source_path.display())
            })?,
        });
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}
