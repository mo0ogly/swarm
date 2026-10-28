//! Startup current-source check only. No RootStore, configuration change,
//! human session, listener or execution-authority fallback is available here.
use super::{json_bytes, write_output, Arguments, CliError};
use std::io::Write;
/// Consume parser-validated closed identity options, check command shape before
/// any input/storage access, verify the
/// pinned declaration and current enrollment, then serialize after explicit close.
/// Output/flush failures remain nonzero with their original typed I/O cause.
/// No project path/options, stdin, raw control command or service activation.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.input.is_empty()
        || a.input == "-"
        || a.server
        || a.root != "."
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let observed = swarm_enrollment_authority::startup::check(
        &swarm_enrollment_state::Context::new(),
        &a.input,
        &a.pos[2],
    )
    .map_err(|cause| CliError::ControlCheck {
        cause: Box::new(cause),
        english: a.language == "en",
    })?;
    let bytes = json_bytes(&observed).map_err(CliError::Output)?;
    write_output(out, &bytes).map(|()| 0)
}

/// Traverse enrollment failures without rendering their private text. Preserve
/// primary sources plus independent close/rollback/refusal branches, including
/// inside a custom IO error handled by the CLI's existing wrapper matrix.
pub(super) fn error_links<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> (Vec<String>, Vec<&'a (dyn std::error::Error + 'static)>) {
    use std::error::Error;
    let mut labels = Vec::new();
    let mut children: Vec<&'a (dyn Error + 'static)> = Vec::new();
    if let Some(e) = error.downcast_ref::<swarm_enrollment_authority::startup::Error>() {
        labels.push(format!("enrollment-startup:{}", e.code()));
        if let Some(e) = e.cleanup_error() {
            children.push(e);
        }
        if let Some(e) = e.source() {
            children.push(e);
        }
    }
    if let Some(e) = error.downcast_ref::<swarm_enrollment_authority::control::Error>() {
        labels.push(format!("enrollment-control:{}", e.code()));
        if let swarm_enrollment_authority::control::Error::AuthorityRefusal { refusal, .. } = e {
            children.push(refusal.as_ref());
        }
        match e {
            swarm_enrollment_authority::control::Error::AfterStorage { storage, .. } => {
                if let Err(error) = storage.as_ref() {
                    children.push(error);
                }
            }
            swarm_enrollment_authority::control::Error::Close {
                storage: Some(error),
                ..
            } => children.push(error),
            _ => {}
        }
        if let Some(e) = e.source() {
            children.push(e);
        }
    }
    if let Some(e) = error.downcast_ref::<swarm_enrollment_config::Error>() {
        labels.push(format!("enrollment-config:{}", e.code()));
        if let Some(e) = e.cleanup_error() {
            children.push(e);
        }
        if let Some(e) = e.source() {
            children.push(e);
        }
    }
    if let Some(e) = error.downcast_ref::<swarm_enrollment_state::Error>() {
        labels.push(format!("enrollment-state:{}", e.code()));
        if e.is_busy() {
            labels.push("enrollment-state:busy".into());
        }
        match e {
            swarm_enrollment_state::Error::Close { close, .. } => children.push(close),
            swarm_enrollment_state::Error::Rollback { rollback, .. } => children.push(rollback),
            _ => {}
        }
        if let Some(e) = e.source() {
            children.push(e);
        }
    }
    if let Some(e) = error.downcast_ref::<swarm_enrollment_bootstrap::Error>() {
        labels.push(format!("enrollment-bootstrap:{}", e.code()));
        if let Some(e) = e.cleanup_error() {
            children.push(e);
        }
        if let swarm_enrollment_bootstrap::Error::Protected(e) = e {
            if let Some(cleanup) = e.cleanup_error() {
                children.push(cleanup);
            }
        }
        if let Some(e) = e.source() {
            children.push(e);
        }
    }
    (labels, children)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_primary_secondary_and_custom_io_reach_safe_cli_diagnostics() {
        let cause = swarm_enrollment_authority::startup::Error::Joined {
            cause: Box::new(swarm_enrollment_authority::startup::Error::Control(
                swarm_enrollment_authority::control::Error::AuthorityRefusal {
                    cause: swarm_enrollment_state::Error::Canceled,
                    refusal: Box::new(swarm_enrollment_authority::control::Error::Permission),
                },
            )),
            cleanup: swarm_enrollment_state::Error::Io(std::io::Error::other(
                "PRIVATE_STARTUP_CLOSE",
            )),
        };
        let error = CliError::Output(std::io::Error::other(CliError::ControlCheck {
            cause: Box::new(cause),
            english: true,
        }));
        let labels = crate::safe_causes(&error);
        for label in [
            "enrollment-state:canceled",
            "enrollment-state:io",
            "enrollment-control:permission",
        ] {
            assert!(
                labels.iter().any(|s| s == label),
                "missing {label}: {labels:?}"
            );
        }
        let nested_storage = || swarm_enrollment_config::Error::Joined {
            cause: Box::new(swarm_enrollment_config::Error::State),
            cleanup: Box::new(swarm_enrollment_config::Error::Pending),
        };
        for control in [
            swarm_enrollment_authority::control::Error::Close {
                authority: Some(swarm_enrollment_state::Error::Closed),
                storage: Some(nested_storage()),
            },
            swarm_enrollment_authority::control::Error::AfterStorage {
                cause: swarm_enrollment_state::Error::Canceled,
                storage: Box::new(Err(nested_storage())),
                effect_unknown: true,
            },
        ] {
            let labels = crate::safe_causes(&control);
            assert!(labels.iter().any(|s| s == "enrollment-config:state"));
            assert!(labels.iter().any(|s| s == "enrollment-config:pending"));
        }
        let mut stderr = Vec::new();
        let outcome = crate::report(error, true, true, &mut stderr);
        assert_ne!(outcome.code, 0);
        assert!(!String::from_utf8(stderr)
            .unwrap()
            .contains("PRIVATE_STARTUP_CLOSE"));
    }
}
