'use strict';
/** Read-only observation of real PAEA effect surfaces. Consumes only the two
 * public guarded reads: GET /api/v1/paea/git-call (startup capability, no
 * consumption) and GET /api/v1/paea/reconcile-effects (recomputed journal
 * reconciliation). No POST, no one-shot call consumption, no authority, no
 * repair: `verified=true` stays recomputed correspondence, never authenticity
 * or acceptance. Any failure keeps the exact refusal code visible.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Read one guarded GET endpoint; refuse to hide a refusal code. */
  async function read(path){const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin'});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Public capability entries shown as-is; nothing is added or reinterpreted. */
  function capability(view){
    if(!view.bound){status('gitcall-status','empty','Aucun appel Git préparé n’est lié au démarrage du service. La préparation deux phases (CLI ou enrôlement) reste la voie réelle.');return}
    const cap=view.capability;
    if(!cap||typeof cap!=='object'){status('gitcall-status','error','Capacité publique absente d’une source liée : état incohérent signalé, aucun appel effectué.');return}
    const labels={scope:'Portée',reason:'Origine',call:'Appel',service:'Service',operation:'Opération',root_digest:'Empreinte de racine',action_digest:'Empreinte d’action'};
    const dl=$('gitcall-capability');dl.replaceChildren();
    for(const [key,value] of Object.entries(cap)){
      if(!(key in labels))continue;
      const dt=document.createElement('dt'),dd=document.createElement('dd');
      dt.textContent=t(labels[key]);dd.textContent=String(value);dl.append(dt,dd);
    }
    $('gitcall-detail').hidden=false;
    status('gitcall-status','success','Un appel Git préparé est lié : capacité publique affichée. La consommation reste un appel one-shot gardé, jamais déclenché depuis cette page.');
  }
  /** Explicit journal state per domain; findings list exact refusal codes. */
  function journal(domain,heading){
    const stateBox=$(domain+'-state'),list=$(domain+'-findings');list.replaceChildren();
    if(heading.state==='uninitialized'){stateBox.textContent=t('Journal non initialisé : aucun effet rapproché (état explicite, pas un succès vide).');return}
    if(heading.state==='corrupt'){
      stateBox.textContent=t('Schéma altéré ou incomplet : aucune ligne rapprochée, aucune réparation tentée.');
      const ul=list;for(const code of heading.codes??[]){const li=document.createElement('li');li.textContent=code;ul.append(li)}
      return;
    }
    if(heading.state==='consistent'){
      const n=heading.effectCount??heading.effect_count??0;
      const findings=heading.findings??[];
      const ok=findings.filter(f=>f.verified).length,ko=findings.length-ok;
      stateBox.textContent=t('{n} effet(s) durable(s) rapproché(s) : {ok} correspondance(s) recalculée(s), {ko} écart(s) signalé(s).',{n,ok,ko});
      if(!findings.length){const li=document.createElement('li');li.textContent=t('Aucun effet durable enregistré dans ce journal.');list.append(li);return}
      for(const f of findings){
        const li=document.createElement('li');
        li.textContent=t('intent {intent} · effet {digest} · {verdict} {codes}',{intent:f.intent_id,digest:f.action_digest,verdict:f.verified?t('correspondance recalculée'):t('écart'),codes:(f.refusals??[]).join(' ')});
        list.append(li);
      }
      return;
    }
    stateBox.textContent=t('État de journal inconnu : rapport hors contrat, aucun verdict affiché.');
  }
  function report(view){
    if(view.schema_version!==1){status('reconcile-status','error','Rapport hors contrat (version de schéma inattendue) ; original conservé dans le rapport technique.');$('reconcile-report').hidden=false;$('reconcile-report-raw').textContent=JSON.stringify(view,null,2);return}
    $('reconcile-report').hidden=false;$('reconcile-report-raw').textContent=JSON.stringify(view,null,2);
    journal('settlement',view.settlement??{});
    journal('git',view.git??{});
    status('reconcile-status','success','Rapprochement recalculé affiché pour les deux domaines réels. Correspondance recalculée n’établit pas l’authenticité ; séparation UID, ancrages externes et stockage immuable restent des prérequis ouverts.');
  }
  /** Preserve language/theme between this page and existing PAEA views. */
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  async function load(){
    status('gitcall-status','loading','Chargement…');status('reconcile-status','loading','Chargement…');
    $('gitcall-detail').hidden=true;$('reconcile-report').hidden=true;
    try{capability(await read('/api/v1/paea/git-call'))}catch(error){status('gitcall-status','error',t('Refus : {code}',{code:error.message}))}
    try{report(await read('/api/v1/paea/reconcile-effects'))}catch(error){status('reconcile-status','error',t('Refus : {code}',{code:error.message}))}
  }
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  $('paea-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';navigation()});
  navigation();load();
})();
