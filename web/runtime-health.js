'use strict';
const RuntimeHealthPanel = {
 value:null, pending:false, signature:'',
 async check(){
  if(this.pending)return;this.pending=true;
  try{this.value=await api('/api/v1/runtime-health');this.render(this.value)}
  catch{this.value={state:'unknown',message:'État du stockage non vérifié.',next_step:'Vérifiez la connexion au serveur puis réessayez.',volumes:[]};this.render(this.value)}
  finally{this.pending=false}
 },
 // Comparison uses the build's embedded VCS revision against this checkout's
 // local git HEAD — both already resolved server-side, no network call.
 renderVersion(v){
  const el=$('server-version');if(!el)return;
  if(!v||!v.available){el.textContent=SwarmI18n.t('Version : état inconnu');el.title=SwarmI18n.t('Identifiant de build indisponible sur ce binaire.');return}
  const rev=v.revision.slice(0,12)+(v.modified?'*':'');
  if(!v.compared){el.textContent=SwarmI18n.t('Version : ')+rev;el.title=SwarmI18n.t('Comparaison indisponible : dépôt local non détecté.');return}
  el.textContent=SwarmI18n.t('Version : ')+rev+(v.current?' · '+SwarmI18n.t('à jour'):' · '+SwarmI18n.t('différente de la copie locale'));
  el.title=v.current?SwarmI18n.t('Identique à la copie locale du dépôt.'):SwarmI18n.t('Le binaire lancé ne correspond pas au HEAD local — redémarrage probable requis.');
 },
 render(h){
  this.renderVersion(h.version);
  const host=$('runtime-health');if(!host)return;
  const key=JSON.stringify([h.state,h.message,h.next_step]);if(key===this.signature)return;this.signature=key;
  host.hidden=h.state==='ready';host.replaceChildren();if(host.hidden)return;
  host.className='notice '+(h.state==='blocked'?'alert':'attention');
  host.append(node('h2',SwarmI18n.engine(h.message)),node('p',SwarmI18n.engine(h.next_step)));
  const b=node('button',SwarmI18n.t('Diagnostic du stockage'));b.type='button';b.onclick=()=>this.open();host.append(b);
 },
 open(){
  openModal(SwarmI18n.t('Diagnostic du stockage'),SwarmI18n.t('Ce contrôle local ne lance aucune IA. Les missions, rapports et copies de travail sont conservés.'),{action:'help'});
  $('confirm').hidden=true;$('cancel').textContent=SwarmI18n.t('Fermer');$('modal-help-toggle').hidden=true;
  const host=$('modal-fields'),context=modalContext;
  const render=()=>{
   if(modalContext!==context)return;const h=this.value;host.replaceChildren();const content=node('section',undefined,'mission-help');host.append(content);
   if(h){content.append(node('p',SwarmI18n.engine(h.message)),node('p',SwarmI18n.engine(h.next_step)));
    for(const v of h.volumes||[]){const box=node('section',undefined,'mission-task');box.append(node('h3',SwarmI18n.t(v.kind==='store'?'Stockage de Swarm':'Fichiers temporaires')),node('pre',v.path),node('p',v.state==='unknown'?SwarmI18n.t('Mesure indisponible'):Math.floor(v.available_bytes/1048576)+' MiB · '+SwarmI18n.t('espace disponible')));content.append(box)}
   }
   const b=node('button',SwarmI18n.t('Vérifier à nouveau'));b.type='button';b.onclick=async()=>{b.disabled=true;await this.check();render();host.querySelector('button')?.focus();if(this.value?.state==='ready')returnFocus=$('refresh')};const actions=node('div',undefined,'toolbar');actions.append(b);content.append(actions);
  };render();
 },
 start(){this.check();setInterval(()=>{if(!document.hidden)this.check()},5000)}
};
document.addEventListener('DOMContentLoaded',()=>RuntimeHealthPanel.start());
