//! The live IPC backend: act on a board the user has open.
//!
//! Wraps `kicad-ipc-rs` (async-first client for KiCad's protobuf-over-NNG IPC API).
//! Unlike [`crate::files`], this needs a **running** KiCad — either the GUI with the
//! API server enabled (Preferences > Plugins) or a headless `kicad-cli api-server`.
//!
//! The socket path is read from `KICAD_API_SOCKET` (set by KiCad when it launches a
//! plugin) and falls back to the platform default. The IPC API is request/reply
//! only — there are no async notifications from KiCad — so this is a plain
//! call/response client.

use crate::{Error, ErrorKind, Result};

/// A connection to a running KiCad instance.
///
/// Heavyweight; clone shares the underlying connection (M-SERVICES-CLONE) once the
/// real client is wired in.
#[derive(Debug, Clone)]
pub struct Kicad {
    // TODO(impl): hold the kicad-ipc-rs client handle, e.g. `inner: Arc<kicad_ipc_rs::Client>`.
    _private: (),
}

impl Kicad {
    /// Connects to the KiCad instance named by `KICAD_API_SOCKET` (or the default).
    ///
    /// # Errors
    /// Returns [`ErrorKind::NotConnected`] if no instance is reachable.
    // async by contract: the real body awaits the kicad-ipc-rs connect (see TODO).
    #[allow(clippy::unused_async)]
    pub async fn connect() -> Result<Self> {
        tracing::debug!(target: "kicad.ipc.connect.start", "connecting to KiCad IPC socket");

        // TODO(impl): real connection, e.g.
        //   let client = kicad_ipc_rs::Client::connect_default().await
        //       .map_err(|e| Error::with_source(ErrorKind::NotConnected, e))?;
        //   Ok(Self { inner: Arc::new(client) })
        Err(Error::new(ErrorKind::NotConnected))
    }

    /// Returns the connected KiCad version string (smoke test for the link).
    ///
    /// # Errors
    /// Returns [`ErrorKind::NotConnected`] if the call fails.
    // async by contract: the real body awaits a kicad-ipc-rs request/reply.
    #[allow(clippy::unused_async)]
    pub async fn version(&self) -> Result<String> {
        let _ = self;
        Err(Error::with_source(
            ErrorKind::Unsupported,
            "kicad-ipc-rs call not yet wired",
        ))
    }
}
