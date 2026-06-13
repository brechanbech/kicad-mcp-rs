//! Entry point for the KiCad MCP server.
//!
//! Initializes structured logging to stderr (stdout is reserved for the MCP stdio
//! transport), builds the server, and serves until the client disconnects or a
//! shutdown signal arrives.

mod server;
mod tools;

use anyhow::Context as _;
use rmcp::{ServiceExt as _, transport::stdio};
use tracing_subscriber::EnvFilter;

use crate::server::KicadMcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    tracing::info!(target: "kicad_mcp.startup", version = env!("CARGO_PKG_VERSION"));

    // stdio transport: JSON-RPC frames over stdin/stdout, so all logs MUST go to stderr.
    let service = KicadMcp::new()
        .serve(stdio())
        .await
        .context("failed to start MCP service on stdio")?;

    service
        .waiting()
        .await
        .context("MCP service ended with error")?;
    Ok(())
}

/// Configures `tracing` to write structured logs to stderr, honoring `RUST_LOG`.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("kicad_mcp_server=info,kicad_access=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
