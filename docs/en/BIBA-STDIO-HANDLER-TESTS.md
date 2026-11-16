# Additional stdio-consumer branches — unexecuted proposal

These eight additional tests form a separate suite. They change neither the
previous eight retention tests nor the frozen r8 caller under review. Exact
typed helpers are reused; the public run_git_stdio -> stdio_probe path and real
Python collector are exercised with subprocess fully doubled before candidate
loading. Proof/fixture modules are explicit doubles. Embedded programs are
written as text, never executed. No Biba/Git/driver/provider is called.

| Named case | Expected outcome, not executed |
| --- | --- |
| healthy_handler_control | One inert call, valid JSON, healthy public path |
| known_completion_malformed_and_postcheck_control | Known exit0, actual JSON and postcheck causes together |
| unknown_completion_does_not_read_or_parse_retained_bytes | Bytes present but unknown exit: no second read/parse |
| spawn_failure_does_not_read_or_parse_empty_channels | Failed spawn: no parsing empty channels |
| missing_stdout_hash_does_not_read_or_parse_again | Collector hash refused: no replacement read |
| changed_retained_stdout_groups_hash_and_original_errors | Known hash then changed bytes: integrity and original cause |
| unreadable_retained_stdout_groups_read_and_original_errors | Late read refusal: IO and original cause |
| final_failure_publication_keeps_both_original_causes | Actual JSON, original and final-publication causes survive |

Every injection has healthy-before and recovery controls in fresh directories,
collected before potentially failing fault assertions. Counters distinguish the
collector hash read from the handler reread and check no forbidden parsing.
Actual exception types and outcome fields are checked structurally, without
searching serialized messages. Changed-channel injection mutates only its owned
synthetic fixture after the recorded hash; no original execution proof is edited.

CLI requires --candidate/--candidate-sha256, --outcome-source/--outcome-sha256,
--helpers-source/--helpers-sha256, and an existing --evidence-parent admitted by
the storage owner. Proposed recipe uses SWARM_HANDLER_CANDIDATE_SHA256,
SWARM_HANDLER_OUTCOME_SHA256, SWARM_HANDLER_HELPERS_SHA256,
SWARM_HANDLER_EVIDENCE_PARENT with canonical source paths. Pins, source bytes,
trials, captured candidate stdout, unittest output and summary are retained;
each source pin is checked independently afterwards. No watchdog or retry is
added. Eight is this additional inventory, not a quota or inherited test proof.
Separate source review and execution authorization remain required. This suite
qualifies no Admin policy, atomic writer, confinement or installation.
