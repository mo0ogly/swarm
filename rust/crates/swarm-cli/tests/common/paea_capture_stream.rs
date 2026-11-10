//! Minimal raw-stream capture primitive for the PAEA approval-web child
//! evidence work: copy a `Read` stream of EXACT bytes into an exclusively
//! new private file — no decoding, no filtering, no sentinel scan, no
//! timeout, no retry, no volume ceiling. The transfer buffer is a technical
//! staging chunk, never a total limit.
//!
//! Deliberately NOT in this lot (separate later tasks, explicitly not
//! started here): the child spawn and concurrent stdout/stderr capture, the
//! real-exit or signal recording, any Drop guard, any parser, and the final
//! `/proc/<pid>/exe` identity or delivered-profile claim.
//!
//! Guards allocate private 0700 fixture leaves EXCLUSIVELY inside the
//! runner's explicit TMPDIR (the private Unix area already resolved by the
//! existing Admin runtime-temporary-directory path). An absent TMPDIR
//! refuses before any fixture exists — there is never a system fallback,
//! never /tmp. The caller (each guard) provides that existing private area
//! and its leaf name; leaves are created by a single exclusive 0700 create
//! that never accepts or chmods a pre-existing leaf, and a collision
//! refuses without retry. Each named guard removes only its own leaf at
//! the end; a panicking guard leaves its leaf behind (documented limit —
//! no Drop guard in this lot). This module is compiled only inside the
//! `paea_approval_web` integration test crate (Linux-only through its
//! crate-level `cfg`), with no new dependency. The exclusive-creation
//! opening and the conservation loop are shared cores
//! (`open_new_capture_file`, `conserve_chunks`) reused verbatim by the
//! live recorded-stream reader (`paea_capture_live`), so the conservation
//! contract cannot diverge between the two forms.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Technical transfer chunk reused for every read/write step. It bounds the
/// staging buffer only, never the captured volume: any stream length is
/// copied chunk by chunk until the reader's EOF.
const TRANSFER_BUFFER_BYTES: usize = 16 * 1024;

/// Stage where a capture failed. `Open` covers the exclusive-creation
/// refusal: the existing file or existing symlink is untouched and nothing
/// was written. Every other stage means the new file EXISTS on disk with
/// its confirmed prefix preserved (possibly empty) — an `Err` never means
/// "no effect", and no stage ever claims an incomplete capture is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamCaptureStage {
    /// Exclusive creation refused or failed; no file created or altered.
    Open,
    /// The live recorded-stream reader's consumer-descriptor preparation
    /// failed AFTER the exclusive creation: the created (EMPTY) 0600
    /// artifact EXISTS on disk and is preserved — never reported as "no
    /// effect" (see `paea_capture_live::LiveRecordedStream::reserve_evidence`).
    ReaderPrepare,
    /// The reader failed; the created file keeps the written prefix.
    Read,
    /// A write failed (a zero-byte write is treated as a failure instead of
    /// an infinite loop); the file keeps the written prefix.
    Write,
    /// Post-EOF flush failed; the file keeps every written byte.
    Flush,
    /// Post-EOF `sync_all` failed; the file keeps every written byte.
    Sync,
}

/// Failure of a raw-stream capture, carrying the real cause and the
/// observable partial state. An `Err` here NEVER claims the capture had no
/// effect, nor that a partial capture is complete.
#[derive(Debug)]
pub struct StreamCaptureError {
    /// Stage that failed (see `StreamCaptureStage` for the on-disk state it
    /// implies).
    pub stage: StreamCaptureStage,
    /// The real I/O cause, preserved verbatim (kind and message).
    pub cause: std::io::Error,
    /// Bytes confirmed written by successful `write` calls before the
    /// failure. After a mid-chunk write failure the file on disk may hold a
    /// few more bytes than this counter (the OS may accept a partial write
    /// yet report an error); the file content is the authoritative partial
    /// state and this counter is the confirmed floor. Always zero on `Open`.
    pub bytes_written: u64,
    /// Error observed while best-effort flushing the written prefix during
    /// failure handling, if that flush itself failed; `None` when the
    /// prefix was flushed cleanly. No `sync_all` is attempted on the
    /// failure path, so the prefix is not fsynced there (documented limit).
    pub flush_error: Option<std::io::Error>,
}

