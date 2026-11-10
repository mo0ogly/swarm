//! Compile an explicit model choice into argv. No policy, catalogue or execution
//! authority is inferred here; callers must establish those before using output.
use crate::{AgentLaunchError, ModelChoice};

/// Provider CLI syntax only, never identity, account capability or permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderModelSyntax {
    /// Codex uses a TOML string override for reasoning effort.
    Codex,
    /// Claude and the existing generic CLI contract use a separate effort flag.
    ModelAndEffort,
}

/// Recognize only top-level model keys in a CLI config assignment. Whitespace
/// uses the Go-compatible trim set; unrelated sandbox/transport keys survive.
fn model_config(value: &str) -> bool {
    value.split_once('=').is_some_and(|(key, _)| {
        matches!(
            crate::text::trim_space(key),
            "model" | "model_reasoning_effort"
        )
    })
}

/// Recognize attached model spelling for the declared CLI syntax only. Codex
/// supports `-mVALUE`; the generic contract does not reinterpret unrelated
/// single-dash flags such as `-mode`. Split flags are handled by the caller;
/// absence means unrecognized syntax, never capability or execution authority.
pub(crate) fn attached_model_argument(syntax: ProviderModelSyntax, argument: &str) -> Option<&str> {
    argument.strip_prefix("--model=").or_else(|| {
        if syntax == ProviderModelSyntax::Codex {
            argument.strip_prefix("-m")
        } else {
            None
        }
    })
}

/// Refuse empty model, flag-like model/effort and terminal/control characters
/// without exposing supplied
/// text. This validates argv representation only, never catalogue availability.
fn choice_valid(choice: &ModelChoice) -> bool {
    !choice.model.is_empty()
        && !choice.model.starts_with('-')
        && !choice.effort.starts_with('-')
        && [&choice.model, &choice.effort]
            .iter()
            .all(|s| crate::agent_terminal_text::terminal_text(s) == s.as_str())
}

