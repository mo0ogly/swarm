const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
let editor, monaco, entries = [], follow = false, decorations, selection;
let ranges=[],current=-1;
const $=id=>document.getElementById(id),reading={comfort:true,wrap:true};
try{const stored=JSON.parse(localStorage.getItem('swarm-log:reading')||'null');for(const key of ['comfort','wrap'])if(typeof stored?.[key]==='boolean')reading[key]=stored[key]}catch{}
const roleLabel=role=>tr({error:'ERREUR',warning:'ATTENTION',success:'SUCCÈS',activity:'ACTIVITÉ',neutral:'ÉTAT'}[role]);
function controls(){
 $('journal-prev-event').disabled=current<=0;$('journal-next-event').disabled=!ranges.length||current>=ranges.length-1;
 $('journal-next-signal').disabled=!ranges.some(r=>['error','warning'].includes(r.role));
 $('journal-event').disabled=!ranges.length;$('journal-event').value=String(current);
 $('journal-reading-position').textContent=tr('{current} / {total} événements',{current:current<0?0:current+1,total:ranges.length});
 $('journal-comfort').setAttribute('aria-pressed',String(reading.comfort));$('journal-wrap').setAttribute('aria-pressed',String(reading.wrap));
}
function selectEvent(index,move=false){
 if(index<0||index>=ranges.length)return;current=index;controls();
 selection?.set([{range:new monaco.Range(ranges[index].start,1,ranges[index].start,1),options:{isWholeLine:true,linesDecorationsClassName:'journal-selected-marker'}}]);
 if(move){follow=false;parent.postMessage({type:'swarm-log-browse'},location.origin);editor.setPosition({lineNumber:ranges[index].start,column:1});editor.revealLineInCenter(ranges[index].start)}
}
function readingOptions(){return {fontSize:reading.comfort?15:13,lineHeight:reading.comfort?25:20,wordWrap:reading.wrap?'on':'off',padding:{top:16,bottom:16}}}
function changeReading(key){reading[key]=!reading[key];try{localStorage.setItem('swarm-log:reading',JSON.stringify(reading))}catch{}editor?.updateOptions(readingOptions());controls()}
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
  const rows=[], marks=[],nextRanges=[],previousKey=ranges[current]?.key;
  for (const entry of entries) {
    const role=tone(entry.kind || ''), start=rows.length+1;
    const label=roleLabel(role);
    const at = new Date(entry.at), time=Number.isNaN(at.getTime())?entry.at:at.toLocaleString(globalThis.SwarmI18n?.locale || 'fr-FR');
    const source=entry.task_id || entry.agent_id || entry.origin || '';
    const prefix='['+time+'] '+label+' · '+entry.kind+' · #'+(entry.seq ?? entry.rank)+(source?' · '+source:'');
    rows.push(prefix);
    if(entry.label)rows.push(entry.label);
    let body=String(entry.message || '');
    // JSON is formatted for reading, with all fields retained. Never interpret HTML or links.
    try {const parsed=JSON.parse(body);if(parsed && typeof parsed==='object')body=JSON.stringify(parsed,null,2);} catch {}
    rows.push(...body.split('\n'), '');
    nextRanges.push({start,end:rows.length-1,role,key:JSON.stringify([entry.at,entry.seq??entry.rank,entry.kind,source]),label:'#'+(entry.seq??entry.rank)+' · '+label+' · '+entry.kind+(source?' · '+source:'')});
    marks.push({range:new monaco.Range(start,1,start,prefix.length+1),options:{inlineClassName:'journal-level-'+role}});
    marks.push({range:new monaco.Range(start,1,start,time.length+3),options:{inlineClassName:'journal-timestamp'}});
    if(role==='error'||role==='warning')marks.push({range:new monaco.Range(start,1,rows.length-1,1),options:{isWholeLine:true,className:'journal-row-'+role}});
  }
  const text=rows.join('\n') || tr('Aucune ligne correspondante.');
  if(editor.getValue()===text){controls();return}
  ranges=nextRanges;current=ranges.findIndex(r=>r.key===previousKey);if(current<0&&ranges.length)current=0;
  const options=document.createDocumentFragment();for(const [index,r]of ranges.entries()){const option=document.createElement('option');option.value=String(index);option.textContent=r.label;options.append(option)}$('journal-event').replaceChildren(options);
  const state=editor.saveViewState();editor.setValue(text);decorations.set(marks);
  if(follow)editor.revealLine(editor.getModel().getLineCount());else editor.restoreViewState(state);
  if(current>=0)selectEvent(current);else{selection?.clear();controls()}
}
function theme(value) {
  document.documentElement.dataset.theme=value;
  if(!monaco)return;
  const css=getComputedStyle(document.documentElement),v=name=>css.getPropertyValue('--wattson-'+name).trim();
  monaco.editor.defineTheme('swarm-journal',{base:value==='sombre'?'vs-dark':'vs',inherit:true,rules:[{token:'journal.key',foreground:v('lien').slice(1)},{token:'journal.number',foreground:v('accent-encre').slice(1)},{token:'journal.literal',foreground:v('accent-encre').slice(1)},{token:'journal.heading',foreground:v('titre').slice(1),fontStyle:'bold'},{token:'journal.code',foreground:v('lien').slice(1)}],colors:{'editor.background':v('champ'),'editor.foreground':v('texte'),'editorLineNumber.foreground':v('texte-discret'),'editorGutter.background':v('carte-appuyee'),'editor.selectionBackground':v('info-fond'),'editorWidget.background':v('carte'),'editorWidget.foreground':v('texte'),'input.background':v('champ'),'input.foreground':v('texte'),'focusBorder':v('lien')}});
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
  monaco.languages.register({id:'swarm-journal'});
  monaco.languages.setMonarchTokensProvider('swarm-journal',{tokenizer:{root:[[/"(?:[^"\\]|\\.)*"(?=\s*:)/,'journal.key'],[/"(?:[^"\\]|\\.)*"/,''],[/\b(?:true|false|null)\b/,'journal.literal'],[/-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b/,'journal.number'],[/^\s*#{1,6}\s+.*$/,'journal.heading'],[/`[^`]+`/,'journal.code']]}});
  // render() owns layout through the guarded observer below. Monaco's automatic
  // observer would still measure wrapping ranges while this iframe is hidden.
  editor=monaco.editor.create(document.getElementById('journal-editor'),{value:'',language:'swarm-journal',theme:'swarm-journal',readOnly:true,domReadOnly:true,automaticLayout:false,minimap:{enabled:false},...readingOptions(),lineNumbers:'on',scrollBeyondLastLine:false,renderLineHighlight:'none',stickyScroll:{enabled:false},accessibilitySupport:'on',editContext:false,links:false,ariaLabel:tr('Console des journaux, lecture seule')});
  decorations=editor.createDecorationsCollection();
  selection=editor.createDecorationsCollection();
  editor.onDidChangeCursorPosition(event=>{const index=ranges.findIndex(r=>event.position.lineNumber>=r.start&&event.position.lineNumber<=r.end);if(index>=0&&index!==current)selectEvent(index)});
  $('journal-prev-event').addEventListener('click',()=>selectEvent(current-1,true));$('journal-next-event').addEventListener('click',()=>selectEvent(current+1,true));
  $('journal-event').addEventListener('change',()=>selectEvent(Number($('journal-event').value),true));
  $('journal-next-signal').addEventListener('click',()=>{const next=ranges.findIndex((r,index)=>index>current&&['error','warning'].includes(r.role));selectEvent(next>=0?next:ranges.findIndex(r=>['error','warning'].includes(r.role)),true)});
  $('journal-comfort').addEventListener('click',()=>changeReading('comfort'));$('journal-wrap').addEventListener('click',()=>changeReading('wrap'));controls();
  const observer=new ResizeObserver(()=>render());observer.observe(document.getElementById('journal-editor'));
  addEventListener('pagehide',()=>observer.disconnect(),{once:true});render();parent.postMessage({type:'swarm-log-ready',monaco:true},location.origin);
} catch(error) {parent.postMessage({type:'swarm-log-ready',monaco:false,diagnostic:String(error?.message||error)},location.origin);}
