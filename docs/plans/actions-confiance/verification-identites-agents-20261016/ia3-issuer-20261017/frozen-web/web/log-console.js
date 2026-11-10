const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
let editor, monaco, entries = [], follow = false, decorations;
function tone(kind) {
  if (/error|stderr|failed|failure|refused|blocked|guard|garde|interrupted/.test(kind)) return 'error';
  if (/warning|limit|attention|cooldown|silence/.test(kind)) return 'warning';
  if (/^(success|validation-accepted|accepted)$/.test(kind)) return 'success';
  if (/decision|planning|activity|command|message|start|relay/.test(kind)) return 'activity';
  return 'neutral';
}
function render() {
  if (!editor) return;
  // A hidden iframe has no DOM ranges for Monaco's wrapping measurement.
  // Retain incoming entries and render when the actual host becomes visible.
  const host=document.getElementById('journal-editor');
  if(host.getBoundingClientRect().width<=0||host.getBoundingClientRect().height<=0)return;
  editor.layout();
  const rows=[], marks=[];
  for (const entry of entries) {
    const role=tone(entry.kind || ''), start=rows.length+1;
    const label=tr({error:'ERREUR',warning:'ATTENTION',success:'SUCCÈS',activity:'ACTIVITÉ',neutral:'ÉTAT'}[role]);
    const at = new Date(entry.at), time=Number.isNaN(at.getTime())?entry.at:at.toLocaleString(globalThis.SwarmI18n?.locale || 'fr-FR');
    const source=entry.task_id || entry.agent_id || entry.origin || '';
    const prefix='['+time+'] '+label+' · '+entry.kind+' · #'+(entry.seq ?? entry.rank)+(source?' · '+source:'');
    rows.push(prefix);
    if(entry.label)rows.push(entry.label);
    let body=String(entry.message || '');
    // JSON is formatted for reading, with all fields retained. Never interpret HTML or links.
    try {const parsed=JSON.parse(body);if(parsed && typeof parsed==='object')body=JSON.stringify(parsed,null,2);} catch {}
    rows.push(...body.split('\n'), '');
    marks.push({range:new monaco.Range(start,1,start,prefix.length+1),options:{inlineClassName:'journal-level-'+role}});
    marks.push({range:new monaco.Range(start,1,start,time.length+3),options:{inlineClassName:'journal-timestamp'}});
    if(role==='error'||role==='warning')marks.push({range:new monaco.Range(start,1,rows.length-1,1),options:{isWholeLine:true,className:'journal-row-'+role}});
  }
  const text=rows.join('\n') || tr('Aucune ligne correspondante.');
  if(editor.getValue()===text)return;
  const state=editor.saveViewState();editor.setValue(text);decorations.set(marks);
  if(follow)editor.revealLine(editor.getModel().getLineCount());else editor.restoreViewState(state);
}
function theme(value) {
  document.documentElement.dataset.theme=value;
  if(!monaco)return;
  const css=getComputedStyle(document.documentElement),v=name=>css.getPropertyValue('--wattson-'+name).trim();
  monaco.editor.defineTheme('swarm-journal',{base:value==='sombre'?'vs-dark':'vs',inherit:true,rules:[],colors:{'editor.background':v('champ'),'editor.foreground':v('texte'),'editorLineNumber.foreground':v('texte-discret'),'editorGutter.background':v('carte-appuyee'),'editor.selectionBackground':v('info-fond'),'editorWidget.background':v('carte'),'editorWidget.foreground':v('texte'),'input.background':v('champ'),'input.foreground':v('texte'),'focusBorder':v('lien')}});
  monaco.editor.setTheme('swarm-journal');
}
addEventListener('message',event=>{
  if(event.source!==parent||event.origin!==location.origin)return;
  if(event.data?.type==='swarm-log-search'){editor?.focus();editor?.getAction('actions.find')?.run();return;}
  if(event.data?.type!=='swarm-log-page'||!Array.isArray(event.data.entries))return;
  entries=event.data.entries;follow=!!event.data.follow;theme(event.data.theme==='sombre'?'sombre':'etat');render();
});
try {
  const css=document.createElement('link');css.rel='stylesheet';css.href='/lib/monaco/editor.css';document.head.append(css);
  globalThis.MonacoEnvironment={getWorker:()=>new Worker('/lib/monaco/worker.js',{type:'module'})};
  monaco=await import('/lib/monaco/editor.js');theme(document.documentElement.dataset.theme);
  editor=monaco.editor.create(document.getElementById('journal-editor'),{value:'',language:'plaintext',theme:'swarm-journal',readOnly:true,domReadOnly:true,automaticLayout:true,minimap:{enabled:false},wordWrap:'on',fontSize:14,lineNumbers:'on',scrollBeyondLastLine:false,renderLineHighlight:'none',stickyScroll:{enabled:false},accessibilitySupport:'on',editContext:false,links:false,ariaLabel:tr('Console des journaux, lecture seule')});
  decorations=editor.createDecorationsCollection();
  const observer=new ResizeObserver(()=>render());observer.observe(document.getElementById('journal-editor'));
  addEventListener('pagehide',()=>observer.disconnect(),{once:true});render();parent.postMessage({type:'swarm-log-ready',monaco:true},location.origin);
} catch(error) {parent.postMessage({type:'swarm-log-ready',monaco:false,diagnostic:String(error?.message||error)},location.origin);}
