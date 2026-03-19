---
name: nodus-package-author
description: Use when a repo is authoring or restructuring a Nodus package. Covers package layout, nodus.toml fields, additive content roots, and validation expectations.
---

# Nodus Package Author

Help maintainers build a Nodus package that can be consumed cleanly by other workspaces. Start from the package layout and manifest invariants, not from adapter-specific folder copying.

## Workflow

1. Model the package shape.
   - Decide whether the repo root itself is the package root or whether the repo should expose extra package roots through `content_roots`.
   - Keep artifact ids stable because Nodus uses them for managed output names and lockfile tracking.
2. Lay out artifacts under package roots.
   - `skills/<id>/SKILL.md`
   - `agents/<id>.md`
   - `rules/<id>.*`
   - `commands/<id>.*`
3. Add manifest configuration only where it changes behavior.
   - Use `content_roots = [...]` for additive package roots.
   - Use `publish_root = true` only when the repo should publish its own discovered assets during local `nodus sync`.
   - Add dependencies only when this package composes other packages.
4. Validate from the package root.
   - Use `nodus info .` to inspect the discovered package.
   - Use `nodus doctor` in a consuming workspace to verify emitted outputs and lockfile state.
5. Keep the package deterministic.
   - Prefer explicit tags for shipped Git dependencies.
   - Avoid duplicate artifact ids across multiple package roots.

## Authoring Rules

- `content_roots` entries must be relative, normalized, and stay inside the repo root.
- Package discovery is additive: the repo root is always scanned, then each configured content root is scanned too.
- Discovery paths stay repo-relative even when the artifacts live under a configured content root.
- Duplicate ids across roots are invalid, even if they come from different directories.
- Use concise, action-oriented instructions inside skills, agents, commands, and rules.
