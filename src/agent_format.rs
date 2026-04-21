use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use toml::Value as TomlValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RawCodexAgentConfig {
    name: String,
    description: String,
    developer_instructions: String,
    #[serde(flatten)]
    extra: BTreeMap<String, TomlValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CodexAgentConfig {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) developer_instructions: String,
    pub(crate) extra: BTreeMap<String, TomlValue>,
}

pub(crate) fn parse_codex_agent_config(bytes: &[u8], context: &str) -> Result<CodexAgentConfig> {
    let contents = String::from_utf8(bytes.to_vec())
        .with_context(|| format!("{context} must be valid UTF-8"))?;
    let raw: RawCodexAgentConfig =
        toml::from_str(&contents).with_context(|| format!("failed to parse {context} as TOML"))?;
    validate_codex_agent_fields(&raw, context)?;
    Ok(CodexAgentConfig {
        name: raw.name,
        description: raw.description,
        developer_instructions: raw.developer_instructions,
        extra: raw.extra,
    })
}

pub(crate) fn serialize_codex_agent_config(config: &CodexAgentConfig) -> Result<Vec<u8>> {
    let mut contents = toml::to_string_pretty(&RawCodexAgentConfig {
        name: config.name.clone(),
        description: config.description.clone(),
        developer_instructions: config.developer_instructions.clone(),
        extra: config.extra.clone(),
    })
    .context("failed to serialize Codex agent TOML")?;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    Ok(contents.into_bytes())
}

pub(crate) fn emitted_codex_agent_toml(
    source_toml: &[u8],
    runtime_name: Option<&str>,
    context: &str,
) -> Result<Vec<u8>> {
    let mut config = parse_codex_agent_config(source_toml, context)?;
    if let Some(runtime_name) = runtime_name {
        config.name = runtime_name.to_string();
    }
    serialize_codex_agent_config(&config)
}

pub(crate) fn emitted_codex_agent_toml_from_markdown(
    source_markdown: &[u8],
    runtime_name: &str,
    description: &str,
    context: &str,
) -> Result<Vec<u8>> {
    let developer_instructions = String::from_utf8(source_markdown.to_vec())
        .with_context(|| format!("{context} must be valid UTF-8"))?;
    serialize_codex_agent_config(&CodexAgentConfig {
        name: runtime_name.to_string(),
        description: description.to_string(),
        developer_instructions,
        extra: BTreeMap::new(),
    })
}

pub(crate) fn markdown_from_codex_agent_toml(source_toml: &[u8], context: &str) -> Result<Vec<u8>> {
    Ok(parse_codex_agent_config(source_toml, context)?
        .developer_instructions
        .into_bytes())
}

pub(crate) fn source_toml_from_managed_markdown(
    managed_markdown: &[u8],
    baseline_toml: &[u8],
    context: &str,
) -> Result<Vec<u8>> {
    let mut config = parse_codex_agent_config(baseline_toml, context)?;
    config.developer_instructions = String::from_utf8(managed_markdown.to_vec())
        .with_context(|| format!("{context} developer instructions must be valid UTF-8"))?;
    serialize_codex_agent_config(&config)
}

pub(crate) fn source_toml_from_managed_codex(
    managed_toml: &[u8],
    baseline_toml: Option<&[u8]>,
    emitted_runtime_name: &str,
    context: &str,
) -> Result<Vec<u8>> {
    let mut config = parse_codex_agent_config(managed_toml, context)?;
    if let Some(baseline_toml) = baseline_toml {
        let baseline = parse_codex_agent_config(baseline_toml, context)?;
        if config.name == emitted_runtime_name && baseline.name != emitted_runtime_name {
            config.name = baseline.name;
        }
    }
    serialize_codex_agent_config(&config)
}

pub(crate) fn default_codex_agent_description(agent_id: &str) -> String {
    format!("Instructions for the `{agent_id}` agent.")
}

fn validate_codex_agent_fields(config: &RawCodexAgentConfig, context: &str) -> Result<()> {
    if config.name.trim().is_empty() {
        bail!("{context} field `name` must not be empty");
    }
    if config.description.trim().is_empty() {
        bail!("{context} field `description` must not be empty");
    }
    if config.developer_instructions.trim().is_empty() {
        bail!("{context} field `developer_instructions` must not be empty");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_serializes_codex_agent_toml() {
        let source = br#"name = "security"
description = "Review security-sensitive code."
developer_instructions = "Be careful."
model = "gpt-5"
"#;

        let config = parse_codex_agent_config(source, "agent").unwrap();
        assert_eq!(config.name, "security");
        assert_eq!(config.description, "Review security-sensitive code.");
        assert_eq!(config.developer_instructions, "Be careful.");
        assert_eq!(
            config.extra.get("model"),
            Some(&TomlValue::String("gpt-5".into()))
        );

        let serialized = String::from_utf8(serialize_codex_agent_config(&config).unwrap()).unwrap();
        assert!(serialized.contains("name = \"security\""));
        assert!(serialized.contains("model = \"gpt-5\""));
        assert!(serialized.ends_with('\n'));
    }

    #[test]
    fn restores_source_name_when_runtime_name_was_only_a_collision_rewrite() {
        let baseline = br#"name = "Security reviewer"
description = "Review security-sensitive code."
developer_instructions = "Be careful."
"#;
        let managed = br#"name = "security_abc123"
description = "Review security-sensitive code."
developer_instructions = "Be extra careful."
"#;

        let restored = String::from_utf8(
            source_toml_from_managed_codex(managed, Some(baseline), "security_abc123", "agent")
                .unwrap(),
        )
        .unwrap();

        assert!(restored.contains("name = \"Security reviewer\""));
        assert!(restored.contains("Be extra careful."));
    }
}
