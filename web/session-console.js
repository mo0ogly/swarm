// Read-only log viewer. No terminal input or provider text is executed here.
// Decode captured tool results for reading; never execute provider content.
export function readableSessionText(text){
 return text.split('\n').map(line=>{
  const match=line.match(/^(\S+) · output · (.*)$/);if(!match)return line;
  let event;try{event=JSON.parse(match[2])}catch{return match[1]+' · warning · Détail capturé incomplet ; consulter la sortie originale.'}
  const results=[];
  if(event.type==='item.completed'&&event.item?.type==='command_execution'){
   results.push((event.item.exit_code===0?'Commande réussie':'Commande terminée (code '+event.item.exit_code+')')+'\n'+(event.item.aggregated_output||'Aucune sortie textuelle.'));
  }
  if(event.type==='user')for(const block of event.message?.content||[]){
   if(block.type!=='tool_result')continue;
   const value=typeof block.content==='string'?block.content:Array.isArray(block.content)?block.content.filter(x=>x.type==='text').map(x=>x.text).join('\n'):'';
   results.push((block.is_error?'Échec de l’outil':'Réponse de l’outil')+'\n'+(value||'Aucune sortie textuelle.'));
  }
  return results.length?results.map(value=>match[1]+' · result · '+value).join('\n'):null;
 }).filter(line=>line!==null).join('\n');
}
export class SessionConsole {
 constructor(){this.text='';this.follow=true;this.host=document.getElementById('session-console');this.fallback=document.getElementById('session-console-fallback');this.loading=null;this.disposed=false}
 async initialize(){
  if(this.loading)return this.loading;
  this.loading=(async()=>{try{
   const css=document.createElement('link');css.rel='stylesheet';css.href='/lib/monaco/editor.css';document.head.append(css);
   globalThis.MonacoEnvironment={getWorker:()=>new Worker('/lib/monaco/worker.js',{type:'module'})};
   const path='/lib/monaco/editor.js';this.monaco=await import(path);if(this.disposed)return;
   this.theme();this.editor=this.monaco.editor.create(this.host,{value:this.text,language:'plaintext',theme:'swarm-session',readOnly:true,domReadOnly:true,automaticLayout:true,minimap:{enabled:false},wordWrap:'on',fontSize:14,lineNumbers:'on',scrollBeyondLastLine:false,renderLineHighlight:'none',stickyScroll:{enabled:false},accessibilitySupport:'on',editContext:false,ariaLabel:'Journal de la session, lecture seule',links:false});
   this.decorations=this.editor.createDecorationsCollection();this.fallback.hidden=true;this.paint();
   document.getElementById('console-mode-label').textContent='Journal de l’agent · lecture seule · Ctrl+F pour rechercher';
  }catch{this.host.hidden=true;this.fallback.hidden=false;document.getElementById('console-mode-label').textContent='Journal en affichage simplifié · contenu conservé'}})();return this.loading;
 }
 append(text){
  // Keep a bounded view; original history stays on the server / in the terminal.
  text=readableSessionText(text).replace(/^(\d{4}-\d{2}-\d{2}T[^ ]+) · (\w+) · /gm,(_,at,kind)=>{const time=new Date(at);return '['+(Number.isNaN(time.getTime())?at:time.toLocaleTimeString('fr-FR'))+'] '+({message:'MESSAGE',result:'RÉSULTAT',warning:'ATTENTION',activity:'ACTIVITÉ',command:'DÉMARRAGE',lifecycle:'ÉTAT',limits:'LIMITES',error:'ERREUR',stderr:'ERREUR'}[kind]||kind.toUpperCase())+' · '});
  this.text=(this.text+text).slice(-250000);this.fallback.textContent=this.text;
  if(this.editor){const state=this.editor.saveViewState();this.editor.setValue(this.text);this.paint();if(this.follow)this.editor.revealLine(this.editor.getModel().getLineCount());else this.editor.restoreViewState(state)}
 }
 paint(){
  if(!this.decorations)return;
  const m=this.monaco,rows=this.text.split('\n'),decorations=[];
  let latestAction=-1;
  rows.forEach((line,index)=>{if(/\] (MESSAGE|ACTIVITÉ|DÉMARRAGE) · /.test(line)&&!line.includes("Résultat d'outil reçu"))latestAction=index});
  rows.forEach((line,index)=>{
   const match=line.match(/^(\[[^\]]+\]) ([^·]+) · (.*)$/);if(!match)return;
   const type=match[2].trim(),body=match[3];
   const role=/ERREUR|FAILED/.test(type)?'error':/LIMITES|ATTENTION|WARNING|MONITORING|OUTPUT-LIMIT/.test(type)?'warning':/ACTIVITÉ|DÉMARRAGE/.test(type)&&!body.includes("Résultat d'outil reçu")?'action':type==='MESSAGE'?'message':type==='RÉSULTAT'?'result':'lifecycle';
   const add=(start,end,cls)=>decorations.push({range:new m.Range(index+1,start,index+1,end),options:{inlineClassName:cls,inlineClassNameAffectsLetterSpacing:true}});
   add(1,match[1].length+1,'console-time');
   add(match[1].length+2,match[1].length+2+match[2].length,'console-type-'+role);
   const start=line.indexOf(' · ')+4;
   add(start,line.length+1,'console-message'+(index===latestAction?' console-message-current':''));
   if(role==='error'||role==='warning')add(start,line.length+1,'console-message-'+role);
   if(index===latestAction)decorations.push({range:new m.Range(index+1,1,index+1,1),options:{isWholeLine:true,className:'console-current-line'}});
  });this.decorations.set(decorations);
 }

 theme(){
  if(!this.monaco)return;const css=getComputedStyle(document.documentElement),v=r=>css.getPropertyValue('--wattson-'+r).trim();
  this.monaco.editor.defineTheme('swarm-session',{base:document.documentElement.dataset.theme==='sombre'?'vs-dark':'vs',inherit:false,rules:[{token:'',foreground:v('texte').replace(/^#/,'')}],colors:{'editor.background':v('champ'),'editor.foreground':v('texte'),'editorLineNumber.foreground':v('texte-discret'),'editorGutter.background':v('carte-appuyee'),'editor.selectionBackground':v('info-fond'),'editor.selectionForeground':v('info-encre'),'editorWidget.background':v('carte'),'editorWidget.foreground':v('texte'),'input.background':v('champ'),'input.foreground':v('texte'),'focusBorder':v('lien'),'editorCursor.foreground':v('lien')}});this.monaco.editor.setTheme('swarm-session');if(this.editor)requestAnimationFrame(()=>{if(!this.disposed){this.editor.layout();this.editor.render(true)}});
 }
 dispose(){this.disposed=true;this.editor?.getModel()?.dispose();this.editor?.dispose()}
}
export function sessionProgress(r){
 const p=r.progress||{},h=r.health||{};
 if(r.status==='completed')return 'L’exécution est terminée : le rapport reste à examiner';
 if(['failed','interrupted'].includes(r.status))return 'L’exécution s’est arrêtée : examinez son motif et le rapport éventuel';
 if(r.desired==='stop'||r.status==='stopping')return 'Arrêt demandé : confirmation attendue';
 if(h.process_state?.startsWith('unknown/')||h.process_state?.endsWith('/unconfirmed'))return 'Activité de l’agent non confirmée';
 if(h.activity_state==='old')return 'Aucune activité récente : progression à vérifier';
 if(p.pending_tools>0)return 'En attente de la réponse d’un outil';
 return r.status==='running'?'En attente de la prochaine activité ou de la fin de l’agent':'Démarrage en attente de confirmation';
}
