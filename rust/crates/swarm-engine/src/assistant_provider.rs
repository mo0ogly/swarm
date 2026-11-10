//! Go-compatible requested tool-free adapter preparation, not effective isolation.
use crate::Provider;
use std::{error::Error, fmt, os::unix::fs::PermissionsExt, path::Path};
/// Safe preparation failures; command paths, model names and private args remain
/// out of diagnostics. Underlying filesystem error is retained through source.
pub enum AssistantProviderError {
    Refused(&'static str),
    Io(std::io::Error),
}
impl AssistantProviderError {
    /// Return a static category, never an execution permission or provider probe.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(code) => code,
            Self::Io(_) => "assistant-provider-executable-unavailable",
        }
    }
}
impl fmt::Display for AssistantProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for AssistantProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AssistantProviderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
/// Inspect an operator-configured absolute command (regular file and executable
/// mode bits), then clone known Claude/harness/Codex argv as requested tool-free
/// sessions. Only the last --model/-m value is carried from coding args; no model
/// choice, account right or default is inferred. API bridge args remain unchanged
/// supplied data: the Root must separately validate its actual connection/role.
/// Unknown adapters and missing/nonregular/nonexecutable commands refuse; typed
/// IO causes survive without private data in Display/Debug. No child, provider
/// call, model request, file write, timeout or permission change occurs. Symlinks
/// follow std metadata as in Go; mode bits are not ACL/effective execution proof.
/// Static adapter flags are the existing role/protocol restrictions, not editable
/// budgets. Preparation proves neither effective external tool isolation, current
/// CLI flag support, authenticated identity nor future executable freshness.
pub fn prepare_assistant_provider(provider: &Provider) -> Result<Provider, AssistantProviderError> {
    use AssistantProviderError as E;
    let path = Path::new(&provider.command);
    if !path.is_absolute() {
        return Err(E::Refused("assistant-provider-absolute-command-required"));
    }
    let metadata = std::fs::metadata(path).map_err(E::Io)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(E::Refused("assistant-provider-executable-unavailable"));
    }
    if !provider.api_connection_id.is_empty() {
        return Ok(provider.clone());
    }
    let base = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    let initial: &[&str] = match base {
        "claude" | "skynet_harness" => &[
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--tools",
            "",
            "--safe-mode",
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "--no-session-persistence",
        ],
        "codex" => &[
            "exec",
            "--json",
            "--sandbox",
            "read-only",
            "--ignore-user-config",
            "--ignore-rules",
            "--ephemeral",
            "--skip-git-repo-check",
            "-c",
            r#"web_search="disabled""#,
        ],
        _ => return Err(E::Refused("assistant-provider-adapter-unported")),
    };
    let mut args: Vec<String> = initial.iter().map(|v| (*v).into()).collect();
    if base == "codex" {
        for feature in [
            "shell_tool",
            "unified_exec",
            "multi_agent",
            "apps",
            "hooks",
            "browser_use",
            "computer_use",
            "image_generation",
            "view_image",
            "code_mode",
            "code_mode_host",
            "tool_suggest",
        ] {
            args.push("--disable".into());
            args.push(feature.into());
        }
    }
    let original = provider.args.as_deref().unwrap_or_default();
    let mut model = "";
    let mut i = 0;
    while i + 1 < original.len() {
        if matches!(original[i].as_str(), "--model" | "-m") {
            model = &original[i + 1];
            i += 2;
        } else {
            i += 1;
        }
    }
    if !model.is_empty() {
        args.push("--model".into());
        args.push(model.into());
    }
    if base == "codex" {
        args.push("-".into());
    }
    let mut prepared = provider.clone();
    prepared.args = Some(args);
    Ok(prepared)
}
