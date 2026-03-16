use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

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
                        .join(".claude/skills")
                        .join(&skill.id)
                        .join(relative),
                    contents: fs::read(entry.path()).with_context(|| {
                        format!("failed to read snapshot file {}", entry.path().display())
                    })?,
                });
            }
        }
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}