/// Exclusively opens the new capture file at `path` (mode 0600,
/// `create_new`; read+write so the live reader can duplicate this same
/// descriptor for its consumer side — see
/// `paea_capture_live::LiveRecordedStream::reserve_evidence`): never
/// truncates an existing proof and never follows an existing final symlink.
/// The 0600 mode is unchanged; the read access bit is only in the open
/// descriptor's file-status flags, never in the on-disk permission bits.
/// Shared core of the batched primitive and the live recorded-stream reader
/// so the exclusive-creation contract cannot diverge. On refusal the error
/// carries stage `Open`, zero confirmed bytes and no flush error; the
/// existing entry is untouched and no worker or caller side effect exists
/// beyond the refused open.
pub(crate) fn open_new_capture_file(path: &Path) -> Result<std::fs::File, StreamCaptureError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|cause| StreamCaptureError {
            stage: StreamCaptureStage::Open,
            cause,
            bytes_written: 0,
            flush_error: None,
        })
}

/// Conservation core shared by the batched primitive and the live
/// recorded-stream reader (ONE loop, no divergent copies): copies the
/// whole `reader` into `file` chunk by chunk, preserving exact bytes
/// (NUL, non-UTF-8, CRLF), calling `notify` with every successfully
/// written slice in write order, after the write returned success (the
/// batched caller passes a no-op; the live reader notifies its consumer).
/// Success requires EOF, then a successful `flush` and `sync_all` —
/// anything less is an `Err`. On failure the file is kept with its written
/// prefix (never deleted, never completed by invention) and the real error
/// is returned (stage, verbatim cause, confirmed byte floor, best-effort
/// flush secondary). The transfer buffer bounds the staging chunk only,
/// never the captured volume; no timeout, retry or ceiling exists.
pub(crate) fn conserve_chunks<R: Read>(
    reader: &mut R,
    file: &mut std::fs::File,
    notify: &mut dyn FnMut(&[u8]),
) -> Result<u64, StreamCaptureError> {
    let mut buffer = vec![0u8; TRANSFER_BUFFER_BYTES];
    let mut written: u64 = 0;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(cause) => {
                return Err(failed_capture(
                    file,
                    StreamCaptureStage::Read,
                    cause,
                    written,
                ));
            }
        };
        let mut offset = 0;
        while offset < read {
            match file.write(&buffer[offset..read]) {
                Ok(0) => {
                    return Err(failed_capture(
                        file,
                        StreamCaptureStage::Write,
                        std::io::Error::from(std::io::ErrorKind::WriteZero),
                        written,
                    ));
                }
                Ok(n) => {
                    notify(&buffer[offset..offset + n]);
                    offset += n;
                    written += n as u64;
                }
                Err(cause) => {
                    return Err(failed_capture(
                        file,
                        StreamCaptureStage::Write,
                        cause,
                        written,
                    ));
                }
            }
        }
    }
    if let Err(cause) = file.flush() {
        return Err(failed_capture(
            file,
            StreamCaptureStage::Flush,
            cause,
            written,
        ));
    }
    if let Err(cause) = file.sync_all() {
        return Err(failed_capture(
            file,
            StreamCaptureStage::Sync,
            cause,
            written,
        ));
    }
    Ok(written)
}

