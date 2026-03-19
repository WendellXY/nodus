# Add Package With Nodus

Add a Nodus dependency to the current workspace without manually copying runtime files.

## Steps

1. Identify the package reference:
   - GitHub shortcut like `owner/repo`
   - full Git URL
   - local path
2. Choose the adapter root the workspace should manage.
3. Narrow components only if the workspace needs a subset.
4. Run `nodus add`.
5. Run `nodus doctor`.

## Examples

```bash
nodus add obra/superpowers --adapter codex
nodus add obra/superpowers --adapter claude --component skills --component rules
nodus add ../local-package --adapter opencode
nodus doctor
```

## Notes

- Prefer tags for released packages.
- Use `--dev` for workspace-local tooling packages that should not be re-exported.
- Let Nodus write `nodus.toml`, `nodus.lock`, and managed outputs.
