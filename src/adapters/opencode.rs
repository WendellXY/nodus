use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::adapters::ManagedFile;
use crate::resolver::ResolvedPackage;

#[derive(Debug, Default)]
pub struct OpenCodeOutputs {
    pub files: Vec<ManagedFile>,
    pub instructions: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OpenCodeConfig {
    #[serde(default)]
    instructions: Vec<String>,
}

pub fn managed_files(
    project_root: &Path,
    package: &ResolvedPackage,
    snapshot_root: &Path,
) -> Result<OpenCodeOutputs> {
    let mut files = Vec::new();
    let mut instructions = Vec::new();

    for agent in &package.manifest.manifest.exports.agents {
        let target_relative = format!(".opencode/instructions/{}.md", agent.id);
        let source_path = snapshot_root.join(&agent.path);
        files.push(ManagedFile {
            path: project_root.join(&target_relative),
            contents: fs::read(&source_path).with_context(|| {
                format!("failed to read snapshot file {}", source_path.display())
            })?,
        });
        instructions.push(target_relative);
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    instructions.sort();
    instructions.dedup();

    Ok(OpenCodeOutputs {
        files,
        instructions,
    })
}

pub fn render_config(instructions: &[String]) -> Result<Vec<u8>> {
    let config = OpenCodeConfig {
        instructions: instructions.to_vec(),
    };
    serde_json::to_vec_pretty(&config).context("failed to serialize OpenCode config")
}
