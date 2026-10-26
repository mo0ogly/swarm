//! Native reviewer transactions, private controller and explicit result recovery.
//! Normal Root/CLI execution shares the proven lifecycle; owned SDKs remain
//! test-only and grant no provider/model/human authenticity or task acceptance.
#[cfg(test)]
mod budget_sdk;
mod capture;
#[cfg(test)]
mod certificate_sdk;
mod claim;
mod failure;
#[cfg(test)]
mod failure_sdk;
mod gate;
mod guard;
mod journal;
mod material;
mod sources;
mod reply;
mod mission_authority;
mod process;
mod publication;
#[cfg(test)]
mod recovery_sdk;
#[cfg(test)]
mod root_identity_sdk;
mod stale;
#[cfg(test)]
mod stale_sdk;
mod runtime;
pub use runtime::{execute_review_gate, TaskReviewOutcome};
mod store;
mod transaction;

impl crate::RootStore {
    /// Exclude registered live/unresolved review work during scoped closure.
    /// Reads the authoritative normalized ledger in the caller transaction;
    /// neither an expired lease nor missing process observations releases it.
    /// No execution, refund or receipt is created; SQL causes are retained.
    pub(crate) fn require_idle_task_review(db: &rusqlite::Connection, work: &str, task: &str) -> Result<(), crate::AgentLaunchError> {
        if db.is_autocommit() { return Err(crate::AgentLaunchError::Refused("planning-closure-transaction-required")); }
        if store::active(db, work, task)?.is_some() { return Err(crate::AgentLaunchError::Refused("planning-closure-review-active")); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::root_store::{task_validation, RootStore};
    use serde::Deserialize;
    use std::{
        fs,
        io::{Read, Write},
        os::unix::{
            fs::PermissionsExt,
            net::UnixStream,
            process::{CommandExt, ExitStatusExt},
        },
        path::PathBuf,
        process::{Child, Command, Stdio},
    };
    /// Test-only libtest framing filter and finite evidence tee. Production
    /// sinks never strip a prefix. Exact header mismatch refuses, never silently
    /// discards provider data; all bytes stay in the fixture original first.
    struct HarnessOutput<'a> {
        raw: &'a mut Vec<u8>,
        forwarded: &'a mut dyn Write,
        header_bytes: usize,
        fail_provider: bool,
        acknowledgement: Option<PathBuf>,
        acknowledged: bool,
    }
    impl Write for HarnessOutput<'_> {
        /// Forward actual provider bytes into the live decoder/capture before
        /// the explicit sink-failure fixture. Chunk boundaries do not reset the
        /// exact SDK header check. This finite Vec is evidence, not product policy.
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            const HEADER: &[u8] = b"\nrunning 1 test\n";
            self.raw.extend_from_slice(bytes);
            let header = HEADER
                .len()
                .saturating_sub(self.header_bytes)
                .min(bytes.len());
            if bytes[..header] != HEADER[self.header_bytes..self.header_bytes + header] {
                return Err(std::io::Error::other(
                    "owned-review-harness-header-mismatch",
                ));
            }
            self.header_bytes += header;
            if bytes.len() > header {
                self.forwarded.write_all(&bytes[header..])?;
                // Test-only progress handshake: the provider cannot exit until
                // this actual pipe callback has fed its terminal to the decoder.
                let terminal = self.raw.ends_with(b"\n")
                    && self
                        .raw
                        .split(|b| *b == b'\n')
                        .rev()
                        .find(|line| !line.is_empty())
                        .is_some_and(|line| {
                            serde_json::from_slice::<serde_json::Value>(line)
                                .is_ok_and(|v| v["type"] == "turn.completed")
                        });
                if terminal && !self.acknowledged {
                    if let Some(path) = &self.acknowledgement {
                        let socket = std::os::unix::net::UnixDatagram::unbound()?;
                        socket.set_nonblocking(true)?;
                        let sent = socket.send_to(b"streamed", path)?;
                        if sent != b"streamed".len() {
                            return Err(std::io::Error::other("owned-review-short-stream-ack"));
                        }
                        self.acknowledged = true;
                    }
                }
                if self.fail_provider {
                    return Err(std::io::Error::other("owned-review-provider-sink-fixture"));
                }
            }
            Ok(bytes.len())
        }
        /// Forward memory flush, never EOF or persistence of fixture evidence.
        fn flush(&mut self) -> std::io::Result<()> {
            self.forwarded.flush()
        }
    }
    /// Finite stderr fixture evidence tee; every byte reaches the shared raw
    /// capture, with no JSONL decoding or production-policy override.
    struct HarnessStderr<'a> {
        raw: &'a mut Vec<u8>,
        forwarded: &'a mut dyn Write,
    }
    impl Write for HarnessStderr<'_> {
        /// Preserve exact observed test bytes and propagate capture IO failure.
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.raw.extend_from_slice(bytes);
            self.forwarded.write_all(bytes)?;
            Ok(bytes.len())
        }
        /// Forward memory-only flush; no EOF, provider success or fsync.
        fn flush(&mut self) -> std::io::Result<()> {
            self.forwarded.flush()
        }
    }
    /// Verify candidate publication only on a token-checked inert Root after a
    /// real waited reviewer. Every injected failure follows a valid preflight;
    /// retain original files before corruption and restore them exactly. SQL
    /// trigger faults are connection-local and never seed a positive receipt.
    /// Failed original calls remain running/exclusive for future reconciliation.
    /// No provider is invoked here; all replays compare the actual first witness.
    fn verify_publication(root: &RootStore, job: &store::Job, valid: bool) {
        let before = root.read_work(&job.work_id).unwrap();
        let before_bytes = crate::encode_model(&before).unwrap();
        let invocation = fs::read(root.root.join("reviewer-invoked.json")).unwrap();
        let unchanged = || {
            assert_eq!(
                crate::encode_model(&root.read_work(&job.work_id).unwrap()).unwrap(),
                before_bytes
            );
            assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(job));
        };
        if !valid {
            assert!(publication::inspect(root, &job.id).is_err());
            assert!(publication::record(root, &job.id).is_err());
            unchanged();
            fs::write(root.root.join("owned-review-publication.json"),serde_json::to_vec(&serde_json::json!({"business_result_published":false,"failed_original_refused":true,"task_accepted":false})).unwrap()).unwrap();
            return;
        }
        publication::inspect(root, &job.id).unwrap();
        unchanged();
        let readonly = RootStore::open(&root.root, false).unwrap();
        assert!(matches!(
            publication::record(&readonly, &job.id),
            Err(crate::RootStoreError::Refused("root-read-only"))
        ));
        readonly.close().unwrap();
        let report = before
            .tasks
            .iter()
            .flatten()
            .find(|t| t.id == job.task_id)
            .unwrap()
            .independent_review
            .as_ref()
            .unwrap()
            .report
            .clone();
        let source = root.root.join(report);
        let source_original = fs::read(&source).unwrap();
        fs::write(
            root.root.join("owned-review-publication-source-original"),
            &source_original,
        )
        .unwrap();
        fs::write(
            &source,
            b"Owned negative source changed after reviewer receipt",
        )
        .unwrap();
        assert!(publication::record(root, &job.id).is_err());
        unchanged();
        fs::write(&source, &source_original).unwrap();
        publication::inspect(root, &job.id).unwrap();
        let private = root.root.join(".swarm").join(journal::name(&job.id));
        let private_original = fs::read(&private).unwrap();
        let (original, _) = journal::read(root, &job.id).unwrap();
        let mut invalid = serde_json::to_value(original).unwrap();
        invalid["original_reply"] = serde_json::json!(b"PRIVATE invalid retained review".to_vec());
        fs::write(
            &private,
            crate::native_receipt::encode(&serde_json::to_vec(&invalid).unwrap()).unwrap(),
        )
        .unwrap();
        let error = publication::record(root, &job.id)
            .err()
            .expect("invalid retained reply accepted");
        assert!(!error.to_string().contains("PRIVATE"));
        unchanged();
        fs::write(&private, &private_original).unwrap();
        publication::inspect(root, &job.id).unwrap();
        root.db.execute_batch("CREATE TEMP TRIGGER fixture_review_publication_fail BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-review-publication-sql-fixture'); END;").unwrap();
        let error = publication::record(root, &job.id)
            .err()
            .expect("SQL publication fault accepted");
        fs::write(
            root.root.join("owned-review-publication-sql-failure.json"),
            serde_json::to_vec(&crate::native_diagnostics::project(&error)).unwrap(),
        )
        .unwrap();
        unchanged();
        let event = format!("{}-result", job.id);
        let count: i64 = root
            .db
            .query_row("SELECT count(*) FROM events WHERE id=?", [&event], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        root.db
            .execute_batch("DROP TRIGGER fixture_review_publication_fail")
            .unwrap();
        let public = super::certificate_sdk::one_candidate(root, &job.id);
        let public_original = fs::read(&public).unwrap();
        fs::write(
            root.root.join("owned-review-public-certificate.json"),
            &public_original,
        )
        .unwrap();
        let mut broken = public_original.clone();
        *broken.last_mut().unwrap() ^= 1;
        fs::write(&public, broken).unwrap();
        assert!(publication::record(root, &job.id).is_err());
        unchanged();
        fs::write(&public, &public_original).unwrap();
        publication::inspect(root, &job.id).unwrap();
        let recorded = publication::record(root, &job.id).unwrap();
        assert!(recorded.review_recorded && !recorded.replayed && !recorded.task_accepted);
        assert_eq!(recorded.work.revision, before.revision + 1);
        let task = recorded
            .work
            .tasks
            .iter()
            .flatten()
            .find(|t| t.id == job.task_id)
            .unwrap();
        let review = task.independent_review.as_ref().unwrap();
        assert_eq!(
            task.status,
            if review.state == "passed" {
                "submitted"
            } else {
                "blocked"
            }
        );
        assert!(matches!(
            review.state.as_str(),
            "passed" | "changes_requested"
        ));
        assert!(review.usage.is_none());
        assert_eq!(
            review.receipt_sha256,
            crate::provider_configuration::configuration_digest(&public_original)
        );
        let inbox: Vec<_> = recorded
            .work
            .planning
            .as_ref()
            .unwrap()
            .inbox
            .iter()
            .flatten()
            .filter(|e| e.id == job.id)
            .collect();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].scope, task.scope_id);
        assert_eq!(inbox[0].kind, "independent_review");
        let scope = recorded
            .work
            .planning
            .as_ref()
            .unwrap()
            .scopes
            .iter()
            .flatten()
            .find(|s| s.id == task.scope_id)
            .unwrap();
        let context = crate::planning_scope_context::observe(
            &root.root,
            &recorded.work,
            scope,
            &root.read_settings().unwrap(),
        )
        .expect("published native review must reach supervisor context");
        let content: serde_json::Value = serde_json::from_slice(&context.bytes).unwrap();
        assert!(content["reviews"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == job.id));
        assert!(content["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["event_id"] == job.id));
        assert!(recorded
            .work
            .planning
            .as_ref()
            .unwrap()
            .inbox
            .iter()
            .flatten()
            .find(|e| e.id == job.id)
            .unwrap()
            .decision
            .is_empty());
        assert!(context
            .delivery
            .events
            .iter()
            .flatten()
            .any(|id| id == &job.id));
        fs::write(
            root.root.join("owned-review-supervisor-context.json"),
            serde_json::to_vec(&content).unwrap(),
        )
        .unwrap();
        let context_before = crate::encode_model(&root.read_work(&job.work_id).unwrap()).unwrap();
        let observe = |work: &crate::Work| {
            crate::planning_scope_context::observe(
                &root.root,
                work,
                scope,
                &root.read_settings().unwrap(),
            )
        };
        fs::write(
            &source,
            b"Owned negative report changed after recorded review",
        )
        .unwrap();
        assert!(observe(&recorded.work).is_err());
        fs::write(&source, &source_original).unwrap();
        observe(&recorded.work).unwrap();
        let mut bad = public_original.clone();
        *bad.last_mut().unwrap() ^= 1;
        fs::write(&public, bad).unwrap();
        assert!(observe(&recorded.work).is_err());
        fs::write(&public, &public_original).unwrap();
        observe(&recorded.work).unwrap();
        // Fixture suffix, not a product size policy. A healthy witness precedes
        // the oversized certificate; restoration must recover the same context.
        let mut oversized = public_original.clone();
        oversized.extend_from_slice(b" extra fixture bytes");
        fs::write(&public, oversized).unwrap();
        assert!(observe(&recorded.work).is_err());
        fs::write(&public, &public_original).unwrap();
        observe(&recorded.work).unwrap();
        let mut bad = private_original.clone();
        *bad.last_mut().unwrap() ^= 1;
        fs::write(&private, bad).unwrap();
        assert!(observe(&recorded.work).is_err());
        fs::write(&private, &private_original).unwrap();
        observe(&recorded.work).unwrap();
        let mut forged = recorded.work.clone();
        forged
            .planning
            .as_mut()
            .unwrap()
            .inbox
            .iter_mut()
            .flatten()
            .find(|e| e.id == job.id)
            .unwrap()
            .message
            .push_str(" forged context only");
        assert!(observe(&forged).is_err());
        observe(&recorded.work).unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&job.work_id).unwrap()).unwrap(),
            context_before
        );
        fs::write(root.root.join("owned-review-consumer-faults.json"),serde_json::to_vec(&serde_json::json!({"context_materialized":true,"original_report_present":true,"public_changed_refused":true,"public_oversized_refused":true,"private_changed_refused":true,"report_changed_refused":true,"forged_inbox_refused":true,"work_unchanged":true,"decision_acknowledged":false,"model_received":false})).unwrap()).unwrap();
        let job_done = store::job(&root.db, &job.id).unwrap().unwrap();
        assert!(job_done.phase == store::Phase::Recorded);
        assert_eq!(job_done.published_revision, Some(recorded.work.revision));
        let duplicate = publication::record(root, &job.id).unwrap();
        assert!(duplicate.replayed && !duplicate.task_accepted);
        assert_eq!(
            crate::encode_model(&duplicate.work).unwrap(),
            crate::encode_model(&recorded.work).unwrap()
        );
        let mut bad_public = public_original.clone();
        *bad_public.last_mut().unwrap() ^= 1;
        fs::write(&public, bad_public).unwrap();
        assert!(
            publication::record(root, &job.id).is_err(),
            "changed published certificate accepted on replay"
        );
        fs::write(&public, &public_original).unwrap();
        assert!(publication::record(root, &job.id).unwrap().replayed);
        let mut broken = private_original.clone();
        *broken.last_mut().unwrap() ^= 1;
        fs::write(&private, broken).unwrap();
        assert!(publication::record(root, &job.id).is_err());
        fs::write(&private, &private_original).unwrap();
        assert!(publication::record(root, &job.id).unwrap().replayed);
        let count: i64 = root
            .db
            .query_row("SELECT count(*) FROM events WHERE id=?", [&event], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            fs::read(root.root.join("reviewer-invoked.json")).unwrap(),
            invocation
        );
        fs::write(root.root.join("owned-review-publication.json"),serde_json::to_vec(&serde_json::json!({"business_result_published":true,"review_state":review.state,"task_status":task.status,"task_accepted":false,"source_changed_refused":true,"invalid_reply_refused":true,"sql_rollback_verified":true,"changed_public_original_refused":true,"replay_without_call":true,"changed_private_replay_refused":true,"inbox_events":inbox.len(),"published_revision":recorded.work.revision})).unwrap()).unwrap();
    }
    /// Owned SDK helper or its explicitly declared inert fixture exec image.
    /// Preserve cleanup on assertion failure; no paid provider or descendant
    /// workload is started by these helpers.
    struct OwnedChild(Option<Child>);
    impl Drop for OwnedChild {
        /// Kill/wait only this owned fixture on early test failure. Cleanup errors
        /// remain visible with the original failing test; this is not the product
        /// controller's pidfd/unreaped-process recovery mechanism.
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                // During panic cleanup no success is reported. Keep both cleanup
                // causes visible without concealing the original test failure.
                if let Err(error) = child.kill() {
                    eprintln!("owned helper kill: {error}");
                }
                if let Err(error) = child.wait() {
                    eprintln!("owned helper wait: {error}");
                }
            }
        }
    }
    /// Run only as an explicitly owned test child on a private Unix stdin socket.
    /// Actual gate material is checked, then dropped without preparing exec.
    /// Valid/invalid/valid/source-refusal/valid exchanges prove no provider call.
    #[test]
    #[ignore = "private child selected by owned candidate SDK recipe only"]
    fn reviewer_gate_owned_child() {
        let input: Input = serde_json::from_slice(
            &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture")).unwrap(),
            input.token
        );
        let id = std::env::var("SWARM_OWNED_REVIEW_GATE_ID").unwrap();
        let descriptor = rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3).unwrap();
        let mut channel = UnixStream::from(descriptor);
        for expected in [b'v', b'r', b'v', b'r', b'v'] {
            let result = gate::admit(&input.root, &id, &mut channel);
            if expected == b'v' {
                let material = result.unwrap();
                material.fresh(&input.root).unwrap();
            } else {
                assert!(result.is_err());
            }
            channel.write_all(&[expected]).unwrap();
        }
        nix::unistd::close(channel).unwrap();
        println!("REVIEW_GATE_ACTUAL_CHILD_NO_EXEC");
    }
    #[derive(Deserialize)]
    struct Input {
        root: PathBuf,
        work: String,
        request: serde_json::Value,
        token: String,
        #[serde(default)]
        controller_case: String,
        #[serde(default)]
        stream_handshake: bool,
        #[serde(default)]
        publish_review: bool,
        #[serde(default)]
        reconcile_failed: bool,
    }
    /// Exercise actual completed public worker/control evidence through the SDK
    /// candidate. An explicit owned fixture marker is required before any SQL
    /// fault. Also exercises an actual owned SDK child/Unix gate without exec.
    /// Repeated gate checks are a test protocol only: production must consume
    /// transport once by exec. This proves no production wiring or review verdict.
    #[test]
    #[ignore = "requires the owned public fixture recipe; never a user/live Root"]
    fn reviewer_claim_owned_public_fixture_rollback_and_replay() {
        let input: Input = serde_json::from_slice(
            &fs::read(
                std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE")
                    .expect("explicit fixture input required"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::metadata(&input.root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture")).unwrap(),
            input.token
        );
        let root = RootStore::open(&input.root, true).unwrap();
        let reservation = root.directory.reserve(false, false).unwrap();
        let settings = root.read_settings().unwrap();
        let request =
            task_validation::decode(&crate::encode_model(&input.request).unwrap()).unwrap();
        let before = root.read_work(&input.work).unwrap();
        let encoded = crate::encode_model(&before).unwrap();
        let calls = before
            .planning
            .as_ref()
            .unwrap()
            .reviewer
            .as_ref()
            .unwrap()
            .calls;
        let witness = fs::read(input.root.join("validation-executed.json")).unwrap();
        let mut tx = root.db.unchecked_transaction().unwrap();
        let prepared =
            claim::stage(&root, &mut tx, settings.clone(), &input.work, &request).unwrap();
        prepared.material.fresh(&root.root).unwrap();
        assert!(prepared.material.duration.is_none());
        assert_eq!(prepared.work.revision, before.revision + 1);
        assert_eq!(
            prepared
                .work
                .planning
                .as_ref()
                .unwrap()
                .reviewer
                .as_ref()
                .unwrap()
                .calls,
            calls + 1
        );
        assert!(prepared
            .work
            .tasks
            .as_ref()
            .unwrap()
            .iter()
            .any(|task| task.id == request.task_id
                && task.status == "submitted"
                && task.independent_review.as_ref().unwrap().state == "running"));
        assert!(store::job(&tx, &prepared.job.id).unwrap().as_ref() == Some(&prepared.job));
        assert_eq!(prepared.job.permission, 0);
        assert!(prepared.job.child_pid.is_none());
        let estimate: f64 = tx
            .query_row(
                "SELECT estimate FROM planning_calls WHERE id=?",
                [&prepared.job.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(estimate > 0.0);
        let foreign_key_faults: i64 = tx
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            foreign_key_faults, 0,
            "healthy staged claim must have no FK fault before injection"
        );
        assert!(claim::record(&root, &input.work, &request).is_err());
        assert!(
            !root.db.is_autocommit(),
            "nested refusal must leave caller transaction active"
        );
        assert!(store::job(&tx, &prepared.job.id).unwrap().as_ref() == Some(&prepared.job));
        tx.rollback().unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            encoded
        );
        assert!(!store::schema(&root.db, false).unwrap());
        transaction::apply(&root, |_| Ok(transaction::Change::Mutate(()))).unwrap();
        assert!(root.db.is_autocommit());
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            encoded
        );
        root.db.execute_batch("CREATE TABLE owned_review_commit_fault(id TEXT PRIMARY KEY,work_id TEXT REFERENCES works(id) DEFERRABLE INITIALLY DEFERRED); CREATE TEMP TRIGGER reject_review_commit_fixture AFTER INSERT ON main.events WHEN NEW.kind='review.claim' BEGIN INSERT INTO owned_review_commit_fault VALUES('owned-invalid-commit','missing-owned-work'); END;").unwrap();
        let error = match claim::record(&root, &input.work, &request) {
            Err(error) => error,
            Ok(_) => panic!("deferred FK must fail actual COMMIT"),
        };
        assert_eq!(error.code(), "agent-launch-commit-refused");
        assert!(error.recorded_work().is_none());
        assert!(root.db.is_autocommit());
        root.db
            .execute_batch(
                "DROP TRIGGER reject_review_commit_fixture; DROP TABLE owned_review_commit_fault",
            )
            .unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            encoded
        );
        assert!(!store::schema(&root.db, false).unwrap());
        root.db.execute_batch("CREATE TEMP TRIGGER reject_review_fixture BEFORE INSERT ON main.events WHEN NEW.kind='review.claim' BEGIN SELECT RAISE(ABORT,'PRIVATE reviewer fixture publication fault'); END;").unwrap();
        let error = match claim::record(&root, &input.work, &request) {
            Ok(_) => panic!("publication fault must refuse"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains("PRIVATE"));
        assert!(root.db.is_autocommit());
        root.db
            .execute_batch("DROP TRIGGER reject_review_fixture")
            .unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            encoded
        );
        assert!(!store::schema(&root.db, false).unwrap());
        let recovered = match claim::record(&root, &input.work, &request).unwrap() {
            claim::Recorded::New(value) => *value,
            claim::Recorded::Replay(_) => panic!("first actual recovery must create one claim"),
        };
        let id = recovered.job.id.clone();
        let after = root.read_work(&input.work).unwrap();
        assert_eq!(
            after
                .planning
                .as_ref()
                .unwrap()
                .reviewer
                .as_ref()
                .unwrap()
                .calls,
            calls + 1
        );
        let tx = root.db.unchecked_transaction().unwrap();
        guard::owner(&recovered.job).unwrap();
        let observed = guard::fresh(&root, &tx, settings.clone(), &recovered.job).unwrap();
        guard::observe_sources(&root, &tx, settings.clone(), &recovered.job).unwrap();
        tx.execute_batch("SAVEPOINT missing_review_funding")
            .unwrap();
        assert_eq!(
            tx.execute("DELETE FROM planning_calls WHERE id=?", [&id])
                .unwrap(),
            1
        );
        assert!(
            guard::fresh(&root, &tx, settings.clone(), &recovered.job).is_err(),
            "missing original reserve must refuse before grant"
        );
        tx.execute_batch("ROLLBACK TO missing_review_funding; RELEASE missing_review_funding")
            .unwrap();
        guard::fresh(&root, &tx, settings.clone(), &recovered.job).unwrap();
        // Healthy current evidence precedes each independent fault. Owned fixture
        // files are restored byte-for-byte; no provider executes during this test.
        let report_path = root.root.join(&observed.submitted.report_path);
        let report = fs::read(&report_path).unwrap();
        fs::write(&report_path, b"changed owned report").unwrap();
        assert!(guard::observe_sources(&root, &tx, settings.clone(), &recovered.job).is_err());
        assert!(guard::fresh(&root, &tx, settings.clone(), &recovered.job).is_err());
        fs::write(&report_path, &report).unwrap();
        guard::observe_sources(&root, &tx, settings.clone(), &recovered.job).unwrap();
        guard::fresh(&root, &tx, settings.clone(), &recovered.job).unwrap();
        let executable_path = PathBuf::from(&observed.model.provider().command);
        let executable = fs::read(&executable_path).unwrap();
        let mut changed = executable.clone();
        changed.extend_from_slice(b"\n# owned fixture executable changed\n");
        fs::write(&executable_path, changed).unwrap();
        assert!(guard::fresh(&root, &tx, settings.clone(), &recovered.job).is_err());
        fs::write(&executable_path, executable).unwrap();
        guard::fresh(&root, &tx, settings.clone(), &recovered.job).unwrap();
        let mut changed_settings = settings.clone();
        changed_settings.revision.0 += 1;
        assert!(guard::fresh(&root, &tx, changed_settings, &recovered.job).is_err());
        guard::fresh(&root, &tx, settings.clone(), &recovered.job).unwrap();
        let mut wrong_owner = recovered.job.clone();
        wrong_owner.owner_stamp.push_str("-changed");
        assert!(guard::owner(&wrong_owner).is_err());
        guard::owner(&recovered.job).unwrap();
        // Public preview continues to reject running reviews; only this internal
        // exact job reobservation can read through the already-claimed record.
        let mut current_request = request.clone();
        current_request.expected_revision = after.revision;
        assert!(root
            .prepare_review_snapshot(&tx, settings.clone(), &input.work, &current_request)
            .is_err());
        tx.rollback().unwrap();
        let mut tx = root.db.unchecked_transaction().unwrap();
        assert!(claim::stage(&root, &mut tx, settings, &input.work, &request).is_err());
        tx.rollback().unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            crate::encode_model(&after).unwrap()
        );
        assert_eq!(
            fs::read(input.root.join("validation-executed.json")).unwrap(),
            witness
        );
        assert!(!input.root.join("reviewer-invoked.json").exists());
        match claim::record(&root, &input.work, &request).unwrap() {
            claim::Recorded::Replay(job) => assert_eq!(job.id, id),
            claim::Recorded::New(_) => panic!("exact replay cannot debit again"),
        }
        let mut conflict = request.clone();
        conflict.task_id = "different-owned-task".into();
        assert!(claim::record(&root, &input.work, &conflict).is_err());
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            crate::encode_model(&after).unwrap()
        );
        let job = store::job(&root.db, &id).unwrap().unwrap();
        assert_eq!(job.permission, 0);
        assert!(job.child_pid.is_none());
        let (mut channel, input_channel) = UnixStream::pair().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "root_store::review_preview::reviewer_candidate::tests::reviewer_gate_owned_child",
            "--ignored",
            "--nocapture",
        ]);
        command.env("SWARM_OWNED_REVIEW_GATE_ID", &id);
        command.stdin(Stdio::from(std::os::fd::OwnedFd::from(input_channel)));
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = OwnedChild(Some(command.spawn().unwrap()));
        drop(command); // Parent must not retain the child socket end through Stdio.
        let pid = child.0.as_ref().unwrap().id();
        let mut registered = job.clone();
        gate::register_recorded(&root, &mut registered, pid).unwrap();
        assert_eq!(registered.permission, 0);
        let mut permitted = registered.clone();
        gate::permit_recorded(&root, &mut permitted).unwrap();
        assert_eq!(permitted.permission, 1);
        // A later grant is prohibited: the monotonic row cannot be registered or
        // permitted again. No grant byte is sent by either staging function.
        let tx = root.db.unchecked_transaction().unwrap();
        assert!(gate::permit(&root, &tx, root.read_settings().unwrap(), &permitted).is_err());
        tx.rollback().unwrap();
        let exchange = |channel: &mut UnixStream, byte: u8, expected: u8| {
            channel.write_all(&[byte]).unwrap();
            let mut reply = [0u8; 1];
            channel.read_exact(&mut reply).unwrap();
            assert_eq!(reply, [expected]);
        };
        exchange(&mut channel, 1, b'v');
        exchange(&mut channel, 0, b'r');
        exchange(&mut channel, 1, b'v');
        fs::write(&report_path, b"owned source changed before gate").unwrap();
        exchange(&mut channel, 1, b'r');
        fs::write(&report_path, report).unwrap();
        exchange(&mut channel, 1, b'v');
        nix::unistd::close(channel).unwrap();
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("REVIEW_GATE_ACTUAL_CHILD_NO_EXEC"));
        assert!(stdout.contains("1 passed; 0 failed"));
        assert!(crate::observe_linux_process(pid)
            .unwrap_err()
            .confirms_absence());
        assert!(!input.root.join("reviewer-invoked.json").exists());
        println!("REVIEW_GATE_ACTUAL_CHILD_NO_EXEC");
        reservation.close().unwrap();
        root.close().unwrap();
        println!("REVIEW_CLAIM_CANDIDATE_ATOMIC_NO_EXEC");
    }
    /// Owned SDK child only. Unlike the admission-only helper this process is
    /// replaced once by the configured inert fixture image; no test verdict is
    /// returned by this helper. Production CLI wiring remains absent.
    #[test]
    #[ignore = "private exec child selected by owned SDK recipe only"]
    fn reviewer_exec_owned_child() {
        let input: Input = serde_json::from_slice(
            &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture")).unwrap(),
            input.token
        );
        let id = std::env::var("SWARM_OWNED_REVIEW_GATE_ID").unwrap();
        let descriptor = rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3).unwrap();
        let channel = UnixStream::from(descriptor);
        match gate::execute_reported(&input.root, &id, channel) {
            Ok(never) => match never {},
            Err(error) => panic!("owned exec gate refused: {error}"),
        }
    }
    /// Claim one review on original public worker/control evidence, then apply
    /// a real public checkpoint to unrelated mission summary/next fields. The
    /// complete reviewer prompt and source bindings must remain usable without
    /// another claim or provider. Marker and explicit binary isolate the fixture.
    /// A changed report and public planning pause must still refuse; report
    /// restoration recovers, but public resume must not revive stale controls. This
    /// exercises source observations, not live process continuation or settlement.
    #[test]
    #[ignore = "owned public fixture and explicit checkpoint binary required"]
    fn reviewer_unrelated_revision_owned_public_fixture() {
        let input: Input = serde_json::from_slice(
            &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture")).unwrap(),
            input.token
        );
        let binary = std::env::var_os("SWARM_OWNED_REVIEW_PUBLIC_BINARY").unwrap();
        let root = RootStore::open(&input.root, true).unwrap();
        let lock = root.directory.reserve(false, false).unwrap();
        let request =
            task_validation::decode(&crate::encode_model(&input.request).unwrap()).unwrap();
        let staged = match claim::record(&root, &input.work, &request).unwrap() {
            claim::Recorded::New(v) => *v,
            _ => panic!("new isolated claim required"),
        };
        let settings = root.read_settings().unwrap();
        let before = {
            let tx = root.db.unchecked_transaction().unwrap();
            let value =
                guard::publication_inputs(&root, &tx, settings.clone(), &staged.job).unwrap();
            tx.rollback().unwrap();
            value
        };
        let checkpoint = serde_json::json!({"schema_version":1,"event_id":"owned-unrelated-checkpoint","expected_revision":before.0.revision,"summary":"Independent mission bookkeeping; review originals unchanged","next":"Continue the same original reviewer"});
        let path = root.root.join("owned-review-checkpoint.input.json");
        fs::write(&path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let argv = [
            "checkpoint",
            input.work.as_str(),
            "--root",
            input.root.to_str().unwrap(),
            "--input",
            path.to_str().unwrap(),
            "--json",
        ];
        fs::write(
            root.root.join("owned-review-checkpoint.command.json"),
            serde_json::to_vec(&argv).unwrap(),
        )
        .unwrap();
        // Do not hold the SDK's shared settings reservation across another
        // public writer: CLI view publication requires an exclusive reservation.
        // The next observation rechecks the saved generation after reacquiring.
        lock.close().unwrap();
        let output = Command::new(&binary)
            .args(argv)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        fs::write(
            root.root.join("owned-review-checkpoint.stdout"),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            root.root.join("owned-review-checkpoint.stderr"),
            &output.stderr,
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let lock = root.directory.reserve(false, false).unwrap();
        let current = root.read_work(&input.work).unwrap();
        assert_eq!(current.revision, before.0.revision + 1);
        assert_eq!(current.tasks, before.0.tasks);
        let tx = root.db.unchecked_transaction().unwrap();
        let after = guard::publication_inputs(&root, &tx, settings.clone(), &staged.job);
        fs::write(root.root.join("owned-review-revision-observation.json"),serde_json::to_vec(&serde_json::json!({"before_revision":before.0.revision,"after_revision":current.revision,"task_unchanged":true,"source_check_ok":after.is_ok(),"provider_started":false})).unwrap()).unwrap();
        let after = after
            .expect("an unrelated checkpoint must not revoke unchanged complete review inputs");
        assert_eq!(before.1.prompt_sha256, after.1.prompt_sha256);
        assert_eq!(
            before.1.preview.validation.source_sha256,
            after.1.preview.validation.source_sha256
        );
        tx.rollback().unwrap();
        let source = staged
            .material
            .inputs
            .materials
            .iter()
            .find(|s| s.kind == "report")
            .unwrap();
        let report = root.root.join(&source.name);
        let original = fs::read(&report).unwrap();
        fs::write(&report, b"owned negative changed report").unwrap();
        {
            let tx = root.db.unchecked_transaction().unwrap();
            assert!(guard::publication_inputs(&root, &tx, settings.clone(), &staged.job).is_err());
            tx.rollback().unwrap();
        }
        fs::write(&report, original).unwrap();
        {
            let tx = root.db.unchecked_transaction().unwrap();
            guard::publication_inputs(&root, &tx, settings.clone(), &staged.job).unwrap();
            tx.rollback().unwrap();
        }
        assert_eq!(root.read_work(&input.work).unwrap(), current);
        // Product pause must revoke an otherwise unchanged original. Public
        // resume changes scope generations and must not revive old controls.
        lock.close().unwrap();
        let control = |kind: &str, event: &str| {
            let w = root.read_work(&input.work).unwrap();
            let value = serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":w.revision,"reason":"Explicit owned reviewer pause/resume guard"});
            let path = root.root.join(format!("owned-review-{kind}.input.json"));
            fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            let argv = [
                "planning",
                kind,
                input.work.as_str(),
                "--root",
                input.root.to_str().unwrap(),
                "--input",
                path.to_str().unwrap(),
                "--json",
            ];
            fs::write(
                root.root.join(format!("owned-review-{kind}.command.json")),
                serde_json::to_vec(&argv).unwrap(),
            )
            .unwrap();
            let out = Command::new(&binary)
                .args(argv)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            fs::write(
                root.root.join(format!("owned-review-{kind}.stdout")),
                &out.stdout,
            )
            .unwrap();
            fs::write(
                root.root.join(format!("owned-review-{kind}.stderr")),
                &out.stderr,
            )
            .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            root.read_work(&input.work).unwrap()
        };
        let paused = control("pause", "owned-review-pause");
        let lock = root.directory.reserve(false, false).unwrap();
        {
            let tx = root.db.unchecked_transaction().unwrap();
            let error = guard::publication_inputs(&root, &tx, settings.clone(), &staged.job)
                .err()
                .expect("paused source must refuse");
            assert!(error
                .to_string()
                .contains("task-validation-planning-paused"));
            tx.rollback().unwrap();
        }
        assert_eq!(root.read_work(&input.work).unwrap(), paused);
        lock.close().unwrap();
        let resumed = control("resume", "owned-review-resume");
        let lock = root.directory.reserve(false, false).unwrap();
        {
            let tx = root.db.unchecked_transaction().unwrap();
            let stale = guard::publication_inputs(&root, &tx, settings, &staged.job)
                .err()
                .expect("resume must not revive controls from earlier scope generations");
            assert!(stale.to_string().contains("review-native-controls-stale"));
            tx.rollback().unwrap();
        }
        assert_eq!(root.read_work(&input.work).unwrap(), resumed);
        assert_eq!(resumed.tasks, before.0.tasks);
        assert!(store::job(&root.db, &staged.job.id).unwrap().as_ref() == Some(&staged.job));
        fs::write(root.root.join("owned-review-pause-observation.json"),serde_json::to_vec(&serde_json::json!({"paused_refused":true,"resumed_stale_refused":true,"task_unchanged":true,"provider_started":false})).unwrap()).unwrap();
        assert!(!root.root.join("reviewer-invoked.json").exists());
        lock.close().unwrap();
        root.close().unwrap();
        println!("REVIEW_UNRELATED_REVISION_CONTINUITY_CHECKED");
    }
    /// Start from real public worker/control originals and a predeclared inert
    /// reviewer. Reserve once, run the owned BOOTTIME controller, commit actual
    /// helper registration/permission, sealed exec once, wait and decode real
    /// provider bytes during real pipe reads under saved reply/capture policies.
    /// Vec tees retain finite fixture evidence only, separately from capture.
    /// Duration and source cases preserve actual owned TERM/KILL/wait failures,
    /// unknown usage and held funding, then verify no second run or publication.
    /// SDK harness framing is removed explicitly; raw stdout/stderr are retained.
    /// Private receipts precede optional candidate business publication; no task acceptance.
    #[test]
    #[ignore = "owned public fixture with explicit inert reviewer required"]
    fn reviewer_execute_owned_public_fixture() {
        let input: Input = serde_json::from_slice(
            &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture")).unwrap(),
            input.token
        );
        assert!(!input.root.join("reviewer-invoked.json").exists());
        let root = RootStore::open(&input.root, true).unwrap();
        let lock = root.directory.reserve(false, false).unwrap();
        let settings = root.read_settings().unwrap();
        let request =
            task_validation::decode(&crate::encode_model(&input.request).unwrap()).unwrap();
        let staged = match claim::record(&root, &input.work, &request).unwrap() {
            claim::Recorded::New(value) => *value,
            claim::Recorded::Replay(_) => panic!("owned exec requires its first recorded claim"),
        };
        let case = if input.controller_case.is_empty() {
            "complete"
        } else {
            input.controller_case.as_str()
        };
        assert!([
            "complete",
            "duration",
            "source",
            "output-error",
            "domain-invalid",
            "changes-requested"
        ]
        .contains(&case));
        let valid_call = matches!(case, "complete" | "changes-requested");
        let source_original = if case == "source" {
            let report = staged
                .material
                .inputs
                .materials
                .iter()
                .find(|s| s.kind == "report")
                .unwrap();
            let path = input.root.join(&report.name);
            let original = fs::read(&path).unwrap();
            fs::write(input.root.join("owned-review-source-original"), &original).unwrap();
            Some((path, original))
        } else {
            None
        };
        fs::write(
            input.root.join("fixture-review-controller.json"),
            serde_json::to_vec(&serde_json::json!({
                "case": case, "report": source_original.as_ref().map(|(p,_)| p),"stream_handshake":input.stream_handshake
            }))
            .unwrap(),
        )
        .unwrap();
        let mut command = Command::new("/proc/self/exe");
        // libtest names omit the crate prefix; derive the remaining module
        // path so a later production-module move cannot silently collect zero.
        let module = module_path!().split_once("::").unwrap().1;
        let child_test = format!("{module}::reviewer_exec_owned_child");
        command.args(["--exact", &child_test, "--ignored", "--nocapture"]);
        command.env("SWARM_OWNED_REVIEW_GATE_ID", &staged.job.id);
        let originals: Vec<_> = staged
            .material
            .inputs
            .materials
            .iter()
            .map(|source| crate::ReviewQuoteOriginal {
                name: &source.name,
                kind: &source.quote_kind,
                content: &source.content,
            })
            .collect();
        let decoder = crate::ReviewOutputStream::from_saved_settings(
            &staged.material.settings.values,
            staged.material.inputs.preview.criterion_count,
            &originals,
        )
        .unwrap();
        assert_eq!(
            staged.material.settings.revision.0,
            staged.job.settings_revision
        );
        let saved_policy = staged
            .material
            .settings
            .values
            .agent_execution_policy()
            .unwrap();
        let transport = capture::Transport::new(saved_policy, decoder).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let observed = {
            let (mut capture_stdout, mut capture_stderr) = transport.sinks();
            let mut out = HarnessOutput {
                raw: &mut stdout,
                forwarded: &mut capture_stdout,
                header_bytes: 0,
                fail_provider: case == "output-error",
                acknowledgement: input
                    .stream_handshake
                    .then(|| input.root.join("review-ack.sock")),
                acknowledged: false,
            };
            let mut err = HarnessStderr {
                raw: &mut stderr,
                forwarded: &mut capture_stderr,
            };
            let result = process::run(&root, &staged.job, command, &mut out, &mut err).unwrap();
            assert_eq!(out.header_bytes, b"\nrunning 1 test\n".len());
            if input.stream_handshake {
                assert!(out.acknowledged);
                assert_eq!(
                    fs::read(input.root.join("owned-review-stream-handshake")).unwrap(),
                    b"streamed"
                );
            }
            result
        };
        let (raw_capture, mut stream) = transport.into_parts();
        let pid = u32::try_from(observed.job.child_pid.unwrap()).unwrap();
        let permitted = observed.job.clone();
        fs::write(input.root.join("owned-review-exec.stdout"), &stdout).unwrap();
        fs::write(input.root.join("owned-review-exec.stderr"), &stderr).unwrap();
        let status = match &observed.process {
            Ok(actual) => {
                assert_ne!(case, "output-error");
                assert!(!actual.timed_out && !actual.output_drain_timed_out);
                actual.usage.status
            }
            Err(error) => {
                assert_eq!(case, "output-error");
                assert_eq!(error.operation(), "retain-output");
                let cause = std::error::Error::source(error)
                    .unwrap()
                    .downcast_ref::<std::io::Error>()
                    .unwrap();
                assert_eq!(cause.to_string(), "owned-review-provider-sink-fixture");
                assert!(error.unresolved_process_id().is_none());
                assert!(error.ownership_lost_process_id().is_none());
                error
                    .reaped_exit_status()
                    .expect("actual wait must survive output failure")
            }
        };
        assert!(observed.grant_attempted && observed.channel_closed);
        assert!(!observed.drain_expired);
        match case {
            "complete" | "changes-requested" => {
                assert!(status.success(), "{}", String::from_utf8_lossy(&stderr));
                assert!(observed.interruption.is_none() && !observed.duration_expired);
            }
            "duration" => {
                assert!(observed.duration_expired && observed.interruption.is_none());
                assert_eq!(
                    status.signal(),
                    Some(rustix::process::Signal::KILL.as_raw())
                );
            }
            "source" => {
                assert!(!observed.duration_expired && observed.interruption.is_some());
                assert!(!status.success());
            }
            "domain-invalid" => assert!(
                status.success() && observed.interruption.is_none() && !observed.duration_expired
            ),
            "output-error" => assert!(
                observed.process.is_err()
                    && observed.interruption.is_none()
                    && !observed.duration_expired
            ),
            _ => unreachable!(),
        }
        assert!(observed.gate.is_none() && observed.gate_bytes.is_empty());
        assert!(observed.transport_errors.is_empty());
        fs::write(input.root.join("owned-review-process-observation.json"), serde_json::to_vec(&serde_json::json!({
            "actual_pid": pid, "reaped": true, "grant_attempted": observed.grant_attempted,
            "max_source_observation_ns": observed.max_source_observation.as_nanos(),
            "duration_expired": observed.duration_expired, "channel_closed": observed.channel_closed,
            "controller_case": case, "source_interrupted": observed.interruption.is_some(),
            "exit_signal": status.signal(),
            "output_failed": observed.process.is_err(),
            "result_published": false, "controller_candidate": true,"stream_handshake":input.stream_handshake
        })).unwrap()).unwrap();
        assert!(crate::observe_linux_process(pid)
            .unwrap_err()
            .confirms_absence());
        let witness: serde_json::Value =
            serde_json::from_slice(&fs::read(input.root.join("reviewer-invoked.json")).unwrap())
                .unwrap();
        assert_eq!(witness["pid"], pid);
        assert_eq!(witness["ppid"], std::process::id());
        assert_eq!(witness["stdin_sha256"], staged.job.prompt_sha256);
        // Rust's SDK harness prints this exact pre-test header before exec. It
        // is not provider output; retain it above and remove only this prefix.
        let provider = stdout.strip_prefix(b"\nrunning 1 test\n").unwrap();
        raw_capture.validate().unwrap();
        assert_eq!(raw_capture.stdout.observed_bytes, provider.len() as u64);
        assert_eq!(raw_capture.stderr.observed_bytes, stderr.len() as u64);
        assert_eq!(
            raw_capture.stdout.sha256,
            crate::provider_configuration::configuration_digest(provider)
        );
        assert_eq!(
            raw_capture.stderr.sha256,
            crate::provider_configuration::configuration_digest(&stderr)
        );
        if !valid_call && case != "domain-invalid" {
            stream.incomplete_transport();
        }
        let reply = stream.finish();
        let facts = stream.facts_snapshot();
        let original_reply = reply
            .as_ref()
            .ok()
            .map(|r| r.transport.response.as_slice())
            .or_else(|| stream.retained_reply());
        let receipt = journal::observe(
            &observed,
            original_reply,
            stream.provider_facts().unwrap(),
            reply.as_ref().err(),
            crate::system_timestamp().unwrap(),
            raw_capture,
        )
        .unwrap();
        if case == "domain-invalid" {
            assert!(!reply
                .as_ref()
                .err()
                .unwrap()
                .to_string()
                .contains("PRIVATE"));
        }
        assert_eq!(receipt.failed, !valid_call);
        assert_eq!(receipt.output_complete, valid_call);
        assert!(receipt.waited);
        let hash = journal::persist(&root, &receipt).unwrap();
        let readonly = RootStore::open(&input.root, false).unwrap();
        assert!(matches!(
            journal::persist(&readonly, &receipt),
            Err(crate::RootStoreError::Refused("root-read-only"))
        ));
        assert_eq!(journal::read(&readonly, &permitted.id).unwrap().1, hash);
        readonly.close().unwrap();
        let (readback, read_hash) = journal::read(&root, &permitted.id).unwrap();
        assert_eq!(hash, read_hash);
        assert_eq!(readback.original_reply, receipt.original_reply);
        assert_eq!(readback.failed, receipt.failed);
        assert_eq!(
            serde_json::to_value(&readback.raw_capture).unwrap(),
            serde_json::to_value(&receipt.raw_capture).unwrap()
        );
        assert!(journal::persist(&root, &receipt).is_err());
        assert_eq!(journal::read(&root, &permitted.id).unwrap().1, hash);
        // Faults are confined to this token-verified owned fixture's private
        // receipt, never Work/ledger. Preserve and restore exact original frame.
        let path = input.root.join(".swarm").join(journal::name(&permitted.id));
        let original = fs::read(&path).unwrap();
        fs::write(
            input.root.join("owned-review-original-receipt.frame"),
            &original,
        )
        .unwrap();
        // A valid legacy v1 receipt stays explicitly unknown; it cannot satisfy
        // the strict raw-capture oracle or imply an empty successful capture.
        let mut legacy = serde_json::to_value(&receipt).unwrap();
        legacy["schema_version"] = 1.into();
        legacy.as_object_mut().unwrap().remove("raw_capture");
        fs::write(
            &path,
            crate::native_receipt::encode(&serde_json::to_vec(&legacy).unwrap()).unwrap(),
        )
        .unwrap();
        assert!(journal::read(&root, &permitted.id)
            .unwrap()
            .0
            .raw_capture
            .is_none());
        fs::write(&path, &original).unwrap();
        for key in ["retained_bytes", "observed_lines"] {
            let mut invalid = serde_json::to_value(&receipt).unwrap();
            let count = invalid["raw_capture"][key].as_u64().unwrap();
            invalid["raw_capture"][key] = (count + 1).into();
            fs::write(
                &path,
                crate::native_receipt::encode(&serde_json::to_vec(&invalid).unwrap()).unwrap(),
            )
            .unwrap();
            assert!(journal::read(&root, &permitted.id).is_err());
            fs::write(&path, &original).unwrap();
            assert_eq!(journal::read(&root, &permitted.id).unwrap().1, hash);
        }
        if !receipt.raw_capture.as_ref().unwrap().stdout.truncated {
            let mut invalid = serde_json::to_value(&receipt).unwrap();
            let local = invalid["raw_capture"]["stdout"]["logical_lines"]
                .as_u64()
                .unwrap();
            let total = invalid["raw_capture"]["observed_lines"].as_u64().unwrap();
            invalid["raw_capture"]["stdout"]["logical_lines"] = (local + 1).into();
            invalid["raw_capture"]["observed_lines"] = (total + 1).into();
            fs::write(
                &path,
                crate::native_receipt::encode(&serde_json::to_vec(&invalid).unwrap()).unwrap(),
            )
            .unwrap();
            assert!(journal::read(&root, &permitted.id).is_err());
            fs::write(&path, &original).unwrap();
        }
        let mut missing = serde_json::to_value(&receipt).unwrap();
        missing.as_object_mut().unwrap().remove("raw_capture");
        fs::write(
            &path,
            crate::native_receipt::encode(&serde_json::to_vec(&missing).unwrap()).unwrap(),
        )
        .unwrap();
        assert!(journal::read(&root, &permitted.id).is_err());
        fs::write(&path, &original).unwrap();
        let mut broken = original.clone();
        *broken.last_mut().unwrap() ^= 1;
        fs::write(&path, broken).unwrap();
        assert!(journal::read(&root, &permitted.id).is_err());
        fs::write(&path, &original).unwrap();
        let mut invalid = serde_json::to_value(&receipt).unwrap();
        invalid["waited"] = false.into();
        invalid["exit_code"] = 0.into();
        fs::write(
            &path,
            crate::native_receipt::encode(&serde_json::to_vec(&invalid).unwrap()).unwrap(),
        )
        .unwrap();
        assert!(journal::read(&root, &permitted.id).is_err());
        fs::write(&path, &original).unwrap();
        assert_eq!(journal::read(&root, &permitted.id).unwrap().1, hash);
        if !valid_call {
            let mut impossible = serde_json::to_value(&receipt).unwrap();
            impossible["process_attempted"] = false.into();
            impossible["grant_attempted"] = false.into();
            fs::write(
                &path,
                crate::native_receipt::encode(&serde_json::to_vec(&impossible).unwrap()).unwrap(),
            )
            .unwrap();
            assert!(
                journal::read(&root, &permitted.id).is_err(),
                "wait without process attempt accepted"
            );
            fs::write(&path, &original).unwrap();
            assert_eq!(journal::read(&root, &permitted.id).unwrap().1, hash);
        }
        fs::write(input.root.join("owned-review-private-receipt.json"),serde_json::to_vec(&serde_json::json!({"sha256":hash,"waited":receipt.waited,"failed":receipt.failed,"output_complete":receipt.output_complete,"create_only_refused":true,"corrupt_frame_refused":true,"inconsistent_fields_refused":true,"original_restored":true,"business_result_published":false,"raw_capture_streamed":true,"raw_capture":receipt.raw_capture})).unwrap()).unwrap();
        if valid_call {
            assert!(facts.usage.is_some());
            let expected = if case == "changes-requested" {
                crate::ReviewReplyState::ChangesRequested
            } else {
                crate::ReviewReplyState::Passed
            };
            assert_eq!(reply.unwrap().state, expected);
        } else {
            assert!(reply.is_err());
            if matches!(case, "source" | "duration") {
                assert!(facts.usage.is_none());
            }
            if case == "domain-invalid" {
                assert_eq!(
                    receipt.original_reply.as_deref(),
                    Some(b"PRIVATE invalid domain fixture".as_slice())
                );
                assert!(facts.usage.is_some()); // unverified declaration survives domain rejection.
            }
            // Declared facts may survive an IO failure; absent usage stays
            // unknown. Neither observation settles or refunds the reservation.
        }
        if let Some((path, original)) = source_original {
            fs::write(path, original).unwrap(); // Explicit owned negative fixture only.
            assert!(process::observe(&root, &permitted, false)
                .unwrap()
                .is_none());
        }
        let mut forbidden_stdout = Vec::new();
        let mut forbidden_stderr = Vec::new();
        assert!(matches!(
            process::run(
                &root,
                &permitted,
                Command::new("/proc/self/exe"),
                &mut forbidden_stdout,
                &mut forbidden_stderr
            ),
            Err(crate::RootStoreError::Refused(
                "review-process-new-claim-required"
            ))
        ));
        assert!(forbidden_stdout.is_empty() && forbidden_stderr.is_empty());
        let after = root.read_work(&input.work).unwrap();
        assert_eq!(
            crate::encode_model(&after).unwrap(),
            crate::encode_model(&staged.work).unwrap()
        );
        assert!(store::job(&root.db, &staged.job.id).unwrap().as_ref() == Some(&permitted));
        assert_eq!(after.tasks.as_ref().unwrap()[0].status, "submitted");
        let mut next_request = request.clone();
        next_request.event_id = "second-owned-review".into();
        next_request.expected_revision = after.revision;
        let mut tx = root.db.unchecked_transaction().unwrap();
        assert!(claim::stage(&root, &mut tx, settings, &input.work, &next_request).is_err());
        tx.rollback().unwrap();
        assert_eq!(
            crate::encode_model(&root.read_work(&input.work).unwrap()).unwrap(),
            crate::encode_model(&after).unwrap()
        );
        if input.publish_review {
            verify_publication(&root, &permitted, valid_call);
        }
        if input.reconcile_failed {
            assert!(!valid_call);
            super::failure_sdk::verify(&root, &permitted);
        }
        lock.close().unwrap();
        root.close().unwrap();
        if input.publish_review {
            println!("REVIEW_CANDIDATE_SEALED_EXEC");
            println!("REVIEW_CANDIDATE_PUBLICATION_CHECKED");
        } else {
            println!("REVIEW_CANDIDATE_SEALED_EXEC_NO_PUBLICATION");
        }
    }
}
