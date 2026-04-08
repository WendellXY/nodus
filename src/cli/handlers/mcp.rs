use anyhow::Context;

use crate::cli::handlers::CommandContext;
use crate::cli::output::write_json;
use crate::mcp::McpOverallStatus;

pub(crate) fn handle_mcp_serve(context: &CommandContext<'_>) -> anyhow::Result<()> {
    let rt =
        tokio::runtime::Runtime::new().context("failed to create async runtime for MCP server")?;
    let cwd = context.cwd.to_path_buf();
    let cache_root = context.cache_root.to_path_buf();
    rt.block_on(crate::mcp::start_server(cwd, cache_root))
}

pub(crate) fn handle_mcp_status(context: &CommandContext<'_>, json: bool) -> anyhow::Result<()> {
    let report = crate::mcp::inspect_status_in_dir(context.cwd)?;
    if json {
        return write_json(context.reporter, &report);
    }

    crate::mcp::render_status(&report, context.reporter)?;
    let message = match report.summary.overall_status {
        McpOverallStatus::Healthy => format!(
            "checked MCP wiring; {} managed config{} ready",
            report.summary.configured_count,
            if report.summary.configured_count == 1 {
                ""
            } else {
                "s"
            }
        ),
        McpOverallStatus::NotConfigured => {
            "checked MCP wiring; no managed MCP config found".to_string()
        }
        McpOverallStatus::Broken => format!(
            "checked MCP wiring; found {} issue{}",
            report.summary.issue_count,
            if report.summary.issue_count == 1 {
                ""
            } else {
                "s"
            }
        ),
    };
    context.reporter.finish(message)
}
