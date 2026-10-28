from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit,atomic
p=Path('rust/crates/swarm-engine/src/agent_completion_journal.rs');s=p.read_text();start=s.index('    let facts: WaitFacts =',s.index('pub(crate) fn observe_terminal'));end=s.index('    let current = crate::read_agent(db, id)',start);body=s[start:end]
s=s[:start]+'    let facts = verify_original_facts(&journal)?;\n'+s[end:]
marker='/// Verify actual terminal native observations in the caller transaction without'
helper='''/// Validate original kernel reap, workspace release, private gate and native
/// provenance independently of the current SQL terminal projection. Recovery uses
/// the same proof BEFORE any settlement; observation additionally checks terminal
/// rows. Malformed/unknown facts refuse, even if the original owner is still live.
fn verify_original_facts(journal: &Journal) -> Result<WaitFacts, RootStoreError> {
'''+body+'    Ok(facts)\n}\n'
assert s.count(marker)==1;s=s.replace(marker,helper+marker)
s=s.replace('    match departed(&journal.expected) {','    verify_original_facts(&journal)?;\n    match departed(&journal.expected) {')
atomic(p,s)
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1234 revue avant recette : présence d'un journal ne prouve pas reap/release/gate.\nExtraire verify_original_facts de l'observateur existant et le réutiliser AVANT\nla mutation de récupération. Le chemin manuel conserve son contrat ; ajouter\nmutants kernel_reaped=false, incomplete_output et original invalide à la recette.\n''')
