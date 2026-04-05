use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::domain::dependency_query::{resolve_direct_dependency, resolve_inspection_target};
use crate::execution::ExecutionMode;
use crate::manifest::{
    DependencyKind, DependencySpec, LoadedManifest, PackageRole, load_root_from_dir,
};
use crate::report::Reporter;
use crate::resolver::sync_in_dir_with_loaded_root;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MembersOperation {
    Enable,
    Disable,
    Set,
}

#[derive(Debug, Clone)]
pub(crate) struct MemberStatus {
    pub(crate) id: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MembersSummary {
    pub(crate) alias: String,
    pub(crate) kind: DependencyKind,
    pub(crate) dependency_preview: String,
    pub(crate) members: Vec<MemberStatus>,
}

#[derive(Debug, Clone)]
pub(crate) struct MembersUpdateSummary {
    pub(crate) alias: String,
    pub(crate) kind: DependencyKind,
    pub(crate) members: MembersSummary,
    pub(crate) managed_file_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MembersUpdateRequest<'a> {
    pub(crate) package: &'a str,
    pub(crate) requested_members: &'a [String],
    pub(crate) operation: MembersOperation,
    pub(crate) allow_high_sensitivity: bool,
    pub(crate) dry_run: bool,
}

pub(crate) fn list_dependency_members_in_dir(
    cwd: &Path,
    cache_root: &Path,
    package: Option<&str>,
) -> Result<Vec<MembersSummary>> {
    if let Some(package) = package {
        let (alias, dependency, root_manifest) = resolve_direct_dependency(cwd, package)?
            .ok_or_else(|| anyhow!("dependency `{package}` does not exist"))?;
        let kind = root_manifest
            .manifest
            .dependency_kind(&alias)
            .ok_or_else(|| anyhow!("dependency `{alias}` does not exist"))?;
        let summary = members_summary_for_dependency(
            &alias,
            kind,
            &dependency,
            &root_manifest,
            cache_root,
            &Reporter::silent(),
        )?
        .ok_or_else(|| anyhow!("dependency `{alias}` does not expose selectable child packages"))?;
        return Ok(vec![summary]);
    }

    let root = load_root_from_dir(cwd)?;
    let mut summaries = root
        .manifest
        .all_dependency_entries()
        .into_iter()
        .filter_map(|entry| {
            members_summary_for_dependency(
                entry.alias,
                entry.kind,
                entry.spec,
                &root,
                cache_root,
                &Reporter::silent(),
            )
            .transpose()
        })
        .collect::<Result<Vec<_>>>()?;
    summaries.sort_by(|left, right| left.alias.cmp(&right.alias));
    Ok(summaries)
}

pub(crate) fn update_dependency_members_in_dir(
    cwd: &Path,
    cache_root: &Path,
    request: MembersUpdateRequest<'_>,
    reporter: &Reporter,
) -> Result<MembersUpdateSummary> {
    let MembersUpdateRequest {
        package,
        requested_members,
        operation,
        allow_high_sensitivity,
        dry_run,
    } = request;
    crate::relay::ensure_no_pending_relay_edits_in_dir(cwd, cache_root)?;
    let (alias, _, mut root_manifest) = resolve_direct_dependency(cwd, package)?
        .ok_or_else(|| anyhow!("dependency `{package}` does not exist"))?;
    let kind = root_manifest
        .manifest
        .dependency_kind(&alias)
        .ok_or_else(|| anyhow!("dependency `{alias}` does not exist"))?;
    let dependency = root_manifest
        .manifest
        .get_dependency(&alias)
        .ok_or_else(|| anyhow!("dependency `{alias}` does not exist"))?
        .spec
        .clone();
    let target =
        resolve_inspection_target(&alias, &dependency, &root_manifest, cache_root, reporter)?;
    let available = selectable_member_ids(&target.manifest, target.role)?;
    if available.is_empty() {
        bail!("dependency `{alias}` does not expose selectable child packages");
    }
    let available_set = available.iter().cloned().collect::<BTreeSet<_>>();

    let requested = normalized_requested_members(requested_members);
    for member in &requested {
        if !available_set.contains(member) {
            bail!(
                "dependency `{alias}` does not expose child package `{member}`; available: {}",
                available.join(", ")
            );
        }
    }

    let current = dependency
        .explicit_members_sorted()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let next = match operation {
        MembersOperation::Set => requested,
        MembersOperation::Enable => current.union(&requested).cloned().collect(),
        MembersOperation::Disable => current.difference(&requested).cloned().collect(),
    };

    let mut dependency = dependency;
    dependency.members = if next.is_empty() {
        None
    } else {
        Some(next.iter().cloned().collect())
    };
    root_manifest
        .manifest
        .dependency_section_mut(kind)
        .insert(alias.clone(), dependency.clone());
    let root_manifest =
        root_manifest.with_manifest(root_manifest.manifest.clone(), PackageRole::Root)?;
    let sync_summary = sync_in_dir_with_loaded_root(
        cwd,
        cache_root,
        false,
        allow_high_sensitivity,
        false,
        &[],
        false,
        if dry_run {
            ExecutionMode::DryRun
        } else {
            ExecutionMode::Apply
        },
        root_manifest,
        reporter,
    )?;

    let members = build_members_summary(&alias, kind, &dependency, available, &next);

    Ok(MembersUpdateSummary {
        alias,
        kind,
        members,
        managed_file_count: sync_summary.managed_file_count,
    })
}

fn members_summary_for_dependency(
    alias: &str,
    kind: DependencyKind,
    dependency: &DependencySpec,
    root_manifest: &LoadedManifest,
    cache_root: &Path,
    reporter: &Reporter,
) -> Result<Option<MembersSummary>> {
    let target = resolve_inspection_target(alias, dependency, root_manifest, cache_root, reporter)?;
    let available = selectable_member_ids(&target.manifest, target.role)?;
    if available.is_empty() {
        return Ok(None);
    }
    let selected = dependency
        .explicit_members_sorted()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    Ok(Some(build_members_summary(
        alias, kind, dependency, available, &selected,
    )))
}

fn selectable_member_ids(manifest: &LoadedManifest, role: PackageRole) -> Result<Vec<String>> {
    let workspace_members = manifest
        .resolved_workspace_members()?
        .into_iter()
        .map(|member| member.id)
        .collect::<Vec<_>>();
    if !workspace_members.is_empty() {
        return Ok(workspace_members);
    }

    let dependencies = manifest.manifest.active_dependency_entries_for_role(role);
    if dependencies.is_empty()
        || role == PackageRole::Root
        || manifest.manifest.workspace.is_some()
        || !manifest.discovered.is_empty()
    {
        return Ok(Vec::new());
    }

    Ok(dependencies
        .into_iter()
        .map(|entry| entry.alias.to_string())
        .collect())
}

fn normalized_requested_members(members: &[String]) -> BTreeSet<String> {
    members
        .iter()
        .map(|member| member.trim())
        .filter(|member| !member.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn build_members_summary(
    alias: &str,
    kind: DependencyKind,
    dependency: &DependencySpec,
    available: Vec<String>,
    selected: &BTreeSet<String>,
) -> MembersSummary {
    MembersSummary {
        alias: alias.to_string(),
        kind,
        dependency_preview: format!("{alias} = {{ {} }}", dependency.inline_fields().join(", ")),
        members: available
            .into_iter()
            .map(|id| MemberStatus {
                enabled: selected.contains(&id),
                id,
            })
            .collect(),
    }
}
