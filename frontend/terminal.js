const tr_frontend_terminal_js = source => globalThis.SwarmI18n?.t(source) ?? source;
import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import {SessionConsole,sessionProgress} from '../web/session-console.js';

const $=id=>document.getElementById(id);
const params=new URLSearchParams(location.search),agent=params.get('agent'),work=params.get('work');
const client=crypto.randomUUID();
let csrf='',lease='',seq=0,cursor=0,status='',mode='',closed=false,polling=false,writing=false,queue=[],lastClaim=0,replaying=true,resizeTimer,resizePending=false;
const message=(text,error=false)=>{$('terminal-notice').textContent=text;$('terminal-notice').dataset.error=String(error)};
const theme=value=>{document.documentElement.dataset.theme=value==='sombre'?'sombre':'etat'};
theme(params.get('theme'));
if(!window.SecurityUtils)throw Error(tr_frontend_terminal_js('Bibliothèque de sécurité indisponible'));
const palette=()=>{
 const css=getComputedStyle(document.documentElement),v=name=>css.getPropertyValue('--wattson-'+name).trim();
 const colors=[v('texte-discret'),v('alerte-encre'),v('succes-encre'),v('attention-encre'),v('lien'),v('info-encre'),v('succes-encre'),v('texte')];
 const t={background:v('champ'),foreground:v('texte'),cursor:v('lien'),cursorAccent:v('champ'),selectionBackground:v('info-fond'),selectionForeground:v('info-encre')};
 ['black','red','green','yellow','blue','magenta','cyan','white'].forEach((key,i)=>{t[key]=colors[i];t['bright'+key[0].toUpperCase()+key.slice(1)]=colors[i]});return t;
};
const terminal=new Terminal({cols:96,rows:28,scrollback:5000,fontSize:14,fontFamily:'monospace',cursorBlink:true,disableStdin:true,screenReaderMode:true,minimumContrastRatio:4.5,theme:palette(),linkHandler:{activate(){},hover(){},leave(){}}});
const consoleView=new SessionConsole();let viewChosen=false;const decoder=new TextDecoder();
function showConsole(on){$('session-console-wrap').hidden=!on;$('terminal-screen').hidden=on;$('console-toggle').setAttribute('aria-pressed',String(on));$('terminal-toggle').setAttribute('aria-pressed',String(!on));if(on)void consoleView.initialize();else terminal.focus()}
const fit=new FitAddon();terminal.loadAddon(fit);terminal.open($('terminal-screen'));
// Disallow synthetic DOM input. Provider-generated terminal protocol responses
// are emitted by xterm itself and only relayed after replay, by the lease holder.
for(const name of ['keydown','keypress','input','paste','compositionstart','compositionupdate','compositionend'])$('terminal-screen').addEventListener(name,e=>{if(!e.isTrusted){e.preventDefault();e.stopImmediatePropagation()}},true);
async function api(path,data){const options={signal:AbortSignal.timeout(5000),headers:{}};if(data!==undefined){options.method='POST';options.headers={'Content-Type':'application/json','X-Swarm-CSRF':csrf};options.body=JSON.stringify(data)}const response=await fetch(path,options);const result=await response.json();if(!response.ok)throw Error(result.error||tr_frontend_terminal_js('Session indisponible'));return result}
const control=(kind,extra={})=>api('/api/v1/terminal/control',{agent,work,client,lease,kind,...extra});
const writable=()=>['terminal','dialogue'].includes(mode)&&status==='running';
function lock(){lease='';queue=[];terminal.options.disableStdin=true;$('terminal-claim').disabled=!writable();$('terminal-claim').textContent=tr_frontend_terminal_js('Prendre la saisie')}
function state(){
 const label={queued:tr_frontend_terminal_js('Démarrage en attente'),starting:tr_frontend_terminal_js('Démarrage'),running:tr_frontend_terminal_js('Session active'),stopping:tr_frontend_terminal_js('Arrêt demandé'),completed:tr_frontend_terminal_js('Session terminée'),failed:tr_frontend_terminal_js('Session en échec'),interrupted:tr_frontend_terminal_js('Session interrompue')}[status]||status;
 $('terminal-status').dataset.state=status;
 $('terminal-status').textContent=label+' · '+(lease?tr_frontend_terminal_js('Saisie active'):['terminal','dialogue'].includes(mode)?tr_frontend_terminal_js('Lecture seule'):tr_frontend_terminal_js('Agent automatisé · suivi en lecture seule'));
 $('terminal-claim').disabled=!writable()||!!lease; $('terminal-stop').disabled=!['starting','running'].includes(status);
 if(!writable())lock();
}
async function claim(){
 const r=await control('claim');lastClaim=Date.now();
 if(r.busy){lock();message(tr_frontend_terminal_js('Une autre vue détient la saisie. Fermez sa vue ou attendez 20 secondes après sa déconnexion.'));return}
 lease=r.lease;seq=r.seq;terminal.options.disableStdin=false;$('terminal-claim').textContent=tr_frontend_terminal_js('Saisie active');state();message(tr_frontend_terminal_js('Écrivez directement dans le terminal. Fermer la vue laisse l’agent continuer.'));await resize();terminal.focus();
}
async function resize(){
 if(!lease||replaying||writing||resizePending)return;
 const size=fit.proposeDimensions();if(!size)return;
 const cols=Math.max(20,Math.min(300,size.cols)),rows=Math.max(5,Math.min(120,size.rows));
 if(cols===terminal.cols&&rows===terminal.rows)return;
 resizePending=true;
 try{await control('resize',{cols,rows});terminal.resize(cols,rows)}finally{resizePending=false}
}
async function drain(){
 if(writing||!lease||closed||!queue.length)return;writing=true;
 try{while(queue.length&&lease&&!closed){const data=queue.shift();seq++;await control('input',{seq,data:btoa(String.fromCharCode(...data))})}}
 catch(e){lock();message(tr_frontend_terminal_js('Réception de la dernière saisie non confirmée. Vérifiez ce qui apparaît ; aucune retransmission automatique. ')+e.message,true)}
 finally{writing=false;state()}
}
terminal.onData(data=>{
 if(!lease||replaying||closed)return;
 const bytes=new TextEncoder().encode(data);
 if(bytes.length>4096||queue.reduce((n,b)=>n+b.length,0)+bytes.length>8192){message(tr_frontend_terminal_js('Saisie trop volumineuse : au plus 4096 octets par collage. Rien de ce collage n’a été envoyé.'),true);return}
 queue.push(bytes);void drain();
});
async function poll(){
 if(closed||polling)return;polling=true;
 try{
  const r=await api('/api/v1/terminal?'+new URLSearchParams({agent,work,after:cursor}));mode=r.mode;status=r.status;
  $('terminal-toggle').textContent=['terminal','dialogue'].includes(mode)?tr_frontend_terminal_js('Répondre à l’agent'):tr_frontend_terminal_js('Sortie originale');
  $('terminal-claim').hidden=!['terminal','dialogue'].includes(mode);
  if(!viewChosen){viewChosen=true;showConsole(true)}
  const p=r.progress||{},h=r.health||{};
  $('session-history-note').textContent=['terminal','dialogue'].includes(mode)?tr_frontend_terminal_js('Historique de la session conservé dans la limite disponible.'):r.capture?tr_frontend_terminal_js('Messages, actions et résultats capturés. Les sorties longues peuvent être abrégées.'):tr_frontend_terminal_js('Messages et actions disponibles selon la version de lancement. Le contenu détaillé des réponses d’outils n’a pas été enregistré : activez « Capture détaillée des sorties » au prochain lancement pour le conserver.');
  $('session-current').textContent=p.action?tr_frontend_terminal_js('Dernière action : ')+p.action:tr_frontend_terminal_js('Aucune action détaillée reçue pour le moment.');
  if(/hashlib|sha256/.test(p.detail||'')&&/source-snapshot|stable|diff/.test(p.detail||''))$('session-current').textContent=tr_frontend_terminal_js('Dernière action : vérifier si les fichiers ont changé pendant les contrôles.');
  else if(/swarm.*work show/.test(p.detail||''))$('session-current').textContent=tr_frontend_terminal_js('Dernière action : consulter les tâches et leurs validations dans Swarm.');
  $('session-tools').textContent=mode==='terminal'?tr_frontend_terminal_js('Appels d’outils non mesurés dans ce terminal'):(p.tool_calls||0)+tr_frontend_terminal_js(' appels d’outils · ')+(p.tool_results||0)+tr_frontend_terminal_js(' réponses reçues');
  $('session-cost').textContent=typeof r.usage?.provider_reported_cost_usd==='number'?tr_frontend_terminal_js('Coût transmis : ')+r.usage.provider_reported_cost_usd.toFixed(2)+' USD':tr_frontend_terminal_js('Coût inconnu : aucun montant transmis par le fournisseur');
  $('session-progress').textContent=sessionProgress(r);
  for(const event of r.events){if(event.cols)terminal.resize(event.cols,event.rows);if(event.data){const bytes=Uint8Array.from(atob(event.data),c=>c.charCodeAt(0));consoleView.append(decoder.decode(bytes,{stream:true}).replace(/\r\n/g,'\n').replace(/\x1b\[[0-?]*[ -/]*[@-~]/g,''));await new Promise(resolve=>terminal.write(bytes,resolve))}cursor=event.seq}
  const publicMessages=consoleView.text.split('\n').filter(line=>/^\[[^\]]+\] MESSAGE · /.test(line));
  $('session-message').hidden=!publicMessages.length;
  if(publicMessages.length){const latest=publicMessages.at(-1).replace(/^\[[^\]]+\] MESSAGE · /,'');$('session-message').textContent=tr_frontend_terminal_js('L’agent explique : ')+latest.slice(0,500)+(latest.length>500?tr_frontend_terminal_js('… (suite dans le journal)'):'')}
  if(r.events.length<16){const first=replaying;replaying=false;if(first)message(['terminal','dialogue'].includes(mode)?tr_frontend_terminal_js('Historique chargé. Prenez la saisie pour répondre à l’agent.'):tr_frontend_terminal_js('Messages et actions de cette tentative en lecture seule. La fin de l’agent ne vaut pas validation de son résultat.'));}
  if(r.desired==='stop'&&['queued','starting','running','stopping'].includes(status))status='stopping';state();
  if(lease&&Date.now()-lastClaim>5000&&!writing){const renewal=await control('claim');lastClaim=Date.now();if(renewal.lease!==lease){lock();message(tr_frontend_terminal_js('Votre saisie a expiré. Reprenez-la explicitement avant de continuer.'),true)}}
 }catch(e){lock();message(tr_frontend_terminal_js('Connexion interrompue : saisie suspendue. ')+e.message,true)}
 finally{polling=false;if(!closed)setTimeout(poll,replaying?30:350)}
}
const click=(id,fn)=>$(id).addEventListener('click',e=>{if(e.isTrusted)Promise.resolve(fn()).catch(error=>{lock();message(error.message,true)})});
click('console-toggle',()=>showConsole(true));
click('terminal-toggle',()=>showConsole(false));
click('console-follow',()=>{consoleView.follow=!consoleView.follow;$('console-follow').setAttribute('aria-pressed',String(consoleView.follow));$('console-follow').textContent=consoleView.follow?tr_frontend_terminal_js('Suivi automatique activé'):tr_frontend_terminal_js('Suivi automatique en pause')});
click('terminal-claim',async()=>{showConsole(false);await claim()});
click('terminal-stop',()=>{$('terminal-stop-review').hidden=false;$('terminal-stop-confirm').focus()});
click('terminal-stop-cancel',()=>{$('terminal-stop-review').hidden=true;terminal.focus()});
click('terminal-stop-confirm',async()=>{
 $('terminal-stop-confirm').disabled=true;
 try{const current=await api('/api/v1/snapshot?'+new URLSearchParams({work}));await api('/api/v1/action',{kind:'stop',agent,work,event_id:crypto.randomUUID(),expected_revision:current.work.revision});lock();status='stopping';state();$('terminal-stop-review').hidden=true;message(tr_frontend_terminal_js('Arrêt demandé. Le superviseur doit encore confirmer la fin du processus.'))}
 finally{$('terminal-stop-confirm').disabled=false}
});
window.addEventListener('message',e=>{if(e.origin!==location.origin||e.source!==parent)return;if(e.data?.kind==='swarm-terminal-theme'){theme(e.data.theme);terminal.options.theme=palette();consoleView.theme()}});
window.addEventListener('keydown',e=>{if(e.isTrusted&&e.key==='Escape'){if(document.querySelector('dialog[open], #session-console .find-widget.visible'))return;e.preventDefault();e.stopImmediatePropagation();parent.postMessage({kind:'swarm-terminal-close'},location.origin)}},true);
window.addEventListener('pagehide',()=>{closed=true;if(lease)fetch('/api/v1/terminal/control',{method:'POST',keepalive:true,signal:AbortSignal.timeout(3000),headers:{'Content-Type':'application/json','X-Swarm-CSRF':csrf},body:JSON.stringify({agent,work,client,lease,kind:'release'})}).catch(()=>{});consoleView.dispose();terminal.dispose()});
new ResizeObserver(()=>{clearTimeout(resizeTimer);resizeTimer=setTimeout(()=>resize().catch(e=>message(e.message,true)),200)}).observe($('terminal-screen'));
try{csrf=(await api('/api/v1/session')).csrf;await poll()}catch(e){message(e.message,true)}
