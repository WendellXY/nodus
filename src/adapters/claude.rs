use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::adapters::{
    ArtifactKind, ManagedArtifactNames, ManagedFile, managed_artifact_path, managed_skill_root,
};
use crate::manifest::{FileEntry, SkillEntry};
use crate::paths::strip_path_prefix;
use crate::resolver::ResolvedPackage;

pub fn skill_files(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    skill: &SkillEntry,
) -> Result<Vec<ManagedFile>> {
    copy_directory(
        managed_skill_root(
            names,
            project_root,
            crate::adapters::Adapter::Claude,
            package,
            &skill.id,
        ),
        snapshot_root.join(&skill.path),
    )
}

pub fn agent_file(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    agent: &FileEntry,
) -> Result<ManagedFile> {
    copy_file(
        managed_artifact_path(
            names,
            project_root,
            crate::adapters::Adapter::Claude,
            ArtifactKind::Agent,
            package,
            &agent.id,
        )
        .expect("claude agent path"),
        snapshot_root.join(&agent.path),
    )
}

pub fn command_file(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    command: &FileEntry,
) -> Result<ManagedFile> {
    copy_file(
        managed_artifact_path(
            names,
            project_root,
            crate::adapters::Adapter::Claude,
            ArtifactKind::Command,
            package,
            &command.id,
        )
        .expect("claude command path"),
        snapshot_root.join(&command.path),
    )
}

pub fn rule_file(
    names: &ManagedArtifactNames,
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    rule: &FileEntry,
) -> Result<ManagedFile> {
    copy_file(
        managed_artifact_path(
            names,
            project_root,
            crate::adapters::Adapter::Claude,
            ArtifactKind::Rule,
            package,
            &rule.id,
        )
        .expect("claude rule path"),
        snapshot_root.join(&rule.path),
    )
}

pub fn sync_on_startup_files(project_root: &Path) -> Result<Vec<ManagedFile>> {
    let settings_path = project_root.join(".claude/settings.local.json");
    Ok(vec![
        ManagedFile {
            path: project_root.join(".claude/hooks/nodus-sync.sh"),
            contents: sync_script_contents("CLAUDE_PROJECT_DIR"),
        },
        ManagedFile {
            path: settings_path.clone(),
            contents: merged_settings_local_contents(&settings_path)?,
        },
    ])
}

fn copy_directory(
    target_root: impl AsRef<Path>,
    source_root: impl AsRef<Path>,
) -> Result<Vec<ManagedFile>> {
    let target_root = target_root.as_ref();
    let source_root = source_root.as_ref();
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(source_root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative = entry.path();
            let relative = strip_path_prefix(relative, source_root)
                .with_context(|| format!("failed to make {} relative", entry.path().display()))?;
            files.push(ManagedFile {
                path: target_root.join(relative),
                contents: fs::read(entry.path()).with_context(|| {
                    format!("failed to read snapshot file {}", entry.path().display())
                })?,
            });
        }
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn copy_file(target_path: impl AsRef<Path>, source_path: impl AsRef<Path>) -> Result<ManagedFile> {
    let target_path = target_path.as_ref();
    let source_path = source_path.as_ref();
    Ok(ManagedFile {
        path: target_path.to_path_buf(),
        contents: fs::read(source_path)
            .with_context(|| format!("failed to read snapshot file {}", source_path.display()))?,
    })
}

fn sync_script_contents(project_dir_env: &str) -> Vec<u8> {
    format!(
        r#"#!/bin/sh
set -eu

project_root="${{{project_dir_env}:-$(pwd)}}"

if ! command -v nodus >/dev/null 2>&1; then
  echo "nodus not found on PATH; skipping startup sync" >&2
  exit 0
fi

cd "$project_root"
if ! nodus sync >/dev/null 2>&1; then
  echo "nodus sync failed in $project_root" >&2
fi
"#
    )
    .into_bytes()
}

fn merged_settings_local_contents(path: &Path) -> Result<Vec<u8>> {
    let mut root = if path.exists() {
        serde_json::from_slice::<Value>(
            &fs::read(path)
                .with_context(|| format!("failed to read existing {}", path.display()))?,
        )
        .with_context(|| format!("failed to parse existing {}", path.display()))?
    } else {
        Value::Object(Map::new())
    };

    let root_object = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{} must contain a JSON object", path.display()))?;
    let hooks = object_field(root_object, "hooks", path)?;
    let session_start = array_field(hooks, "SessionStart", path)?;

    if let Some(existing) = session_start.iter_mut().find(|entry| {
        entry
            .get("matcher")
            .and_then(Value::as_str)
            .is_some_and(|matcher| matcher == "startup")
    }) {
        let existing_hooks = array_field(
            existing.as_object_mut().ok_or_else(|| {
                anyhow::anyhow!(
                    "{} hooks.SessionStart entries must contain JSON objects",
                    path.display()
                )
            })?,
            "hooks",
            path,
        )?;

        let already_present = existing_hooks.iter().any(|hook| {
            hook.get("type").and_then(Value::as_str) == Some("command")
                && hook.get("command").and_then(Value::as_str)
                    == Some("./.claude/hooks/nodus-sync.sh")
        });
        if !already_present {
            existing_hooks.push(sync_hook_value());
        }
    } else {
        session_start.push(json!({
            "matcher": "startup",
            "hooks": [sync_hook_value()],
        }));
    }

    let mut contents =
        serde_json::to_vec_pretty(&root).context("failed to serialize Claude settings")?;
    contents.push(b'\n');
    Ok(contents)
}

fn object_field<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<&'a mut Map<String, Value>> {
    let value = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    value.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!(
            "{} field `{key}` must contain a JSON object",
            path.display()
        )
    })
}

fn array_field<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<&'a mut Vec<Value>> {
    let value = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    value.as_array_mut().ok_or_else(|| {
        anyhow::anyhow!("{} field `{key}` must contain a JSON array", path.display())
    })
}

fn sync_hook_value() -> Value {
    json!({
        "type": "command",
        "command": "./.claude/hooks/nodus-sync.sh",
    })
}
