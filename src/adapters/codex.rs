use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::adapters::{ManagedArtifactNames, ManagedFile, managed_skill_root};
use crate::manifest::SkillEntry;
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
            crate::adapters::Adapter::Codex,
            package,
            &skill.id,
        ),
        snapshot_root.join(&skill.path),
    )
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

pub fn sync_on_startup_files(project_root: &Path) -> Result<Vec<ManagedFile>> {
    let hooks_path = project_root.join(".codex/hooks.json");
    Ok(vec![
        ManagedFile {
            path: project_root.join(".codex/hooks/nodus-sync.sh"),
            contents: sync_script_contents(),
        },
        ManagedFile {
            path: hooks_path.clone(),
            contents: merged_hooks_contents(&hooks_path)?,
        },
    ])
}

fn merged_hooks_contents(path: &Path) -> Result<Vec<u8>> {
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
    let already_present = session_start.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|hooks| {
                hooks.iter().any(|hook| {
                    hook.get("type").and_then(Value::as_str) == Some("command")
                        && hook.get("command").and_then(Value::as_str) == Some(sync_hook_command())
                })
            })
    });
    if !already_present {
        session_start.push(json!({
            "matcher": "startup|resume",
            "hooks": [sync_hook_value()],
        }));
    }

    let mut contents =
        serde_json::to_vec_pretty(&root).context("failed to serialize Codex hooks")?;
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
        "command": sync_hook_command(),
    })
}

fn sync_hook_command() -> &'static str {
    r#"sh "$(git rev-parse --show-toplevel 2>/dev/null || pwd)/.codex/hooks/nodus-sync.sh""#
}

fn sync_script_contents() -> Vec<u8> {
    br#"#!/bin/sh
set -eu

project_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

if ! command -v nodus >/dev/null 2>&1; then
  echo "nodus not found on PATH; skipping startup sync" >&2
  exit 0
fi

cd "$project_root"
if ! nodus sync >/dev/null 2>&1; then
  echo "nodus sync failed in $project_root" >&2
fi
"#
    .to_vec()
}