/// Produce new argv for an already selected model, preserving caller inputs.
/// Removes recognized model/effort flags and top-level model config overrides;
/// only Codex interprets attached `-mVALUE`, preserving generic single-dash flags.
/// keeps unrelated args and a final stdin `-`. Dangling value flags and `--`
/// refuse because their interpretation cannot safely be inferred. Embedded NUL
/// and control/flag-bearing choices refuse. Codex effort is TOML basic-string quoted;
/// no shell, IO, defaults, fallback, cost change or process occurs. Output remains
/// a candidate requiring persisted policy/catalogue and fresh execution checks.
pub fn compile_provider_model_arguments(
    syntax: ProviderModelSyntax,
    original: &[String],
    choice: &ModelChoice,
) -> Result<Vec<String>, AgentLaunchError> {
    if !choice_valid(choice) {
        return Err(AgentLaunchError::Refused("agent-model-choice-invalid"));
    }
    if original.iter().any(|v| v.contains('\0') || v == "--") {
        return Err(AgentLaunchError::Refused("agent-model-arguments-ambiguous"));
    }
    let mut output = Vec::new();
    let mut i = 0;
    while i < original.len() {
        let arg = original[i].as_str();
        if matches!(arg, "--model" | "-m" | "--effort") {
            let value = original.get(i + 1).ok_or(AgentLaunchError::Refused(
                "agent-model-argument-value-missing",
            ))?;
            if value.is_empty() || value.starts_with('-') {
                return Err(AgentLaunchError::Refused(
                    "agent-model-argument-value-missing",
                ));
            }
            i += 2;
            continue;
        }
        if let Some(value) =
            attached_model_argument(syntax, arg).or_else(|| arg.strip_prefix("--effort="))
        {
            if value.is_empty() || value.starts_with('-') {
                return Err(AgentLaunchError::Refused(
                    "agent-model-argument-value-missing",
                ));
            }
            i += 1;
            continue;
        }
        if matches!(arg, "-c" | "--config") {
            let value = original.get(i + 1).ok_or(AgentLaunchError::Refused(
                "agent-model-argument-value-missing",
            ))?;
            if value.is_empty() || value.starts_with('-') {
                return Err(AgentLaunchError::Refused(
                    "agent-model-argument-value-missing",
                ));
            }
            if !model_config(value) {
                output.push(original[i].clone());
                output.push(value.clone());
            }
            i += 2;
            continue;
        }
        let override_value = arg
            .strip_prefix("--config=")
            .or_else(|| arg.strip_prefix("-c"));
        if !override_value.is_some_and(model_config) {
            output.push(original[i].clone());
        }
        i += 1;
    }
    let stdin = output.last().is_some_and(|v| v == "-");
    if stdin {
        output.pop();
    }
    output.extend(["--model".to_owned(), choice.model.clone()]);
    if !choice.effort.is_empty() {
        match syntax {
            ProviderModelSyntax::Codex => {
                // Controls are refused above. For the remaining UTF-8, JSON
                // quotes/backslash escapes are valid TOML basic-string escapes.
                let quoted = serde_json::to_string(&choice.effort)
                    .map_err(|e| AgentLaunchError::caused("agent-model-effort-encode", e))?;
                output.extend(["-c".to_owned(), format!("model_reasoning_effort={quoted}")]);
            }
            ProviderModelSyntax::ModelAndEffort => {
                output.extend(["--effort".to_owned(), choice.effort.clone()]);
            }
        }
    }
    if stdin {
        output.push("-".to_owned());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Owned argv fixtures avoid shell interpretation; original inputs are kept
    /// for rejection and no-mutation assertions.
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }
    /// Explicit fixture model/effort are data, never production defaults.
    fn choice() -> ModelChoice {
        ModelChoice {
            model: "fixture-chosen".into(),
            effort: "fixture-effort".into(),
        }
    }
    /// Replace split/joined choices while preserving sandbox, tools and stdin.
    #[test]
    fn codex_replaces_all_conflicting_forms_without_losing_stdin_or_sandbox() {
        let input = args(&[
            "exec",
            "--json",
            "--model",
            "old",
            "-m",
            "older",
            "--model=oldest",
            "-mattached",
            "--effort",
            "high",
            "--effort=max",
            "-c",
            " model =\"old\"",
            "--config=model_reasoning_effort=\"old\"",
            "-cmodel=\"old\"",
            "--config",
            "\u{a0}model_reasoning_effort\u{a0}=\"old\"",
            "-c",
            "sandbox_mode=\"read-only\"",
            "--config=features.model=true",
            "--tools",
            "",
            "-",
        ]);
        let saved = input.clone();
        assert_eq!(
            compile_provider_model_arguments(ProviderModelSyntax::Codex, &input, &choice())
                .unwrap(),
            args(&[
                "exec",
                "--json",
                "-c",
                "sandbox_mode=\"read-only\"",
                "--config=features.model=true",
                "--tools",
                "",
                "--model",
                "fixture-chosen",
                "-c",
                "model_reasoning_effort=\"fixture-effort\"",
                "-"
            ])
        );
        assert_eq!(input, saved);
    }
    /// Generic single-dash flags survive while explicit model flags are replaced;
    /// no unrelated option/value pair becomes a positional prompt accidentally.
    #[test]
    fn generic_single_dash_flags_are_not_codex_attached_model_aliases() {
        let input = args(&["-mode", "safe", "--model", "old", "--tools", ""]);
        assert_eq!(
            compile_provider_model_arguments(
                ProviderModelSyntax::ModelAndEffort,
                &input,
                &choice()
            )
            .unwrap(),
            args(&[
                "-mode",
                "safe",
                "--tools",
                "",
                "--model",
                "fixture-chosen",
                "--effort",
                "fixture-effort"
            ])
        );
    }

    /// An empty selected effort inherits no previous CLI override.
    #[test]
    fn generic_effort_is_explicit_and_empty_choice_does_not_inherit_old_effort() {
        let input = args(&["-p", "--model=old", "--effort", "old", "--safe-mode"]);
        assert_eq!(
            compile_provider_model_arguments(
                ProviderModelSyntax::ModelAndEffort,
                &input,
                &choice()
            )
            .unwrap(),
            args(&[
                "-p",
                "--safe-mode",
                "--model",
                "fixture-chosen",
                "--effort",
                "fixture-effort"
            ])
        );
        let c = ModelChoice {
            effort: String::new(),
            ..choice()
        };
        assert_eq!(
            compile_provider_model_arguments(ProviderModelSyntax::ModelAndEffort, &input, &c)
                .unwrap(),
            args(&["-p", "--safe-mode", "--model", "fixture-chosen"])
        );
    }
    /// A valid control precedes each malformed flag; no supplied argv is mutated.
    #[test]
    fn malformed_flags_refuse_without_consuming_a_following_option() {
        for input in [
            args(&["--model"]),
            args(&["--model", "--safe-mode"]),
            args(&["--effort", ""]),
            args(&["--model="]),
            args(&["--effort="]),
            args(&["--config"]),
            args(&["-c", "--safe-mode"]),
            args(&["--model", "-"]),
        ] {
            assert!(
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &choice())
                    .is_ok()
            );
            let saved = input.clone();
            let error =
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &input, &choice())
                    .unwrap_err();
            assert_eq!(error.code(), "agent-model-argument-value-missing");
            assert_eq!(input, saved);
        }
    }
    /// Ambiguous execution input refuses safely and does not poison a later call.
    #[test]
    fn delimiter_and_nul_refuse_with_safe_code_and_valid_recovery() {
        for input in [args(&["exec", "--", "prompt"]), args(&["secret\0payload"])] {
            assert!(
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &choice())
                    .is_ok()
            );
            let error =
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &input, &choice())
                    .unwrap_err();
            assert_eq!(error.to_string(), "agent-model-arguments-ambiguous");
            assert!(!format!("{error:?}").contains("secret"));
            assert!(
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &choice())
                    .is_ok()
            );
        }
    }
    /// Preserve identifiers exactly or refuse; control and flag-like effort values
    /// cannot be silently sanitized or interpreted as new CLI options.
    #[test]
    fn choice_controls_refuse_instead_of_sanitizing_the_identifier() {
        for bad in [
            "",
            "-option",
            "model\n",
            "model\0",
            "model\u{202e}",
            "model\u{e0001}",
        ] {
            assert!(
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &choice())
                    .is_ok()
            );
            let c = ModelChoice {
                model: bad.into(),
                ..choice()
            };
            assert_eq!(
                compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &c)
                    .unwrap_err()
                    .code(),
                "agent-model-choice-invalid"
            );
        }
        for syntax in [
            ProviderModelSyntax::Codex,
            ProviderModelSyntax::ModelAndEffort,
        ] {
            for invalid_effort in ["bad\tvalue", "-c", "--model=other", "-1"] {
                assert!(compile_provider_model_arguments(syntax, &[], &choice()).is_ok());
                let c = ModelChoice {
                    effort: invalid_effort.into(),
                    ..choice()
                };
                assert_eq!(
                    compile_provider_model_arguments(syntax, &[], &c)
                        .unwrap_err()
                        .code(),
                    "agent-model-choice-invalid"
                );
            }
        }
    }
    /// Quoted Unicode and backslashes remain one explicit effort string argument.
    #[test]
    fn quoted_unicode_effort_roundtrips_as_one_toml_string_argument() {
        let c = ModelChoice {
            model: "fixture/é".into(),
            effort: "é\"\\x".into(),
        };
        let actual = compile_provider_model_arguments(ProviderModelSyntax::Codex, &[], &c).unwrap();
        assert_eq!(actual.len(), 4);
        assert_eq!(actual[1], c.model);
        assert_eq!(actual[3], "model_reasoning_effort=\"é\\\"\\\\x\"");
        let quoted = actual[3].strip_prefix("model_reasoning_effort=").unwrap();
        assert_eq!(serde_json::from_str::<String>(quoted).unwrap(), c.effort);
    }
}
