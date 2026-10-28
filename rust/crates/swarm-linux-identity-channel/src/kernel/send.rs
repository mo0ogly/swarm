//! Sole native packet send, no admission, deadline, retry or global SIGPIPE change.
use crate::{Error, SendFailure};
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};
/// Validate the borrowed configured channel before one send. Preserve every
/// nonnegative accepted count on unexpected short output; never split/retry a
/// record. Negative output retains the immediate real errno and unknown effect.
pub(crate) fn packet(fd: BorrowedFd<'_>, bytes: &[u8]) -> Result<(), SendFailure> {
    super::ready(fd).map_err(|cause| SendFailure {
        cause,
        attempted: false,
        accepted_len: None,
    })?;
    if bytes.is_empty() {
        return Err(SendFailure {
            cause: Error::refused("identity-channel-send-empty"),
            attempted: false,
            accepted_len: None,
        });
    }
    // SAFETY: valid borrowed socket and immutable initialized slice of exact len.
    // Native synchronous call retains neither pointer nor Rust reference. NOSIGNAL
    // is a per-call Linux ABI flag; it never changes another thread's signal policy.
    let result = unsafe {
        libc::send(
            fd.as_raw_fd(),
            bytes.as_ptr().cast(),
            bytes.len(),
            libc::MSG_NOSIGNAL,
        )
    };
    if result < 0 {
        return Err(SendFailure {
            cause: Error::syscall("identity-channel-send", io::Error::last_os_error()),
            attempted: true,
            accepted_len: None,
        });
    }
    let accepted = result as usize;
    if accepted != bytes.len() {
        return Err(SendFailure {
            cause: Error::refused("identity-channel-send-short"),
            attempted: true,
            accepted_len: Some(accepted),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
