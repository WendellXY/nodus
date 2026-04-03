# Notify + BLAKE3 Relay Watch Design

## Summary

Replace the polling-based relay watch loop with an event-driven architecture using `notify` for filesystem events and `BLAKE3` for content fingerprinting. Migrate the entire codebase from `sha2` (SHA-256) to `blake3`.

## Motivation

The current relay watch uses a 1-second `thread::sleep` polling loop with SHA-256 fingerprints. This has three costs:
- Up to 1s latency before changes are detected
- Unnecessary CPU/IO every second when files are idle
- SHA-256 is slower than BLAKE3 for hashing file contents

## Architecture: Hybrid Notify + BLAKE3

`notify` watches directories for wakeup signals. BLAKE3 fingerprints remain the source of truth for whether content actually changed. A fallback periodic sweep (30s) provides robustness if notify misses an event.

```
loop {
    select! {
        _ = notify_rx.recv() => {
            // Coalesce rapid writes with 100ms debounce
            tokio::time::sleep(100ms).await;
            drain_pending_events(&mut notify_rx);
        }
        _ = tokio::time::sleep(30s) => {}  // fallback sweep
        _ = ctrl_c() => { return summaries; }
    }
    let next_state = capture_watch_state();  // BLAKE3 fingerprints
    if next_state != state { relay_changed_packages(); }
}
```

## BLAKE3 Migration

### Computational hashing (direct swap)

Replace `sha2::{Digest, Sha256}` with `blake3::Hasher` in:
- `src/git.rs` — commit hashing
- `src/relay/runtime.rs` — `sha256_hex` becomes `blake3_hex`
- `src/resolver/runtime/resolve.rs` — package digest computation
- `src/relay/runtime/watch.rs` — file fingerprints

### Persisted formats (backwards-compatible)

- **Digest prefix**: New writes use `"blake3:..."`. Read-side accepts both `"sha256:..."` and `"blake3:..."` prefixes. On next `nodus sync`, digests are recomputed as `blake3:`.
- **Store directory**: Rename `store/sha256` to `store/blake3`. Old cache can be wiped (it's a cache, not user data).
- **Config field**: Rename `source_sha256` to `source_hash` in `local_config.rs`. Use `#[serde(alias = "source_sha256")]` for backwards compatibility.
- **Prefix mismatch**: When a lockfile contains `sha256:` digests and computation produces `blake3:`, treat as "needs recompute" rather than integrity violation.

### Dependency changes

- Add `blake3` to `[dependencies]`
- Add `notify = "8"` to the platform-gated `[target.'cfg(...)'.dependencies]`
- Remove `sha2` once fully migrated

## Async Watch Design

### Public API change

`watch_dependency_in_dir` and `watch_dependencies_in_dir` become `async fn` returning `Result<Vec<RelaySummary>>`. The CLI handler wraps the call in the existing tokio runtime.

### Watcher setup

- Watch each package's managed output root with `notify::RecursiveMode::Recursive`
- Watch config files (manifest, lockfile, local config, adapter markers) with `NonRecursive`
- Use `notify::RecommendedWatcher` for platform-appropriate backend (FSEvents, inotify, ReadDirectoryChanges)

### RelayWatchOptions

```rust
struct RelayWatchOptions {
    debounce: Duration,          // coalesce rapid writes (default 100ms)
    fallback_interval: Duration, // periodic sweep (default 30s)
    max_events: Option<usize>,   // controls test exit
    timeout: Option<Duration>,   // replaces max_polls for tests
}
```

## Error Handling

- **Watcher init failure** (e.g., inotify limit exhausted): Fall back to pure polling at `fallback_interval` with a warning via `reporter.warn()`. BLAKE3 fingerprint loop works standalone.
- **Watcher error events**: Log and continue. Fallback sweep catches anything missed.
- **Graceful shutdown**: `tokio::signal::ctrl_c()` in the `select!` for clean watcher teardown. Returns accumulated summaries.

## Test Strategy

### Watch tests

- Existing watch tests become `#[tokio::test]`
- Replace `poll_interval: Duration::from_millis(20)` + `max_polls: Some(200)` with `debounce: Duration::from_millis(10)` + `timeout: Some(Duration::from_secs(5))` + `max_events: Some(2)`
- Readiness spin loop uses `tokio::time::sleep` instead of `thread::sleep`
- File writes to tempdirs trigger real `notify` events (no mocking)

### BLAKE3 migration tests

- Update existing hash assertions from `sha256_hex` to `blake3_hex`
- Add test: deserializing a lockfile with `"sha256:..."` digests still parses
- Add test: `source_sha256` field alias in local_config deserialization works

### Unaffected tests

All non-watch relay tests (conflict detection, batch relay, etc.) are unchanged — they don't use the watch loop.

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Add `blake3`, `notify = "8"`; remove `sha2` |
| `src/git.rs` | `sha2` -> `blake3` |
| `src/relay/runtime.rs` | `sha256_hex` -> `blake3_hex`, watch fns become async |
| `src/relay/runtime/watch.rs` | Async notify loop, BLAKE3 fingerprints |
| `src/resolver/runtime/resolve.rs` | `sha2` -> `blake3`, digest prefix `blake3:` |
| `src/lockfile.rs` | Accept both `sha256:` and `blake3:` prefixes |
| `src/adapters.rs` | Accept both `sha256:` and `blake3:` prefixes |
| `src/store.rs` | `store/sha256` -> `store/blake3` |
| `src/local_config.rs` | `source_sha256` -> `source_hash` with alias |
| `src/cli/handlers/project.rs` | `.await` on watch calls |
| `src/cli/tests.rs` | Update store path assertions |
