# Sync And Verify A Workspace

Refresh managed outputs from `nodus.toml`, then verify that the workspace, lockfile, and runtime roots are consistent.

## Commands

```bash
nodus sync
nodus doctor
```

## CI Variants

```bash
nodus sync --locked
nodus sync --frozen
```

## When To Use Which

- Use `nodus sync` after manifest changes or package updates.
- Use `nodus sync --locked` in CI when `nodus.lock` is expected to already be current.
- Use `nodus sync --frozen` when you must install the exact Git revisions already written in `nodus.lock`.
- Always finish with `nodus doctor` when debugging drift or validating a fresh setup.
