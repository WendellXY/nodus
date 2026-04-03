mod handlers;
mod server;
mod tools;

pub use tools::*;

use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value as JsonValue;

pub fn tool_definitions() -> Vec<(&'static str, &'static str, JsonValue)> {
    vec![
        (
            TOOL_ADD,
            "Add a dependency to the project",
            add_input_schema(),
        ),
        (
            TOOL_REMOVE,
            "Remove a dependency from the project",
            remove_input_schema(),
        ),
        (TOOL_SYNC, "Sync all dependencies", sync_input_schema()),
        (TOOL_LIST, "List installed packages", list_input_schema()),
        (
            TOOL_RELAY,
            "Relay managed edits to linked source repos",
            relay_input_schema(),
        ),
        (
            TOOL_RELAY_STATUS,
            "Show pending relay edits and conflicts",
            relay_status_input_schema(),
        ),
        (
            TOOL_INFO,
            "Show project or package info",
            info_input_schema(),
        ),
        (
            TOOL_CHECK_UPDATES,
            "Check for available package updates",
            check_updates_input_schema(),
        ),
    ]
}

pub async fn start_server(cwd: PathBuf, cache_root: PathBuf) -> Result<()> {
    server::run(cwd, cache_root).await
}
