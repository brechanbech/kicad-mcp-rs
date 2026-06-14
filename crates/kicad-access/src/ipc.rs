//! The live IPC backend: act on a board the user has open.
//!
//! Wraps `kicad-ipc-rs` (async-first client for KiCad's protobuf-over-NNG IPC API).
//! Unlike [`crate::files`], this needs the **running** KiCad 10 GUI with the API server
//! enabled (Preferences > Plugins > IPC API). KiCad 10 has no headless API server, so
//! this is the one backend that is not GUI-free.
//!
//! The socket path is read from `KICAD_API_SOCKET` (set by KiCad when it launches a
//! plugin) and falls back to the platform default. The IPC API is request/reply
//! only — there are no async notifications from KiCad — so this is a plain
//! call/response client.

use kicad_ipc_rs::KiCadClient;

use crate::{Error, ErrorKind, Result};

/// A connection to a running KiCad instance.
///
/// Wraps a [`KiCadClient`], which is itself `Arc`-backed, so cloning shares the one
/// underlying connection (M-SERVICES-CLONE).
#[derive(Debug, Clone)]
pub struct Kicad {
    inner: KiCadClient,
}

impl Kicad {
    /// Connects to the KiCad instance named by `KICAD_API_SOCKET` (or the default).
    ///
    /// Requires a running KiCad with the API server enabled and a board/project open.
    ///
    /// # Errors
    /// Returns [`ErrorKind::NotConnected`] if no instance is reachable.
    pub async fn connect() -> Result<Self> {
        tracing::debug!(target: "kicad.ipc.connect.start", "connecting to KiCad IPC socket");

        let inner = KiCadClient::connect()
            .await
            .map_err(|e| Error::with_source(ErrorKind::NotConnected, e))?;

        tracing::debug!(target: "kicad.ipc.connect.ok", socket = inner.socket_uri());
        Ok(Self { inner })
    }

    /// Returns the connected KiCad version string (smoke test for the link).
    ///
    /// # Errors
    /// Returns [`ErrorKind::NotConnected`] if the request fails (dropped link, timeout).
    pub async fn version(&self) -> Result<String> {
        let info = self
            .inner
            .get_version()
            .await
            .map_err(|e| Error::with_source(ErrorKind::NotConnected, e))?;
        Ok(info.full_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live smoke test against a running KiCad 10 GUI (API server enabled, a board
    /// open). Ignored by default — there's no server in CI. Run it manually with:
    /// `cargo test -p kicad-access --features ipc -- --ignored live_version`
    #[tokio::test]
    #[ignore = "requires a running KiCad 10 GUI with the IPC API server enabled"]
    async fn live_version() {
        let kicad = Kicad::connect().await.expect("connect to KiCad IPC");
        let version = kicad.version().await.expect("get version");
        assert!(
            version.starts_with("10."),
            "unexpected KiCad version: {version}"
        );
    }
}