/// Batched capture of a whole `reader` stream into an exclusively new file
/// at `path` as EXACT raw bytes: NUL, non-UTF-8 sequences and CRLF are
/// preserved; there is no decode, filter or normalization before
/// conservation.
///
/// Inputs and preconditions: `reader` is any `Read` source owned by the
/// caller; `path` is inside a private fixture directory the caller owns.
/// The caller chooses and documents the capture location — this primitive
/// never touches /tmp, a live mission database or a shared path.
///
/// Effects: creates EXCLUSIVELY a new file (`create_new`, mode 0600) via
/// the shared core `open_new_capture_file`, then conserves chunk by chunk
/// until the reader's EOF via the shared core `conserve_chunks` (with a
/// no-op per-write notification here), reused verbatim by the live
/// recorded-stream reader in `paea_capture_live`. Success requires EOF, a
/// successful `flush` and a successful `sync_all` — anything less is an
/// `Err`.
///
/// Output: `Ok(bytes)` with the number of bytes written, equal to the
/// created file's length on success.
///
/// Errors (`StreamCaptureError`): the stage, the real cause, the confirmed
/// byte count and any best-effort-flush error. On a `Read`/`Write`/`Flush`/
/// `Sync` failure the created file is deliberately KEPT with its written
/// prefix (never deleted, never completed by invention); the caller cleans
/// up its own fixture if it wants to. Limits: the durability anchor is
/// `sync_all` on the success path only; drop-close errors after a
/// successful sync are not reported separately. No timeout, retry or
/// volume ceiling exists; the loop is bounded only by the reader's EOF.
pub fn capture_stream_to_new_file<R: Read>(
    reader: &mut R,
    path: &Path,
) -> Result<u64, StreamCaptureError> {
    let mut file = open_new_capture_file(path)?;
    conserve_chunks(reader, &mut file, &mut |_| {})
}

/// Failure-path helper: best-effort flush of the written prefix (its own
/// error, if any, is recorded in `flush_error`, never swallowed), then the
/// error carrying the real cause, the stage and the confirmed count. The
/// file is never removed here.
fn failed_capture(
    file: &mut std::fs::File,
    stage: StreamCaptureStage,
    cause: std::io::Error,
    written: u64,
) -> StreamCaptureError {
    let flush_error = file.flush().err();
    StreamCaptureError {
        stage,
        cause,
        bytes_written: written,
        flush_error,
    }
}

/// Exclusive private 0700 fixture leaf for these guards and the
/// `paea_capture_live` guards, inside the
/// runner's EXPLICIT TMPDIR (the private Unix area resolved by the existing
/// Admin runtime-temporary-directory path; never /tmp, never a system
/// fallback), named uniquely with the test label, process id and wall-clock
/// nanos so a rerun never reuses a previous leaf.
///
/// Contract and limits: the caller (each guard) provides the existing
/// private area through that variable; this helper only READS the
/// environment — it never mutates the environment, the configuration or any
/// policy, creates NO parent directory, and performs no Admin resolution
/// and no live-database read. Refusals happen before any fixture exists: a
/// missing TMPDIR panics with a clear refusal (no system fallback), and a
/// non-directory TMPDIR is refused. The leaf itself is created by a single
/// exclusive `DirBuilder` create with mode 0700: a pre-existing leaf (same
/// label/pid/nanos collision) is never accepted nor chmodded — the call
/// refuses, and no retry, timeout or ceiling exists. The created mode is
/// verified private. Each guard removes only its own leaf at the end; a
/// panicking guard leaves its leaf behind in the runner's private area
/// (documented limit — no Drop guard in this lot), and nothing else under
/// the area is ever removed.
pub(crate) fn private_fixture_leaf(label: &str) -> PathBuf {
    let root = PathBuf::from(std::env::var_os("TMPDIR").unwrap_or_else(|| {
        panic!(
            "explicit TMPDIR is required for the capture-stream guards; \
             no system fallback (never /tmp) is allowed for fixture evidence"
        )
    }));
    assert!(
        root.is_dir(),
        "runner-provided TMPDIR {} must already exist as a private directory",
        root.display()
    );
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock")
        .as_nanos();
    let leaf = root.join(format!(
        "paea-capture-stream-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::DirBuilder::new().mode(0o700).create(&leaf).unwrap_or_else(|error| {
        panic!(
            "exclusive 0700 leaf creation refused for {}: {error} \
             (a collision is a refusal; no retry is attempted)",
            leaf.display()
        )
    });
    let mode = fs::metadata(&leaf)
        .expect("leaf metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "fixture leaf must stay private 0700");
    leaf
}

/// Test-local failing reader: serves `data` verbatim until `fail_after`
/// bytes have been served, then fails with a fixed fixture cause. Its only
/// purpose is a verifiable written prefix before a real reader failure; it
/// simulates nothing else and touches nothing outside its guard.
struct FailingAfterPrefix<'a> {
    data: &'a [u8],
    served: usize,
    fail_after: usize,
}
impl Read for FailingAfterPrefix<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.served >= self.fail_after {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "fixture reader failed after its verifiable prefix",
            ));
        }
        let n = buf.len().min(self.fail_after - self.served);
        buf[..n].copy_from_slice(&self.data[self.served..self.served + n]);
        self.served += n;
        Ok(n)
    }
}

