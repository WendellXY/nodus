use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::resolver::{PackageSource, ResolvedPackage};

mod output;

pub mod agents;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod opencode;

pub(crate) use output::build_output_plan;

#[derive(Debug, Clone)]
pub struct ManagedFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Adapter {
    #[value(name = "agents")]
    Agents,
    #[value(name = "claude")]
    Claude,
    #[value(name = "codex")]
    Codex,
    #[value(name = "copilot")]
    Copilot,
    #[value(name = "cursor")]
    Cursor,
    #[value(name = "opencode", alias = "open-code")]
    OpenCode,
}

impl Adapter {
    pub const ALL: [Self; 6] = [
        Self::Agents,
        Self::Claude,
        Self::Codex,
        Self::Copilot,
        Self::Cursor,
        Self::OpenCode,
    ];

    const fn bit(self) -> u8 {
        match self {
            Self::Agents => 1 << 0,
            Self::Claude => 1 << 1,
            Self::Codex => 1 << 2,
            Self::Copilot => 1 << 3,
            Self::Cursor => 1 << 4,
            Self::OpenCode => 1 << 5,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Copilot => "copilot",
            Self::Cursor => "cursor",
            Self::OpenCode => "opencode",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Adapters(u8);

impl Adapters {
    pub const NONE: Self = Self(0);
    pub const AGENTS: Self = Self(Adapter::Agents.bit());
    pub const CLAUDE: Self = Self(Adapter::Claude.bit());
    pub const CODEX: Self = Self(Adapter::Codex.bit());
    pub const COPILOT: Self = Self(Adapter::Copilot.bit());
    pub const CURSOR: Self = Self(Adapter::Cursor.bit());
    pub const OPENCODE: Self = Self(Adapter::OpenCode.bit());

    pub const fn contains(self, adapter: Adapter) -> bool {
        self.0 & adapter.bit() != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[cfg(test)]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn from_slice(adapters: &[Adapter]) -> Self {
        adapters
            .iter()
            .copied()
            .fold(Self::NONE, |selected, adapter| {
                selected.union(adapter.into())
            })
    }

    pub fn to_vec(self) -> Vec<Adapter> {
        self.iter().collect()
    }

    pub fn iter(self) -> impl Iterator<Item = Adapter> {
        Adapter::ALL
            .into_iter()
            .filter(move |adapter| self.contains(*adapter))
    }
}

impl From<Adapter> for Adapters {
    fn from(value: Adapter) -> Self {
        Self(value.bit())
    }
}

impl std::fmt::Display for Adapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Skill,
    Agent,
    Rule,
    Command,
}

impl ArtifactKind {
    pub const fn supported_adapters(self) -> Adapters {
        match self {
            Self::Skill => Adapters::AGENTS
                .union(Adapters::CLAUDE)
                .union(Adapters::CODEX)
                .union(Adapters::COPILOT)
                .union(Adapters::CURSOR)
                .union(Adapters::OPENCODE),
            Self::Agent => Adapters::CLAUDE
                .union(Adapters::COPILOT)
                .union(Adapters::OPENCODE),
            Self::Rule => Adapters::CLAUDE
                .union(Adapters::CURSOR)
                .union(Adapters::OPENCODE),
            Self::Command => Adapters::AGENTS
                .union(Adapters::CLAUDE)
                .union(Adapters::CURSOR)
                .union(Adapters::OPENCODE),
        }
    }

    pub const fn plural_name(self) -> &'static str {
        match self {
            Self::Skill => "skills",
            Self::Agent => "agents",
            Self::Rule => "rules",
            Self::Command => "commands",
        }
    }
}

pub fn namespaced_skill_id(package: &ResolvedPackage, skill_id: &str) -> String {
    namespaced_artifact_id(package, skill_id)
}

pub fn runtime_root(project_root: &Path, adapter: Adapter) -> PathBuf {
    project_root.join(match adapter {
        Adapter::Agents => ".agents",
        Adapter::Claude => ".claude",
        Adapter::Codex => ".codex",
        Adapter::Copilot => ".github",
        Adapter::Cursor => ".cursor",
        Adapter::OpenCode => ".opencode",
    })
}

pub fn managed_skill_root(
    project_root: &Path,
    adapter: Adapter,
    package: &ResolvedPackage,
    skill_id: &str,
) -> PathBuf {
    runtime_root(project_root, adapter)
        .join("skills")
        .join(namespaced_skill_id(package, skill_id))
}

pub fn managed_artifact_path(
    project_root: &Path,
    adapter: Adapter,
    kind: ArtifactKind,
    package: &ResolvedPackage,
    artifact_id: &str,
) -> Option<PathBuf> {
    let runtime_root = runtime_root(project_root, adapter);
    match (adapter, kind) {
        (Adapter::Agents, ArtifactKind::Command) => Some(
            runtime_root
                .join("commands")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::Claude, ArtifactKind::Agent) => Some(
            runtime_root
                .join("agents")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::Claude, ArtifactKind::Command) => Some(
            runtime_root
                .join("commands")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::Copilot, ArtifactKind::Agent) => {
            Some(runtime_root.join("agents").join(namespaced_file_name(
                package,
                artifact_id,
                "agent.md",
            )))
        }
        (Adapter::Claude, ArtifactKind::Rule) => Some(
            runtime_root
                .join("rules")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::Cursor, ArtifactKind::Command) => Some(
            runtime_root
                .join("commands")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::Cursor, ArtifactKind::Rule) => Some(
            runtime_root
                .join("rules")
                .join(namespaced_file_name(package, artifact_id, "mdc")),
        ),
        (Adapter::OpenCode, ArtifactKind::Agent) => Some(
            runtime_root
                .join("agents")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::OpenCode, ArtifactKind::Command) => Some(
            runtime_root
                .join("commands")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        (Adapter::OpenCode, ArtifactKind::Rule) => Some(
            runtime_root
                .join("rules")
                .join(namespaced_file_name(package, artifact_id, "md")),
        ),
        _ => None,
    }
}

pub fn namespaced_artifact_id(package: &ResolvedPackage, artifact_id: &str) -> String {
    format!("{artifact_id}_{}", package_short_id(package))
}

pub fn namespaced_file_name(
    package: &ResolvedPackage,
    artifact_id: &str,
    extension: &str,
) -> String {
    format!(
        "{}.{}",
        namespaced_artifact_id(package, artifact_id),
        extension.trim_start_matches('.')
    )
}

pub fn package_short_id(package: &ResolvedPackage) -> String {
    match &package.source {
        PackageSource::Git { rev, .. } => short_source_id(rev),
        PackageSource::Path { .. } | PackageSource::Root => short_source_id(
            package
                .digest
                .strip_prefix("sha256:")
                .unwrap_or(&package.digest),
        ),
    }
}

pub fn short_source_id(value: &str) -> String {
    let short = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(6)
        .collect::<String>()
        .to_ascii_lowercase();

    if short.is_empty() {
        "local0".into()
    } else {
        short
    }
}
