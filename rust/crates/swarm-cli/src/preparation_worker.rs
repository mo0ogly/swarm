//! Private API-text worker: only the engine's registered anonymous gate may
//! produce a network-capable request. No CLI provider command or Work dispatch.
use crate::{Arguments, CliError};
use swarm_engine::{RootStore, RootStoreError as E, SystemEnvironment};

/// Consume exactly a SHA256-sized private gate from stdin, claim the owned
/// turn, run cancellable HTTP under saved policy and persist its terminal state.
/// Any possible contact retains estimated budget; local cancellation says nothing
/// about remote completion/billing. Original failures survive finalization errors.
pub(super) fn execute(a: &Arguments) -> Result<u8, CliError> {
    use std::io::Read;
    if a.pos.len() != 2
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let mut token = [0u8; 64];
    let read = std::io::stdin()
        .lock()
        .read_exact(&mut token)
        .map_err(|e| swarm_engine::preparation_worker_error("preparation-worker-gate-read", e));
    let closed = nix::unistd::close(0)
        .map_err(|e| swarm_engine::preparation_worker_error("preparation-worker-gate-close", e));
    let token = match (read, closed) {
        (Ok(()), Ok(())) => std::str::from_utf8(&token).map_err(|e| {
            CliError::Root(Box::new(swarm_engine::preparation_worker_error(
                "preparation-worker-gate-invalid",
                e,
            )))
        })?,
        (Err(e), Ok(())) | (Ok(()), Err(e)) => return Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => {
            return Err(CliError::Root(Box::new(E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(cleanup),
            })))
        }
    };
    let root = RootStore::open(std::path::Path::new(&a.root), true)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let id = &a.pos[1];
    let result = (|| {
        let request = match root.preparation_worker_claim(id, token) {
            Ok(r) => r,
            Err(primary) => {
                let settled = root.preparation_worker_finish(
                    id,
                    None,
                    None,
                    &crate::safe_causes(&primary).join(","),
                    &mut SystemEnvironment,
                );
                return match settled {
                    Ok(_) => Err(primary),
                    Err(cleanup) => Err(E::Cleanup {
                        primary: Some(Box::new(primary)),
                        cleanup: Box::new(cleanup),
                    }),
                };
            }
        };
        let result = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => match request {
                swarm_engine::PreparationWorkerRequest::Api(request) => {
                    runtime.block_on(run(&root, id, request))
                }
                swarm_engine::PreparationWorkerRequest::Cli(_) => {
                    root.preparation_worker_run_cli(id, token)
                }
            },
            Err(e) => Err(swarm_engine::preparation_worker_error(
                "preparation-worker-runtime",
                e,
            )),
        };
        match result {
            Ok((content, usage)) => match root.preparation_worker_finish(
                id,
                Some(&content),
                usage.as_ref(),
                "",
                &mut SystemEnvironment,
            ) {
                Ok(_) => Ok(()),
                Err(primary) => match root.preparation_worker_finish(
                    id,
                    None,
                    None,
                    &crate::safe_causes(&primary).join(","),
                    &mut SystemEnvironment,
                ) {
                    Ok(_) => Err(primary),
                    Err(cleanup) => Err(E::Cleanup {
                        primary: Some(Box::new(primary)),
                        cleanup: Box::new(cleanup),
                    }),
                },
            },
            Err(primary) => match root.preparation_worker_finish(
                id,
                None,
                None,
                &crate::safe_causes(&primary).join(","),
                &mut SystemEnvironment,
            ) {
                Ok(_) => Err(primary),
                Err(cleanup) => Err(E::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(cleanup),
                }),
            },
        }
    })();
    match (result, root.close()) {
        (Ok(()), Ok(())) => Ok(0),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::Root(Box::new(E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }))),
    }
}
/// Run one HTTP future with caller-owned product timeout and stop observation.
/// Dropping the future cancels local IO only. No retry, fallback or paid fixture.
async fn run(
    root: &RootStore,
    id: &str,
    r: swarm_engine::PreparationApiRequest,
) -> Result<(String, Option<swarm_engine::Usage>), E> {
    let observation = std::time::Duration::from_millis(r.observation_ms);
    let timeout = r.timeout_seconds;
    let request = swarm_api_client::Request {
        base_url: r.base_url,
        key: r.key,
        model: r.model,
        prompt: r.prompt,
        transport: if r.transport == "sse" {
            swarm_api_client::Transport::Sse
        } else {
            swarm_api_client::Transport::Json
        },
        response_max_bytes: r.response_max_bytes,
        event_max_bytes: r.event_max_bytes,
        http1_max_headers: r.http1_max_headers,
        http2_max_header_bytes: r.http2_max_header_bytes,
    };
    let until = if timeout == 0 {
        None
    } else {
        Some(
            tokio::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(timeout))
                .ok_or(E::Refused("preparation-call-deadline-representation"))?,
        )
    };
    let call = swarm_api_client::complete(request);
    tokio::pin!(call);
    let deadline = async move {
        if let Some(until) = until {
            tokio::time::sleep_until(until).await
        } else {
            std::future::pending::<()>().await
        }
    };
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            result=&mut call=>{let response=result.map_err(|e|swarm_engine::preparation_worker_error("preparation-api-call",e))?;let usage=usage(response.usage.as_ref())?;return Ok((response.content,usage));},
            _=&mut deadline=>return Err(E::Refused("preparation-call-timeout")),
            _=tokio::time::sleep(observation)=>{if !root.preparation_worker_continue(id)?{return Err(E::Refused("preparation-call-interrupted"));}}
        }
    }
}
/// Preserve absence of vendor usage; accept only supplied nonnegative integral
/// input/output totals. These are provider declarations, never a measured invoice.
fn usage(value: Option<&serde_json::Value>) -> Result<Option<swarm_engine::Usage>, E> {
    let Some(value) = value else { return Ok(None) };
    let input = value
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .filter(|v| *v >= 0)
        .ok_or(E::Refused("preparation-api-usage-invalid"))?;
    let output = value
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .filter(|v| *v >= 0)
        .ok_or(E::Refused("preparation-api-usage-invalid"))?;
    Ok(Some(swarm_engine::Usage {
        input_tokens: input,
        output_tokens: output,
        source: "api-provider-declared".into(),
        scope: "preparation-turn".into(),
        ..Default::default()
    }))
}

