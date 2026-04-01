pub(super) const ROOT_LONG_ABOUT: &str = r#"Nodus adds AI agent packages to this repo and keeps the generated tool files in sync.

Most common tasks:
  nodus add nodus-rs/nodus --adapter codex
  nodus doctor
  nodus sync
  nodus update

Typical workflows:
  first install: add -> doctor
  rebuild current setup: sync -> doctor
  upgrade packages: update -> doctor
  remove a package: remove -> doctor
"#;

pub(super) const ROOT_AFTER_LONG_HELP: &str = r#"Need details? Run `nodus <command> --help` for examples and flag details.
"#;

pub(super) const ADD_LONG_ABOUT: &str = r#"Install one package into this repo and immediately write the managed files the selected AI tool needs.

Most common use:
  nodus add nodus-rs/nodus --adapter codex

What this changes:
  - creates or updates `nodus.toml`
  - resolves and records exact package revisions in `nodus.lock`
  - writes managed files under tool folders such as `.codex/` or `.claude/`

Run `nodus doctor` next to verify the repo is healthy."#;

pub(super) const ADD_AFTER_LONG_HELP: &str = r#"Examples:
  nodus add nodus-rs/nodus --adapter codex
  nodus add ./vendor/playbook --adapter claude
  nodus add owner/repo --tag v1.2.3 --adapter codex
  nodus add owner/marketplace --accept-all-dependencies --adapter codex
  nodus add owner/repo --global --adapter codex

After a project-scoped install, run `nodus doctor` to confirm the repo is consistent."#;

pub(super) const UPDATE_LONG_ABOUT: &str = r#"Resolve newer allowed versions for configured dependencies, rewrite `nodus.lock`, and sync managed outputs to match the new result.

Use `nodus update` when you want newer package revisions. Use `nodus sync` when you only want to rebuild from the versions you already have recorded."#;

pub(super) const UPDATE_AFTER_LONG_HELP: &str = r#"Examples:
  nodus update
  nodus update --dry-run
  nodus update --allow-high-sensitivity"#;

pub(super) const SYNC_LONG_ABOUT: &str = r#"Resolve the dependencies already declared in `nodus.toml` and write the managed adapter outputs that should exist for the current repo.

Use `nodus sync` after manifest changes, after editing package content locally, or when you want to rebuild outputs without upgrading dependencies."#;

pub(super) const SYNC_AFTER_LONG_HELP: &str = r#"Examples:
  nodus sync
  nodus sync --locked
  nodus sync --frozen
  nodus sync --force

Use `--locked` when the lockfile must stay unchanged. Use `--frozen` when installs must come exactly from the existing `nodus.lock`."#;

pub(super) const DOCTOR_LONG_ABOUT: &str = r#"Validate that `nodus.toml`, `nodus.lock`, the shared store, and the managed adapter outputs are still in sync.

Run this after `nodus add`, `nodus sync`, `nodus update`, or `nodus remove` when you want a final health check."#;

pub(super) const DOCTOR_AFTER_LONG_HELP: &str = r#"Examples:
  nodus doctor
  nodus doctor --json"#;
