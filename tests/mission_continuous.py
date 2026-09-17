#!/usr/bin/env python3
"""Real processes, isolated DB, no model calls; browser-independent mission recipe."""
import sys,json,time,hashlib
from pathlib import Path
from autonomie_exercices import Exercice,AGENT_QUI_LIVRE
x=Exercice(sys.argv[1]);out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
checks=[]
try:
 w=x.muter(['work','create'],0,dict(title='Mission continue',objective='Deux livrables',scope='recette isolée',criteria=['preuves']))['work'];wid=w['id']
 for task,deps in [('a',[]),('b',['a'])]:x.muter(['task','add',wid],x.revision(wid),dict(id=task,title=task,deliverable='docs/'+task+'.md',criteria=['preuve'],depends=deps,next='produire'))
 docs=Path(x.racine)/'docs';docs.mkdir()
 x.fournisseur('livre',AGENT_QUI_LIVRE%str(docs/'a.md'))
 x.profil(wid,'','livre');x.demarrer_web();time.sleep(2.5)
 assert len(x.cli('agent','list',wid)['agents'])==0;checks.append('aucun départ sans autorisation locale explicite')
 x.cli('mission','start',wid)
 x.attendre(lambda:x.travail(wid)['tasks'][0]['status']=='submitted',quoi='rapport A soumis automatiquement')
 assert len(x.cli('agent','list',wid)['agents'])==1;checks.append('mission lancée une fois, navigateur absent, rapport soumis sans acceptation')
 x.cli('mission','pause',wid)
 artifact='docs/a.md';digest=hashlib.sha256((docs/'a.md').read_bytes()).hexdigest()
 evidence=dict(method_version='2',scope_id='a',artifacts={artifact:digest},domains={'tests':1},checks=[dict(id='proof',domain='tests',mandatory=True,gate='delivery',penalty=100,max_penalty=100,severity='major')],results=[dict(id='proof',status='PASS',count=0,evidence=[artifact])])
 x.muter(['gate',wid],x.revision(wid),dict(task_id='a',phase='delivery',name='preuve fixture',document=evidence))
 x.muter(['task','update',wid],x.revision(wid),dict(id='a',status='accepted',next='validé après revue de fixture'))
 time.sleep(2.5);assert len(x.cli('agent','list',wid)['agents'])==1;checks.append('acceptation pendant pause ne lance pas B')
 original=(docs/'a.md').read_text();(docs/'a.md').write_text('preuve changée')
 x.cli('mission','resume',wid);time.sleep(2.5)
 assert len(x.cli('agent','list',wid)['agents'])==1
 d=x.cli('mission','status',wid);assert d['validated']==0;assert any(t['state']=='intervention' for t in d['tasks']);checks.append('preuve périmée : B retenue et action de revalidation visible')
 x.arreter_web();(docs/'a.md').write_text(original);x.demarrer_web()
 x.attendre(lambda:len(x.cli('agent','list',wid)['agents'])==2,quoi='B part après reprise serveur sans clic')
 time.sleep(2.5);assert len(x.cli('agent','list',wid)['agents'])==2;checks.append('reprise persistée après redémarrage, B lancée exactement une fois après A validée')
 live=x.vue('/api/v1/snapshot?work='+wid)['mission'];cli=x.cli('mission','status',wid)
 assert live['validated']==cli['validated'] and live['total']==cli['total'];checks.append('diagnostic CLI/web commun')
 x.cli('mission','stop',wid);assert not x.cli('mission','status',wid)['enabled'];checks.append('arrêt de la mission persisté')
 result={'status':'PASS','checks':checks,'scope':'processus factices réels, sans IA'};print(json.dumps(result,ensure_ascii=False));(out/'recipe.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
finally:
 x.arreter_web()
