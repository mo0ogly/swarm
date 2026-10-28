from pathlib import Path
import json,sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit,atomic
p='rust/crates/swarm-engine/src/root_store/mission_status_reports.rs'
old='''        if agent.status == "completed" && !matches!(task.status.as_str(), "submitted" | "accepted")
            && !(task.status=="blocked" && matches!(&observed_review,Some(ReviewObservation::Presented(_))))
            && !(unsubmitted_pair(task, agent) && attribution.is_some()) {
'''
new='''        let completed_supported = matches!(task.status.as_str(), "submitted" | "accepted")
            || (task.status=="blocked" && matches!(&observed_review,Some(ReviewObservation::Presented(_))))
            || (unsubmitted_pair(task, agent) && attribution.is_some());
        if agent.status == "completed" && !completed_supported {
'''
edit(p,old,new)
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260')
atomic(b/'replan1277.json',json.dumps(dict(original='quality1274/clippy.stderr',cause='nonminimal_bool: completed guard combines three negated alternatives',decision='Name the disjunction completed_supported, reject completed only when false; same conditions, no allow/suppression',verified_before='105 status +44CLI tests PASS; both public recipes1276 PASS and integrity checked',next='Fresh Clippy/default profile/public recipe; retain original quality failure'),indent=2)+'\n')
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1277 :149tests ciblés PASS et deux recettes1276 PASS/intégrité ; Clippy refuse\nnonminimal_bool. Nommer la disjonction completed_supported puis négation unique,\nconditions inchangées et aucun allow. Sources/profil/recette à requalifier.\n''')
