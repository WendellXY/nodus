---
name: nodus-consumer
description: Use when a user wants to install, inspect, update, remove, or sync Nodus packages in a workspace. Prefer Nodus commands over manual copying of runtime files.
---

# Nodus Consumer

Help the user consume agent packages with Nodus in a predictable way. Treat `nodus.toml`, `nodus.lock`, and the managed runtime roots as the source of truth.

## Workflow

1. Identify the repo role.
   - If the repo is consuming packages, work from the root `nodus.toml`.
   - If the user points at a package repo directly, use `nodus info` first before changing anything.
2. Pick the dependency source and ref.
   - Prefer a Git tag for released packages.
   - Use `branch` only when the package intentionally tracks a moving head.
   - Use `revision` for exact reproducibility.
   - Use `path` for local development checkouts.
3. Choose the narrowest useful component set.
   - Install only `skills`, `agents`, `rules`, or `commands` that the repo actually needs.
   - Omit `components` when the repo should consume the full package.
4. Let Nodus manage runtime outputs.
   - Use `nodus add`, `nodus sync`, `nodus update`, `nodus remove`, and `nodus doctor`.
   - Do not manually copy files into `.agents/`, `.claude/`, `.codex/`, `.cursor/`, or `.opencode/`.
5. Verify the final state.
   - Run `nodus doctor` after changes.
   - Use `nodus info <alias-or-package>` to confirm selected components, discovered artifacts, and source pins.

## Common Commands

```bash
nodus add <package> --adapter <adapter>
nodus add <package> --adapter <adapter> --component skills --component rules
nodus info <package-or-alias>
nodus sync
nodus sync --locked
nodus sync --frozen
nodus doctor
nodus remove <alias>
nodus update
```

## Decision Rules

- Prefer stable aliases because `nodus remove` and `nodus relay` use them directly.
- Prefer `--locked` in CI when the workspace should already be synchronized.
- Use `--frozen` only when the existing `nodus.lock` must be installed exactly as written.
- If a package declares high-sensitivity capabilities, explain why `--allow-high-sensitivity` is needed before using it.
- If relay links exist, do not overwrite pending relayed edits; inspect relay state first.
