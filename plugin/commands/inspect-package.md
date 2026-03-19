# Inspect A Nodus Package

Inspect a local package, a configured dependency alias, or a remote package reference before changing workspace state.

## Commands

```bash
nodus info .
nodus info <dependency-alias>
nodus info owner/repo
```

## What To Check

- resolved source and pinned ref
- discovered skills, agents, rules, and commands
- selected components
- nested dependencies
- declared capabilities
- manifest warnings

## Guidance

- Use `nodus info .` when authoring a package repo.
- Use `nodus info <alias>` when debugging an installed dependency in a consuming workspace.
