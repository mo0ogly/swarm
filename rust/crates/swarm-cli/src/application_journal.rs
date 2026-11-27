//! Safe boundary records for the Linux application journal.
use serde::Serialize;
use std::{io, path::PathBuf, time::Instant};

/// The native system journal is an OS transport. Swarm creates no alternate
/// file and inherits journal retention and visibility from systemd.
pub(super) struct Journal {
    socket: PathBuf,
}

/// Secondary diagnostic when a record could not be submitted. No transport
/// path or arbitrary IO text is exposed to clients or command output.
#[derive(Debug, Serialize)]
pub(super) struct Unavailable {
    code: &'static str,
    errno: Option<i32>,
}

impl From<io::Error> for Unavailable {
    fn from(error: io::Error) -> Self {
        Self {
            code: "application-journal-unavailable",
            errno: error.raw_os_error(),
        }
    }
}

#[derive(Serialize)]
struct Event {
    schema_version: u8,
    component: &'static str,
    operation: &'static str,
    level: &'static str,
    outcome: &'static str,
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_status: Option<u16>,
    known_work_effect: bool,
}

/// Map only application-owned command families. Never copy raw arguments,
/// task/work IDs, paths, provider names, URLs, or malformed input.
pub(super) fn operation(command: Option<&str>) -> &'static str {
    match command {
        Some("agent" | "agent-logs" | "agent-stop") => "agent",
        Some("identity" | "trust") => "identity-trust",
        Some("mission" | "mission-watch") => "mission",
        Some("planning" | "prepare") => "planning",
        Some("task") => "task",
        Some("work") => "work",
        Some("logs") => "logs",
        Some("web" | "web-ticket" | "web-rotate") => "web",
        Some("engine-settings" | "providers" | "run-limits" | "quotas") => "configuration",
        _ => "command",
    }
}

impl Journal {
    /// Select the service-manager endpoint on Linux. Other platforms preserve
    /// their original output contract without claiming journal availability.
    pub(super) fn system() -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            Some(Self {
                socket: PathBuf::from("/run/systemd/journal/socket"),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    /// Queue a single structured datagram with a classified command failure.
    /// Failure to reach journald never changes the command result.
    pub(super) fn command_failure(
        &self,
        operation: &'static str,
        code: &'static str,
        exit_code: u8,
        started: Instant,
        known_work_effect: bool,
    ) -> Result<(), Unavailable> {
        let refused = matches!(
            code,
            "usage"
                | "authorization_required"
                | "revision_conflict"
                | "event_conflict"
                | "task-validation-request-invalid"
                | "command_not_ported"
        );
        self.send(
            if refused { 4 } else { 3 },
            Event {
                schema_version: 1,
                component: "swarm-cli",
                operation,
                level: if refused { "warn" } else { "error" },
                outcome: if refused { "refused" } else { "failed" },
                code,
                elapsed_ms: Some(started.elapsed().as_millis()),
                exit_code: Some(exit_code),
                http_status: None,
                known_work_effect,
            },
        )
    }

    /// Record a terminal HTTP failure or a degraded read reported by the
    /// native observer. Caller supplies only the classified status/code and
    /// known-effect flag. No route, body, credential or response is retained.
    #[cfg(target_os = "linux")]
    pub(super) fn http_observation(
        &self,
        status: u16,
        code: &'static str,
        known_work_effect: bool,
    ) -> Result<(), Unavailable> {
        let (priority, outcome) = match status {
            500.. => (3, "failed"),
            400..=499 => (4, "refused"),
            _ => (4, "degraded"),
        };
        self.send(
            priority,
            Event {
                schema_version: 1,
                component: "swarm-web",
                operation: "http-request",
                level: if priority == 4 { "warn" } else { "error" },
                outcome,
                code,
                elapsed_ms: None,
                exit_code: None,
                http_status: Some(status),
                known_work_effect,
            },
        )
    }

    /// The native socket accepts one UTF-8 field per line. JSON escapes line
    /// breaks by construction. Send once, nonblocking, and report absence or
    /// congestion separately. Successful send does not establish retention.
    fn send(&self, priority: u8, event: Event) -> Result<(), Unavailable> {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixDatagram;
            let payload = serde_json::to_string(&event).map_err(io::Error::other)?;
            let record = format!(
                "SYSLOG_IDENTIFIER=swarm-rust\nPRIORITY={priority}\nSWARM_COMPONENT={}\nSWARM_OPERATION={}\nMESSAGE={payload}\n",
                event.component, event.operation
            );
            let socket = UnixDatagram::unbound()?;
            socket.set_nonblocking(true)?;
            let sent = socket.send_to(record.as_bytes(), &self.socket)?;
            if sent != record.len() {
                return Err(io::Error::from(io::ErrorKind::WriteZero).into());
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = (priority, event, &self.socket);
            Err(io::Error::from(io::ErrorKind::Unsupported).into())
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixDatagram;

    /// A local datagram receiver proves framing and classified data. Removing
    /// the owned endpoint proves failures remain independent of the business
    /// result. Neither path touches the workstation journal or live state.
    #[test]
    fn application_journal_records_safe_failures_and_preserves_cli_result() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal");
        let sink = UnixDatagram::bind(&path).unwrap();
        sink.set_nonblocking(true).unwrap();
        let journal = Journal {
            socket: path.clone(),
        };
        let command = [
            "task".into(),
            "review".into(),
            "private-work-sentinel".into(),
            "--json".into(),
        ];
        let mut stderr = Vec::new();
        let result =
            crate::run_detailed_using(&command, "en", &mut Vec::new(), &mut stderr, Some(&journal));
        assert_ne!(result.code, 0);
        let reply: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert!(reply.get("application_log").is_none());
        let mut buffer = [0; 4096];
        let count = sink.recv(&mut buffer).unwrap();
        let wire = std::str::from_utf8(&buffer[..count]).unwrap();
        assert!(!wire.contains("private-work-sentinel"));
        assert!(wire.contains("SWARM_OPERATION=task\n"));
        let event: serde_json::Value = serde_json::from_str(
            wire.lines()
                .find_map(|line| line.strip_prefix("MESSAGE="))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(event["code"], reply["failure"]["code"]);
        assert_eq!(event["exit_code"], result.code);
        assert_eq!(
            sink.recv(&mut buffer).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );

        journal
            .http_observation(403, "web-access-refused", false)
            .unwrap();
        let count = sink.recv(&mut buffer).unwrap();
        let wire = std::str::from_utf8(&buffer[..count]).unwrap();
        assert!(wire.contains("SWARM_COMPONENT=swarm-web\n"));
        assert!(wire.contains("\"http_status\":403"));
        std::fs::remove_file(path).unwrap();
        stderr.clear();
        let missing =
            crate::run_detailed_using(&command, "en", &mut Vec::new(), &mut stderr, Some(&journal));
        let degraded: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(missing.code, result.code);
        assert_eq!(degraded["failure"], reply["failure"]);
        assert_eq!(
            degraded["application_log"]["code"],
            "application-journal-unavailable"
        );
    }
}
