//! The `kicad-cli` backend: production outputs and rule checks.
//!
//! The IPC API does not do plotting, exporting, DRC, or ERC; the upstream guidance
//! is to use `kicad-cli` for those. This module is a thin async wrapper over the
//! binary — no KiCad crate dependency, just the `kicad-cli` executable on `PATH`
//! (or pointed at explicitly).
//!
//! Everything here is sans-board: you pass a path to a `.kicad_pcb` / `.kicad_sch`
//! and get an artifact or a report back. That makes it usable headlessly.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::{Error, ErrorKind, Result};

/// A configured handle to the `kicad-cli` executable.
///
/// Cheap to clone; holds only the resolved binary path.
#[derive(Debug, Clone)]
pub struct KicadCli {
    program: PathBuf,
}

impl Default for KicadCli {
    fn default() -> Self {
        // Resolved from PATH at spawn time.
        Self {
            program: PathBuf::from("kicad-cli"),
        }
    }
}

impl KicadCli {
    /// Creates a handle resolving `kicad-cli` from `PATH`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a handle to a `kicad-cli` at an explicit path.
    ///
    /// Use this on systems where the binary isn't on `PATH` (common on FreeBSD/macOS
    /// where it lives under the KiCad app bundle or `/usr/local/bin`).
    pub fn at(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// Exports Gerber files for `pcb` into `out_dir`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::Cli`] if the binary can't be spawned or exits non-zero.
    pub async fn export_gerbers(&self, pcb: &Path, out_dir: &Path) -> Result<()> {
        self.run([
            OsStr::new("pcb"),
            OsStr::new("export"),
            OsStr::new("gerbers"),
            OsStr::new("--output"),
            out_dir.as_os_str(),
            pcb.as_os_str(),
        ])
        .await
        .map(|_| ())
    }

    /// Runs DRC on `pcb` and returns the raw JSON report `kicad-cli` produces.
    ///
    /// # Errors
    /// Returns [`ErrorKind::Cli`] if the binary can't be spawned or DRC fails to run
    /// (a malformed board, say). Note this does *not* fail just because the board has
    /// violations: we deliberately omit `--exit-code-violations` so a board with DRC
    /// errors still yields its report, which the caller parses to decide pass/fail.
    pub async fn run_drc(&self, pcb: &Path) -> Result<String> {
        // `--format json` keeps the output machine-parseable for kicad-checks.
        let out = self
            .run([
                OsStr::new("pcb"),
                OsStr::new("drc"),
                OsStr::new("--format"),
                OsStr::new("json"),
                pcb.as_os_str(),
            ])
            .await?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Spawns `kicad-cli` with `args`, capturing output.
    async fn run<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = tokio::process::Command::new(&self.program)
            .args(args)
            .output()
            .await
            .map_err(|e| Error::with_source(ErrorKind::Cli, e))?;

        if output.status.success() {
            tracing::debug!(target: "kicad.cli.exec.ok", code = ?output.status.code());
            Ok(output)
        } else {
            tracing::warn!(
                target: "kicad.cli.exec.fail",
                code = ?output.status.code(),
                stderr = %String::from_utf8_lossy(&output.stderr),
            );
            Err(Error::new(ErrorKind::Cli))
        }
    }
}
