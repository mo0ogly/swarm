const tr_web_prephase_editor_js = source => globalThis.SwarmI18n?.t(source) ?? source;
// One editor/model per document; models never leave the page as executable HTML.
export class PreparationEditor {
 constructor(onChange) { this.onChange=onChange;this.models=new Map();this.states=new Map();this.host=document.getElementById('monaco');this.plain=document.getElementById('plain-editor');this.simple=matchMedia('(max-width:767px)').matches;this.key='';this.updating=false;this.plain.addEventListener('input',e=>{if(e.isTrusted)this.onChange(this.plain.value)}); }
 async initialize() {
  if(this.simple){this.modeMessage(tr_web_prephase_editor_js('Éditeur texte — adapté aux petits écrans.'));return}
  try {
   const css=document.createElement('link');css.rel='stylesheet';css.href='/lib/monaco/editor.css';const cssReady=new Promise((resolve,reject)=>{css.onload=resolve;css.onerror=()=>reject(new Error(tr_web_prephase_editor_js('CSS indisponible')));});document.head.append(css);
   // Only the documented Monaco integration object is global.
   globalThis.MonacoEnvironment={getWorker:(_id,label)=>{const worker=new Worker(label==='json'?'/lib/monaco/json-worker.js':'/lib/monaco/worker.js',{type:'module'});worker.addEventListener('error',()=>{const text=this.value();this.simple=true;this.setText(text);this.showMode();this.modeMessage(tr_web_prephase_editor_js('Éditeur enrichi indisponible — votre texte reste éditable ici.'));});return worker}};
   let timeout;try{const loaded=await Promise.race([Promise.all([import('/lib/monaco/editor.js'),cssReady]),new Promise((_,reject)=>{timeout=setTimeout(()=>reject(new Error(tr_web_prephase_editor_js('Chargement trop long'))),15000)})]);this.monaco=loaded[0];}finally{clearTimeout(timeout);}
   this.theme();
   this.editor=this.monaco.editor.create(this.host,{model:null,theme:'wattson-preparation',automaticLayout:true,minimap:{enabled:false},fontSize:15,wordWrap:'on',maxTokenizationLineLength:2000,autoIndent:'none',autoClosingBrackets:'never',autoClosingQuotes:'never',autoSurround:'never',formatOnPaste:false,formatOnType:false,scrollBeyondLastLine:false,accessibilitySupport:'on',editContext:false,ariaLabel:tr_web_prephase_editor_js('Document de préparation'),tabIndex:0,renderLineHighlight:'none',stickyScroll:{enabled:false}});
   this.editor.onDidChangeModelContent(()=>{if(!this.updating){this.wrap(this.editor.getValue());this.onChange(this.editor.getValue())}});
   this.simple=false;this.showMode();this.modeMessage(tr_web_prephase_editor_js('Éditeur enrichi · Ctrl/Cmd + S pour enregistrer · Ctrl + M pour le mode Tab.'));
   if(this.key)this.open(this.key,this.kind,this.plain.value);
  }catch(e){this.simple=true;this.showMode();this.modeMessage(tr_web_prephase_editor_js('Éditeur enrichi indisponible — votre texte reste éditable ici.'));}
 }
 wrap(text){const long=text.split("\n").some(line=>line.length>2000);if(this.longLines!==long){this.longLines=long;this.editor?.updateOptions({wordWrap:long?"off":"on",wordWrapOverride1:long?"off":"inherit",stopRenderingLineAfter:-1});}this.modeMessage(long?tr_web_prephase_editor_js("Éditeur enrichi · lignes longues : défilement horizontal, texte intégral conservé."):tr_web_prephase_editor_js("Éditeur enrichi · Ctrl/Cmd + S pour enregistrer · Ctrl + M pour le mode Tab."));}
 modeMessage(text){document.getElementById('editor-state').textContent=text;}
 theme(){
  if(!this.monaco)return;
  const palette=getComputedStyle(document.documentElement),t=role=>palette.getPropertyValue('--wattson-'+role).trim();
  const colors={'editor.background':t('champ'),'editor.foreground':t('texte'),'editorLineNumber.foreground':t('texte-discret'),'editorLineNumber.activeForeground':t('titre'),'editorCursor.foreground':t('lien'),'editor.selectionBackground':t('info-fond'),'editor.selectionForeground':t('info-encre'),'editor.inactiveSelectionBackground':t('carte-appuyee'),'editorWidget.background':t('carte'),'editorWidget.foreground':t('texte'),'editorWidget.border':t('ligne'),'input.background':t('champ'),'input.foreground':t('texte'),'focusBorder':t('lien'),'editorGutter.background':t('champ')};
  this.monaco.editor.defineTheme('wattson-preparation',{base:document.documentElement.dataset.theme==='sombre'?'vs-dark':'vs',inherit:false,rules:[{token:'',foreground:t('texte').replace(/^#/,'')},{token:'comment',foreground:t('texte-discret').replace(/^#/,'')},{token:'keyword',foreground:t('lien').replace(/^#/,'')},{token:'string',foreground:t('texte').replace(/^#/,'')},{token:'number',foreground:t('texte').replace(/^#/,'')}],colors});
  this.monaco.editor.setTheme('wattson-preparation');
 }
 open(key,kind,text){
  if(this.editor&&this.key)this.states.set(this.key,this.editor.saveViewState());
  this.key=key;this.kind=kind;this.updating=true;this.plain.value=text;
  if(this.editor){this.wrap(text);let model=this.models.get(key);if(!model){model=this.monaco.editor.createModel(text,kind==='plan'?'json':'markdown',this.monaco.Uri.parse('swarm://preparation/'+key));this.models.set(key,model)}else if(model.getValue()!==text)model.setValue(text);this.editor.setModel(model);if(this.states.has(key))this.editor.restoreViewState(this.states.get(key));}
  this.updating=false;this.showMode();
 }
 value(){return this.simple||!this.editor?this.plain.value:this.editor.getValue()}
 setText(text){this.updating=true;this.plain.value=text;if(this.editor&&this.editor.getModel()&&this.editor.getValue()!==text)this.editor.setValue(text);this.updating=false;}
 showMode(){this.host.hidden=this.simple||!this.editor;this.plain.hidden=!this.host.hidden;document.getElementById('editor-toggle').textContent=this.simple?tr_web_prephase_editor_js('Utiliser l’éditeur enrichi'):tr_web_prephase_editor_js('Utiliser l’éditeur texte');}
 async toggle(){const value=this.value();if(!this.editor){this.simple=false;await this.initialize();}else{this.simple=!this.simple;}this.setText(value);this.showMode();this.modeMessage(this.simple?tr_web_prephase_editor_js('Éditeur texte · Ctrl/Cmd + S pour enregistrer.'):tr_web_prephase_editor_js('Éditeur enrichi · Ctrl/Cmd + S pour enregistrer · Ctrl + M pour le mode Tab.'));}
 reset(){this.editor?.setModel(null);for(const m of this.models.values())m.dispose();this.models.clear();this.states.clear();this.key='';}
 dispose(){this.reset();this.editor?.dispose();}
}
