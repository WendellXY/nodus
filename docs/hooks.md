# Hooks

Nodus lets a package declare portable hook intents in `nodus.toml`, then emits
adapter-specific wiring (Claude `settings.json`, Codex `config.toml`, OpenCode
`plugins/nodus-hooks.js`) during `nodus sync`. Hooks that an adapter cannot
express are silently filtered out — the manifest stays portable and the
generated output stays valid.

This page is the source of truth for what each adapter supports today.

## Event catalog

These are the seven events the nodus manifest recognizes. The value on the
left is what you write in `event = "..."`.

| `event`                 | Purpose                                                         |
|-------------------------|-----------------------------------------------------------------|
| `session_start`         | A new agent session is beginning (startup, resume, etc.)        |
| `user_prompt_submit`    | The user submitted a prompt                                     |
| `pre_tool_use`          | Fired before a tool call                                        |
| `permission_request`    | Fired when the agent asks the user for permission               |
| `post_tool_use`         | Fired after a tool call                                         |
| `stop`                  | The agent turn ended                                            |
| `session_end`           | The session is closing                                          |

## Adapter support matrix

A hook only reaches an adapter's generated config if the adapter supports
that event. Consumers never have to strip these manually.

| Adapter    | Supported events                                                                                      | `session_start` sources             |
|------------|-------------------------------------------------------------------------------------------------------|-------------------------------------|
| `claude`   | `session_start`, `user_prompt_submit`, `pre_tool_use`, `post_tool_use`, `stop`, `session_end`         | `startup`, `resume`, `clear`, `compact` |
| `codex`    | `session_start`, `user_prompt_submit`, `pre_tool_use`, `permission_request`, `post_tool_use`, `stop`  | `startup`, `resume`                 |
| `opencode` | `session_start`, `pre_tool_use`, `post_tool_use`, `stop`                                              | `startup`                           |
| `agents`   | none                                                                                                  | —                                   |
| `copilot`  | none                                                                                                  | —                                   |
| `cursor`   | none                                                                                                  | —                                   |

Notes:
- `permission_request` is Codex-only. Claude does not expose a comparable
  event; declaring a hook that targets only Claude with this event fails
  manifest validation.
- `user_prompt_submit` and `session_end` have no native equivalent in
  OpenCode's plugin API today and are dropped for that adapter.
- OpenCode currently only wires four events of its native plugin surface
  (`session.created`, `session.idle`, `tool.execute.before`,
  `tool.execute.after`). Other OpenCode events (`permission.*`, `file.*`,
  `message.*`, `todo.*`, etc.) are not routed by nodus — if you need them,
  ship a Claude-or-OpenCode-specific plugin instead of declaring `[[hooks]]`.

## Matcher semantics

`matcher` is optional. Which fields are allowed depends on the event:

| Event                            | `matcher.sources`    | `matcher.tool_names` |
|----------------------------------|----------------------|----------------------|
| `session_start`                  | allowed              | rejected             |
| `pre_tool_use`                   | rejected             | allowed              |
| `permission_request`             | rejected             | allowed              |
| `post_tool_use`                  | rejected             | allowed              |
| `user_prompt_submit`             | rejected             | rejected             |
| `stop`                           | rejected             | rejected             |
| `session_end`                    | rejected             | rejected             |

Values:
- `sources`: any of `startup`, `resume`, `clear`, `compact`. Nodus drops
  sources the target adapter doesn't support; if none remain after filtering,
  the hook is skipped for that adapter.
- `tool_names`: currently only `bash` is recognized.

Duplicates inside `sources` or `tool_names` are rejected by the validator.

## Handler

Every hook has a `handler`. Today only command-style handlers exist:

```toml
[hooks.handler]
type    = "command"
command = "nodus sync"      # shell string, required
cwd     = "git_root"        # optional: "git_root" (default) or "session"
```

`cwd` controls where the script runs. `git_root` resolves to
`git rev-parse --show-toplevel`, with a fallback to the process's current
directory if the repo isn't a git worktree. `session` keeps the working
directory the adapter chose.

Top-level fields on `[[hooks]]`:

| Field         | Type             | Default | Meaning                                                                                 |
|---------------|------------------|---------|-----------------------------------------------------------------------------------------|
| `id`          | string           | —       | Required, globally unique within the resolved package graph.                            |
| `event`       | string           | —       | See [Event catalog](#event-catalog).                                                    |
| `adapters`    | array of strings | `[]`    | Restricts which adapters may emit this hook. Empty = any supported adapter.             |
| `matcher`     | table            | —       | See [Matcher semantics](#matcher-semantics).                                            |
| `handler`     | table            | —       | Required.                                                                               |
| `timeout_sec` | integer          | —       | Exposed to the script as `NODUS_HOOK_TIMEOUT_SEC`. Does not enforce a kill timer.       |
| `blocking`    | bool             | `false` | If `true`, the adapter should fail the event when the script fails.                     |

## Runtime environment

Nodus-generated hook scripts export these env vars before running the user
command:

- `NODUS_HOOK_ID` — the `id` from the manifest
- `NODUS_HOOK_EVENT` — the snake_case event name
- `NODUS_HOOK_TIMEOUT_SEC` — only set if `timeout_sec` is declared

Everything else (the event payload, tool inputs/outputs) is delivered via
stdin by the adapter, in the shape that adapter already uses.

## Deduplication

If both the root manifest and a dependency declare a hook with the same `id`,
nodus keeps the root's declaration and drops the dependency's. This lets a
package ship hooks that consumers can override without forking.

## `claude_plugin_hooks` (Claude escape hatch)

For Claude packages shipping a pre-built plugin `hooks/hooks.json` that uses
`CLAUDE_PLUGIN_ROOT` semantics, declare it at the manifest top level instead
of translating it to native `[[hooks]]`:

```toml
claude_plugin_hooks = ["hooks/hooks.json"]
```

The contents are passed through verbatim under the Claude-specific plugin
root. They only affect the Claude adapter and are not portable across Codex or
OpenCode.

## Minimal example

```toml
[[hooks]]
id    = "mypkg.sync_on_startup"
event = "session_start"

[hooks.matcher]
sources = ["startup", "resume"]

[hooks.handler]
type    = "command"
command = "nodus sync"
```

This fires on Claude (both `startup` and `resume`), on Codex (both sources),
and on OpenCode (`startup` only — `resume` is filtered). It is dropped for
`agents`, `copilot`, and `cursor`.

## Pre-tool example

```toml
[[hooks]]
id    = "mypkg.audit_bash"
event = "pre_tool_use"

[hooks.matcher]
tool_names = ["bash"]

[hooks.handler]
type    = "command"
command = "mypkg audit-bash"
```

Emitted for Claude, Codex, and OpenCode. The adapters filter further by tool
name at runtime.

## Inspecting what a package will emit

Every `nodus info` payload includes a `hook-adapter-support` section computed
from the rules on this page:

```bash
nodus info <package> --json | jq .hook_adapter_support
```

Use this when adding a hook to verify it reaches the adapters you expect
before running `nodus sync`.