/// Private helper entry: exact fixed-size stdin gate, never a user-facing grant.
/// Root verifies the actual registered parent and current source before exec.
/// Errors close stdin explicitly; no provider prompt appears in argv or logs.
pub(super) fn execute_cli_gate(a: &Arguments) -> Result<u8, CliError> {
    use std::io::Read;
    if a.pos.len() != 3
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let mut token = [0u8; 64];
    let read = std::io::stdin()
        .lock()
        .read_exact(&mut token)
        .map_err(|e| swarm_engine::preparation_worker_error("preparation-cli-gate-read", e));
    if let Err(primary) = read {
        let closed = nix::unistd::close(0)
            .map_err(|e| swarm_engine::preparation_worker_error("preparation-cli-gate-close", e));
        return Err(CliError::Root(Box::new(match closed {
            Ok(()) => primary,
            Err(cleanup) => E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(cleanup),
            },
        })));
    }
    let token = match std::str::from_utf8(&token) {
        Ok(token) => token,
        Err(error) => {
            let primary =
                swarm_engine::preparation_worker_error("preparation-cli-gate-invalid", error);
            let closed = nix::unistd::close(0).map_err(|e| {
                swarm_engine::preparation_worker_error("preparation-cli-gate-close", e)
            });
            return Err(CliError::Root(Box::new(match closed {
                Ok(()) => primary,
                Err(cleanup) => E::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(cleanup),
                },
            })));
        }
    };
    match swarm_engine::execute_preparation_cli_gate(
        std::path::Path::new(&a.root),
        &a.pos[1],
        token,
        std::path::Path::new(&a.pos[2]),
    ) {
        Ok(never) => match never {},
        Err(e) => Err(CliError::Root(Box::new(e))),
    }
}
