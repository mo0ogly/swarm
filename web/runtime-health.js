'use strict';
const VersionHistoryPanel={
 value:null,error:null,pending:null,returnFocus:null,
 t(source){return globalThis.SwarmI18n?.t(source)??source},
 short(commit){return typeof commit==='string'&&/^[0-9a-f]{40}$/.test(commit)?commit.slice(0,12):this.t('inconnu')},
 normalize(raw){
  if(!raw||typeof raw!=='object')return null;
  if(raw.binary)return {binary:raw.binary,source:raw.source||{},history:raw.history||{available:false,releases:[]}};
  const commit=raw.available&&typeof raw.revision==='string'?raw.revision:null;
  const sourceCommit=typeof raw.source_revision==='string'&&/^[0-9a-f]{40}$/.test(raw.source_revision)?raw.source_revision:null;
  return {binary:{version:'devel',commit,modified:raw.available?!!raw.modified:null,build_date:null,provenance:'legacy'},source:{available:!!sourceCommit,commit:sourceCommit,modified:null,compared:!!raw.compared&&!!sourceCommit,matches_binary:raw.compared&&sourceCommit?!!raw.current:null},history:{available:false,releases:[]}};
 },
 ensureDialog(){
  let dialog=document.getElementById('version-dialog');if(dialog)return dialog;
  dialog=document.createElement('dialog');dialog.id='version-dialog';dialog.setAttribute('aria-labelledby','version-dialog-title');
  const header=document.createElement('header'),title=document.createElement('h2'),close=document.createElement('button'),body=document.createElement('section'),footer=document.createElement('footer'),done=document.createElement('button');
  title.id='version-dialog-title';title.textContent=this.t('Version et nouveautés');close.type='button';close.textContent=this.t('Fermer');close.dataset.versionClose='';body.id='version-dialog-content';body.setAttribute('aria-live','polite');done.type='button';done.textContent=this.t('Fermer');done.dataset.versionClose='';header.append(title,close);footer.append(done);dialog.append(header,body,footer);document.body.append(dialog);
  dialog.addEventListener('close',()=>{const target=this.returnFocus;this.returnFocus=null;if(target?.isConnected)target.focus()});
  dialog.addEventListener('click',event=>{if(event.target.closest('[data-version-close]'))dialog.close()});
  return dialog;
 },
 safeLink(url,label){
  let parsed;try{parsed=new URL(url)}catch{return null}if(parsed.protocol!=='https:'||parsed.hostname!=='github.com'||parsed.username||parsed.password)return null;
  const link=document.createElement('a');link.href=parsed.href;link.target='_blank';link.rel='noopener noreferrer';link.textContent=label;return link;
 },
 line(term,value){const row=document.createElement('div'),dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=term;dd.textContent=value;row.append(dt,dd);return row},
 paintSummary(){
  const value=this.value,binary=value?.binary;
  for(const summary of document.querySelectorAll('[data-version-summary]')){
   if(this.error){summary.textContent=this.t('Version indisponible');continue}
   if(!value){summary.textContent=this.t('Chargement de la version…');continue}
   const modified=binary?.modified===true?'*':'';summary.textContent=(binary?.version||'devel')+' · '+this.short(binary?.commit)+modified;
  }
 },
 paintDialog(){
  const host=document.getElementById('version-dialog-content');if(!host)return;host.replaceChildren();
  if(this.error){const p=document.createElement('p');p.className='notice alert';p.textContent=this.t('Version et historique indisponibles. Réessayez lorsque le serveur répond.');host.append(p);return}
  if(!this.value){const p=document.createElement('p');p.className='notice info';p.textContent=this.t('Chargement de la version et des nouveautés…');host.append(p);return}
  const {binary,source,history}=this.value,identity=document.createElement('section'),heading=document.createElement('h3'),list=document.createElement('dl');heading.textContent=this.t('Binaire lancé');
  list.append(this.line(this.t('Version'),binary?.version||'devel'),this.line(this.t('Commit du binaire'),this.short(binary?.commit)+(binary?.modified===true?' *':'')),this.line(this.t('Date de build'),binary?.build_date||this.t('inconnue')));
  identity.append(heading,list);host.append(identity);
  const sourceBox=document.createElement('section'),sourceTitle=document.createElement('h3'),sourceText=document.createElement('p');sourceTitle.textContent=this.t('Sources locales');
  sourceText.textContent=!source?.available?this.t('Copie locale non détectée.'):!source.compared?this.t('Comparaison avec le binaire indisponible.'):source.matches_binary?this.t('Les sources et le binaire utilisent le même commit. Les modifications locales ne sont pas comparées.'):this.t('Le commit des sources diffère de celui du binaire.');sourceBox.append(sourceTitle,sourceText);host.append(sourceBox);
  const news=document.createElement('section'),newsTitle=document.createElement('h3');newsTitle.textContent=this.t('Nouveautés');news.append(newsTitle);
  if(!history?.available){const p=document.createElement('p');p.className='notice attention';p.textContent=this.t('Historique des versions indisponible dans ce binaire.');news.append(p)}
  else if(!Array.isArray(history.releases)||history.releases.length===0){const p=document.createElement('p');p.className='notice info';p.textContent=this.t('Aucune version publiée n’est encore déclarée.');news.append(p)}
  else for(const release of history.releases){const article=document.createElement('article'),h=document.createElement('h4'),summary=document.createElement('p'),links=document.createElement('p');h.textContent=release.version+' · '+release.date;summary.textContent=globalThis.SwarmI18n?.language==='en'?release.summary?.en:release.summary?.fr;const releaseLink=this.safeLink(release.release_url,this.t('Voir la version sur GitHub'));if(releaseLink)links.append(releaseLink);for(const commit of release.commits||[]){const link=this.safeLink(commit.url,this.short(commit.sha));if(link){if(links.childNodes.length)links.append(document.createTextNode(' · '));links.append(link)}}article.append(h,summary,links);news.append(article)}
  const complete=this.safeLink('https://github.com/mo0ogly/swarm/commits/main/',this.t('Voir l’historique complet des commits sur GitHub'));if(complete){const p=document.createElement('p');p.append(complete);news.append(p)}host.append(news);
 },
 async load(){
  if(this.pending)return this.pending;this.error=null;this.paintSummary();this.paintDialog();
  this.pending=fetch('/api/v1/runtime-health',{signal:AbortSignal.timeout(15000)}).then(async response=>{if(!response.ok)throw new Error(String(response.status));const health=await response.json();const value=this.normalize(health.version);if(!value)throw new Error('version contract');this.value=value;this.paintSummary();this.paintDialog();return health}).catch(error=>{this.error=error;this.paintSummary();this.paintDialog();throw error}).finally(()=>{this.pending=null});return this.pending;
 },
 open(button){this.returnFocus=button;const dialog=this.ensureDialog();this.paintDialog();dialog.showModal();if(!this.value&&!this.pending)this.load().catch(()=>{})},
 start(){for(const button of document.querySelectorAll('[data-version-open]'))button.addEventListener('click',()=>this.open(button));this.paintSummary();this.load().then(health=>RuntimeHealthPanel.render(health)).catch(()=>RuntimeHealthPanel.renderFailure())}
};
const RuntimeHealthPanel={
 value:null,signature:'',
 renderVersion(v){VersionHistoryPanel.value=VersionHistoryPanel.normalize(v);VersionHistoryPanel.error=null;VersionHistoryPanel.paintSummary();VersionHistoryPanel.paintDialog()},
 render(h){
  this.value=h;const host=document.getElementById('runtime-health');if(!host)return;const key=JSON.stringify([h.state,h.message,h.next_step]);if(key===this.signature)return;this.signature=key;
  host.hidden=h.state==='ready';host.replaceChildren();if(host.hidden)return;host.className='notice '+(h.state==='blocked'?'alert':'attention');
  const title=document.createElement('h2'),message=document.createElement('p'),button=document.createElement('button');title.textContent=globalThis.SwarmI18n?.engine(h.message)??h.message;message.textContent=globalThis.SwarmI18n?.engine(h.next_step)??h.next_step;button.type='button';button.textContent=VersionHistoryPanel.t('Diagnostic du stockage');button.addEventListener('click',()=>this.open());host.append(title,message,button);
 },
 renderFailure(){const host=document.getElementById('runtime-health');if(!host)return;host.hidden=false;host.className='notice attention';host.textContent=VersionHistoryPanel.t('État du stockage non vérifié. Vérifiez la connexion au serveur puis réessayez.')},
 async check(){try{const health=await VersionHistoryPanel.load();this.render(health)}catch{this.renderFailure()}},
 open(){
  if(typeof openModal!=='function')return this.check();openModal(VersionHistoryPanel.t('Diagnostic du stockage'),VersionHistoryPanel.t('Ce contrôle local ne lance aucune IA. Les missions, rapports et copies de travail sont conservés.'),{action:'help'});
  document.getElementById('confirm').hidden=true;document.getElementById('cancel').textContent=VersionHistoryPanel.t('Fermer');document.getElementById('modal-help-toggle').hidden=true;
  const host=document.getElementById('modal-fields'),content=document.createElement('section'),health=this.value;content.className='mission-help';host.replaceChildren(content);
  if(health){const message=document.createElement('p'),next=document.createElement('p');message.textContent=globalThis.SwarmI18n?.engine(health.message)??health.message;next.textContent=globalThis.SwarmI18n?.engine(health.next_step)??health.next_step;content.append(message,next);for(const volume of health.volumes||[]){const box=document.createElement('section'),title=document.createElement('h3'),path=document.createElement('pre'),state=document.createElement('p');box.className='mission-task';title.textContent=VersionHistoryPanel.t(volume.kind==='store'?'Stockage de Swarm':'Fichiers temporaires');path.textContent=volume.path;state.textContent=volume.state==='unknown'?VersionHistoryPanel.t('Mesure indisponible'):Math.floor(volume.available_bytes/1048576)+' MiB · '+VersionHistoryPanel.t('espace disponible');box.append(title,path,state);content.append(box)}}
  const refresh=document.createElement('button');refresh.type='button';refresh.textContent=VersionHistoryPanel.t('Vérifier à nouveau');refresh.onclick=async()=>{refresh.disabled=true;await this.check();this.open();document.getElementById('modal-fields')?.querySelector('button')?.focus()};content.append(refresh);
 },
 start(){VersionHistoryPanel.start();if(document.getElementById('runtime-health'))setInterval(()=>{if(!document.hidden)this.check()},5000)}
};
globalThis.VersionHistoryPanel=VersionHistoryPanel;
document.addEventListener('DOMContentLoaded',()=>RuntimeHealthPanel.start());
