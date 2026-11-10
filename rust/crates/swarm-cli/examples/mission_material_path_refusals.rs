//! Two fresh owned material-path cases against the ordinary engine library.
//!
//! No caller Root or observation input is accepted. The cases initialize their
//! own inactive UUID-named fixtures through public APIs, restore both paths before
//! checking refusal, and preserve failed fixture bytes. No SQL, provider, mission,
//! Go runtime, authority forgery or alternate review wrapper is used. This is an
//! SDK qualification example, not a production mission operation or proof that
//! every material-path consumer is protected. Dependencies/profile/binary identity
//! and the engine's non-cfg(test) compilation must be qualified by the runner.
pub use swarm_engine::*;

#[cfg(target_os = "linux")]
#[path = "../../swarm-engine/src/root_store/material_path_tests.rs"]
mod cases;

/// Flush only a static successful-case marker. IO failures remain real failures;
/// an absent marker must not be treated as a successful case or no-effect proof.
#[cfg(target_os = "linux")]
fn marker(value: &'static str) -> std::io::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}")?;
    out.flush()
}

/// Run the writer first so a VFS-protected writer is recorded even if the reader
/// reproduces its independent defect. Each function supplies its own valid control
/// and fresh owned fixtures; no external Root path or command arguments are used.
#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    if std::env::args_os().count() != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "material-path example accepts no arguments",
        ));
    }
    cases::writer_case();
    marker("CASE writer PASS")?;
    cases::reader_case();
    marker("CASE reader PASS")
}

/// The retained fixture exercises Linux Root/process/storage contracts only.
/// Unsupported platforms fail explicitly without creating any fixture.
#[cfg(not(target_os = "linux"))]
fn main() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "material-path example requires Linux",
    ))
}
