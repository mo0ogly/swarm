//! Command JSON framing, inherited from Go readInput (16 MiB), not execution policy.
//! Named inputs must be regular files; stdin is an explicitly selected stream.
use crate::SettingCount;
use serde::Serialize;
use std::{error::Error, fmt, io::Read};
/// Existing Go command transport bound. Not an agent budget or validation timeout.
pub const COMMAND_INPUT_MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandInputTransport {
    pub max_bytes: SettingCount,
    pub named_files_must_be_regular: bool,
    pub stdin_streams_supported: bool,
}
impl CommandInputTransport {
    pub fn current() -> Self {
        Self {
            max_bytes: SettingCount(COMMAND_INPUT_MAX_BYTES as i64),
            named_files_must_be_regular: true,
            stdin_streams_supported: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandInputOperation {
    Open,
    Inspect,
    Read,
    Close,
    Frame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandInputFailure {
    Io,
    NotRegular,
    TooLarge,
}
pub struct CommandInputError {
    pub operation: CommandInputOperation,
    pub failure: CommandInputFailure,
    cause: Option<Box<dyn Error + Send + Sync>>,
    pub cleanup: Option<Box<dyn Error + Send + Sync>>,
}
impl CommandInputError {
    fn io(operation: CommandInputOperation, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            failure: CommandInputFailure::Io,
            cause: Some(Box::new(cause)),
            cleanup: None,
        }
    }
    fn refused(failure: CommandInputFailure) -> Self {
        Self {
            operation: CommandInputOperation::Frame,
            failure,
            cause: None,
            cleanup: None,
        }
    }
    pub fn code(&self) -> &'static str {
        match self.failure {
            CommandInputFailure::Io => "input-document-io",
            CommandInputFailure::NotRegular => "input-regular-file-required",
            CommandInputFailure::TooLarge => "input-document-too-large",
        }
    }
}
impl fmt::Display for CommandInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for CommandInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for CommandInputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
pub fn check_command_input_size(raw: &[u8]) -> Result<(), CommandInputError> {
    if raw.len() > COMMAND_INPUT_MAX_BYTES {
        Err(CommandInputError::refused(CommandInputFailure::TooLarge))
    } else {
        Ok(())
    }
}
/// Borrowed stream ownership stays with the caller; no implicit close or deadline.
pub fn read_command_document(reader: &mut impl Read) -> Result<Vec<u8>, CommandInputError> {
    let mut raw = Vec::new();
    reader
        .take((COMMAND_INPUT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|e| CommandInputError::io(CommandInputOperation::Read, e))?;
    check_command_input_size(&raw)?;
    Ok(raw)
}
#[cfg(unix)]
pub fn read_command_file(path: &std::path::Path) -> Result<Vec<u8>, CommandInputError> {
    use rustix::fs::{open, Mode, OFlags};
    // Symlinks to regular request documents retain Go behavior. This reader
    // confers no identity-store ownership or confinement guarantee.
    let mut file: std::fs::File = open(
        path,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| CommandInputError::io(CommandInputOperation::Open, e))?
    .into();
    let result = (|| {
        let metadata = file
            .metadata()
            .map_err(|e| CommandInputError::io(CommandInputOperation::Inspect, e))?;
        if !metadata.is_file() {
            return Err(CommandInputError::refused(CommandInputFailure::NotRegular));
        }
        if metadata.len() > COMMAND_INPUT_MAX_BYTES as u64 {
            return Err(CommandInputError::refused(CommandInputFailure::TooLarge));
        }
        read_command_document(&mut file)
    })();
    finish_named_input(result, nix::unistd::close(file))
}
#[cfg(unix)]
fn finish_named_input(
    result: Result<Vec<u8>, CommandInputError>,
    closed: Result<(), nix::errno::Errno>,
) -> Result<Vec<u8>, CommandInputError> {
    match (result, closed) {
        (Ok(raw), Ok(())) => Ok(raw),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CommandInputError::io(CommandInputOperation::Close, e)),
        (Err(mut e), Err(cleanup)) => {
            e.cleanup = Some(Box::new(cleanup));
            Err(e)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn primary_input_error_and_close_cleanup_remain_separate_and_recover() {
        assert_eq!(
            finish_named_input(Ok(b"control".to_vec()), Ok(())).unwrap(),
            b"control"
        );
        let primary = CommandInputError::io(
            CommandInputOperation::Read,
            std::io::Error::other("PRIVATE cause"),
        );
        let error = finish_named_input(Err(primary), Err(nix::errno::Errno::EBADF)).unwrap_err();
        assert_eq!(error.operation, CommandInputOperation::Read);
        assert_eq!(error.source().unwrap().to_string(), "PRIVATE cause");
        assert!(error
            .cleanup
            .as_ref()
            .unwrap()
            .downcast_ref::<nix::errno::Errno>()
            .is_some());
        assert!(!format!("{error:?}").contains("PRIVATE"));
        let error = finish_named_input(
            Ok(b"read succeeded".to_vec()),
            Err(nix::errno::Errno::EBADF),
        )
        .unwrap_err();
        assert_eq!(error.operation, CommandInputOperation::Close);
        assert_eq!(
            finish_named_input(Ok(b"recovery".to_vec()), Ok(())).unwrap(),
            b"recovery"
        );
    }
}
