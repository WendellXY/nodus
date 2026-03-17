use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::adapters::{ArtifactKind, ManagedFile, managed_artifact_path};
use crate::manifest::FileEntry;
use crate::resolver::ResolvedPackage;

pub fn rule_file(
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
    rule: &FileEntry,
) -> Result<ManagedFile> {
    copy_file(
        managed_artifact_path(
            project_root,
            crate::adapters::Adapter::Codex,
            ArtifactKind::Rule,
            package,
            &rule.id,
        )
        .expect("codex rule path"),
        snapshot_root.join(&rule.path),
    )
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