/// First guard: the exact-bytes witness. A payload larger than the transfer
/// chunk (crossing chunk boundaries, not a chunk multiple) and containing
/// NUL, non-UTF-8 bytes and CRLF is copied verbatim into the new file; the
/// returned count, the byte equality and the private 0600 mode must hold.
#[test]
fn capture_stream_writes_exact_bytes_to_a_new_file() {
    let leaf = private_fixture_leaf("exact-bytes");
    let mut payload: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    payload.extend_from_slice(b"tail\r\nwith\0nul\xFFnon-utf8\r\n");
    let target = leaf.join("captured.bin");
    let mut reader: &[u8] = &payload;
    let written = capture_stream_to_new_file(&mut reader, &target).expect("exact capture");
    assert_eq!(written, payload.len() as u64, "confirmed count");
    assert_eq!(
        fs::read(&target).expect("read back"),
        payload,
        "conserved bytes must be exact"
    );
    let mode = fs::metadata(&target)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "captured evidence file must stay private");
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Second guard: exclusive-creation refusals leave the existing entries
/// byte-identical. VALID-CAPTURE WITNESS FIRST, under the same
/// leaf/healthy-reader preconditions as every fault below: the "original"
/// proof is itself produced by the primitive on a healthy reader, so a
/// shared precondition failure cannot be masked by a manually seeded file.
/// An already-present proof file then refuses with stage `Open` and
/// `AlreadyExists`, zero bytes written and no flush error, and its content
/// is unchanged; a dangling symlink at the target path refuses the same way
/// without being followed (the absent target stays absent and the link
/// itself is untouched).
#[test]
fn capture_stream_refuses_existing_file_and_dangling_symlink_without_alteration() {
    let leaf = private_fixture_leaf("refuse-existing");
    let existing = leaf.join("existing.bin");
    let original: Vec<u8> = b"ORIGINAL-PROOF".to_vec();
    let mut healthy: &[u8] = &original;
    let written = capture_stream_to_new_file(&mut healthy, &existing)
        .expect("precondition: a healthy capture must succeed on the same leaf");
    assert_eq!(written, original.len() as u64);
    assert_eq!(fs::read(&existing).expect("read back"), original);
    let mut reader: &[u8] = b"replacement bytes that must never land";
    let error =
        capture_stream_to_new_file(&mut reader, &existing).expect_err("existing file must refuse");
    assert_eq!(error.stage, StreamCaptureStage::Open, "{error:?}");
    assert_eq!(
        error.cause.kind(),
        std::io::ErrorKind::AlreadyExists,
        "{error:?}"
    );
    assert_eq!(error.bytes_written, 0, "{error:?}");
    assert!(error.flush_error.is_none(), "{error:?}");
    assert_eq!(
        fs::read(&existing).expect("read back").as_slice(),
        &b"ORIGINAL-PROOF"[..],
        "existing proof must stay byte-identical"
    );
    let dangling = leaf.join("dangling.bin");
    let absent_target = leaf.join("absent-target.bin");
    std::os::unix::fs::symlink(&absent_target, &dangling).expect("dangling link fixture");
    let mut reader: &[u8] = b"bytes that must not be written through the link";
    let error = capture_stream_to_new_file(&mut reader, &dangling)
        .expect_err("existing symlink must refuse");
    assert_eq!(error.stage, StreamCaptureStage::Open, "{error:?}");
    assert_eq!(
        error.cause.kind(),
        std::io::ErrorKind::AlreadyExists,
        "{error:?}"
    );
    assert!(
        !absent_target.exists(),
        "an existing final symlink must never be followed"
    );
    assert!(
        dangling.symlink_metadata().is_ok(),
        "the refused link must stay untouched"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Third guard: a reader failing after a verifiable prefix. VALID-CAPTURE
/// WITNESS FIRST on the SAME leaf with the SAME payload and a healthy
/// reader (same preconditions as the fault that follows), so an
/// environment-level capture failure cannot be mistaken for the exercised
/// reader fault. The failing-reader error then carries stage `Read`, the
/// real fixture cause preserved verbatim, the confirmed prefix count and no
/// flush error; the created file keeps the exact prefix (neither deleted
/// nor extended). Recovery uses a NEW file with a healthy reader over the
/// same data and succeeds exactly.
#[test]
fn capture_stream_preserves_prefix_and_cause_on_reader_failure_then_recovers_fresh() {
    let leaf = private_fixture_leaf("faulty-reader");
    let payload: Vec<u8> = (0..1000u32).map(|i| (i % 253) as u8).collect();
    let control = leaf.join("control-capture.bin");
    let mut healthy: &[u8] = &payload;
    let control_written = capture_stream_to_new_file(&mut healthy, &control)
        .expect("precondition: a healthy capture of the same payload must succeed first");
    assert_eq!(control_written, payload.len() as u64);
    assert_eq!(
        fs::read(&control).expect("read back"),
        payload,
        "healthy witness on the same leaf must conserve the exact bytes"
    );
    let mut reader = FailingAfterPrefix {
        data: &payload,
        served: 0,
        fail_after: 700,
    };
    let failed = leaf.join("failed-capture.bin");
    let error = capture_stream_to_new_file(&mut reader, &failed).expect_err("must fail");
    assert_eq!(error.stage, StreamCaptureStage::Read, "{error:?}");
    assert_eq!(
        error.cause.kind(),
        std::io::ErrorKind::BrokenPipe,
        "{error:?}"
    );
    assert!(
        error.cause.to_string().contains("verifiable prefix"),
        "real cause must be preserved verbatim: {error:?}"
    );
    assert_eq!(error.bytes_written, 700, "confirmed prefix count: {error:?}");
    assert!(
        error.flush_error.is_none(),
        "best-effort prefix flush must not invent a second failure: {error:?}"
    );
    assert_eq!(
        fs::read(&failed).expect("read back"),
        &payload[..700],
        "written prefix must be preserved verbatim"
    );
    let recovered = leaf.join("recovered-capture.bin");
    let mut healthy: &[u8] = &payload;
    let written = capture_stream_to_new_file(&mut healthy, &recovered).expect("fresh recovery");
    assert_eq!(written, payload.len() as u64);
    assert_eq!(
        fs::read(&recovered).expect("read back"),
        payload,
        "fresh recovery must conserve the exact bytes"
    );
    assert_eq!(
        fs::read(&failed).expect("read back"),
        &payload[..700],
        "the failed capture stays as-is: neither deleted nor completed"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}
