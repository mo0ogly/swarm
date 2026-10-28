from pathlib import Path
import sys,json
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit,atomic
name='rust/crates/swarm-engine/src/root_store/mission_status_historical.rs'
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260');p=b/'originals'/name;p.parent.mkdir(parents=True,exist_ok=True);atomic(p,Path(name).read_bytes())
edit(name,'for state in ["todo","blocked","abandoned"] {','for state in ["todo","abandoned"] {')
old='''        let mut invalid=agent.clone();invalid.task_id="other".into();
        assert!(supported(&work,&[invalid]).is_err());
        supported(&work,&[agent]).unwrap();
'''
new='''        let mut blocked=work.clone();blocked.tasks.as_mut().unwrap()[0].status="blocked".into();
        supported(&blocked,std::slice::from_ref(&agent)).unwrap();
        blocked.tasks.as_mut().unwrap()[0].blocker="Livraison incomplète : evidence missing".into();
        assert!(supported(&blocked,std::slice::from_ref(&agent)).is_err());
        let mut invalid=agent.clone();invalid.task_id="other".into();
        assert!(supported(&work,&[invalid]).is_err());
        supported(&work,&[agent]).unwrap();
'''
edit(name,old,new)
edit(name,'for status in ["failed","interrupted"] {','for status in ["failed","interrupted","completed"] {')
edit(name,'for status in ["queued","starting","running","stopping","completed","unknown"] {','for status in ["queued","starting","running","stopping","unknown"] {')
edit(name,'/// Failed/interrupted owned rows remain unvalidated and need an Agent in view.','/// Failed/interrupted/completed blocked rows need an Agent in view; completed\n    /// shape alone does not supply attribution or establish validation.')
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1272 : recette1271 PASS après mise au repos publique du conducteur (stop consenti,\nsans provider tué).1269 garde le refus de cohérence concurrente original ; aucun\nretry/guard affaibli.105tests status :103PASS,2FAIL attentes anciennes refusant\nblocked/completed. Attentes alignées sur nouveau contrat : forme simple permise,\nlivraison incomplète/mauvais rôle/review/recovery restent refusés. Requalifier\nles entrées modifiées même si seules ces assertions de test changent.\n''')
