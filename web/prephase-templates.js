import {PreparationWorkGraph} from './preparation-work-graph.js';
const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
export class PreparationTemplates {
 constructor(api,methods,apply){
  this.api=api;this.methods=methods;this.apply=apply;this.items=[];this.answers=new Map();this.interventions=new Map();this.generation=0;this.$=id=>document.getElementById(id);
  this.workGraph=new PreparationWorkGraph(this.$);this.increments=new Map();
  this.$('template-increment').addEventListener('change',()=>{const t=this.selected();this.increments.set(t.id,this.$('template-increment').value);this.paint()});
  this.$('template-work-export').addEventListener('click',async e=>{if(!e.isTrusted)return;const v=await this.assess();if(!v?.work_plan)return;const url=URL.createObjectURL(new Blob([JSON.stringify(v.work_plan)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download=v.template_id+'-'+v.increment+'.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)});
  this.$('template-open').addEventListener('click',e=>{if(e.isTrusted)this.open()});
  this.$('template-choice').addEventListener('change',()=>this.paint());
  this.$('template-intervention').addEventListener('change',()=>{const t=this.selected();if(t){this.interventions.set(t.id,this.$('template-intervention').value);this.paint()}});
  this.$('template-check').addEventListener('click',e=>{if(e.isTrusted)this.assess()});
  this.$('template-use').addEventListener('click',async e=>{
   if(!e.isTrusted)return;const t=this.selected();if(!t)return;
   const result=await this.assess();if(!result)return;
   if(this.apply(result.title,result.need,result.recommended_method||this.method(t),result))this.$('template-dialog').close();
   else this.error(tr('Votre brouillon est conservé. Téléchargez-le ou commencez une nouvelle préparation avant d’utiliser un modèle.'));
  });
  this.$('template-dialog').addEventListener('close',()=>{this.generation++;this.$('template-open').focus()});
 }
 locale(){return globalThis.SwarmI18n?.language==='en'?'en':'fr'}
 selected(){return this.items.find(t=>t.id===this.$('template-choice').value)}
 intervention(t){return t.subject?.interventions.find(i=>i.id===(this.interventions.get(t.id)||t.subject.interventions[0]?.id))}
 method(t){return this.intervention(t)?.recommended_method||t.recommended_method}
 error(text){this.$('template-error').hidden=false;this.$('template-error').textContent=text}
 busy(value){for(const id of ['template-use','template-check','template-choice','template-intervention','template-questions','template-increment','template-work-export'])this.$(id).disabled=value}
 status(missing){
  this.$('template-check-status').className='notice '+(missing.length?'attention':'info');
  this.$('template-check-status').textContent=missing.length?tr('À compléter avant de préparer le plan : ')+missing.length:tr('Besoin renseigné — adoptez le brief et faites vérifier le plan avant lancement.');
  const t=this.selected(),lang=this.locale();this.$('template-missing').replaceChildren(...missing.map(id=>{const li=document.createElement('li'),button=document.createElement('button');button.type='button';button.textContent=t.questions.find(q=>q.id===id).label[lang];button.addEventListener('click',()=>this.$('template-answer-'+id).focus());li.append(button);return li}));
 }
 paint(){
  this.generation++;this.busy(false);this.$('template-error').hidden=true;
  const t=this.selected();if(!t){this.busy(true);return}
  const lang=this.locale(),answers=this.answers.get(t.id)||{};this.answers.set(t.id,answers);
  this.$('template-subject').hidden=!t.subject;
  this.$('template-intervention-field').hidden=!t.subject;
  this.$('template-intervention').replaceChildren(...(t.subject?.interventions||[]).map(i=>new Option(i.title[lang],i.id)));
  if(t.subject){
   this.$('template-intervention').value=this.intervention(t).id;
   this.$('template-outcome').textContent=t.subject.outcome[lang];this.$('template-example').textContent=t.subject.example[lang];
   for(const key of ['deliverables','evidence','risks'])this.$('template-'+key).replaceChildren(...t.subject[key][lang].map(text=>{const li=document.createElement('li');li.textContent=text;return li}));
   this.$('template-intervention-guidance').textContent=this.intervention(t).guidance[lang];
  }
  this.$('template-preview').textContent=t.need[lang];
  this.$('template-work-model').hidden=!t.work_model;
  this.$('template-increment').replaceChildren(...(t.work_model?.increments||[]).map(i=>new Option(i.title[lang],i.id)));
  if(t.work_model){const id=this.increments.get(t.id)||t.work_model.increments[0].id;this.$('template-increment').value=id;const inc=t.work_model.increments.find(i=>i.id===id);this.workGraph.paint(inc.plans[lang],t.work_model.decisions[lang])}
  const methodID=this.method(t),method=this.methods().find(m=>m.id===methodID);
  this.$('template-method').textContent=method?.available?tr(method.title)+' — '+(methodID==='debug'?tr('Diagnostic et planification disponibles dans ce projet.'):tr('Analyse et planification disponibles dans ce projet.')):(methodID==='debug'?tr('La méthode de diagnostic n’est pas disponible. Vous pouvez conserver le brouillon ; aucun appel IA ne sera lancé.'):tr('La méthode d’analyse et de planification n’est pas disponible. Vous pouvez conserver le brouillon ; aucun appel IA ne sera lancé.'));
  this.$('template-use').textContent=tr('Utiliser ce parcours');
  this.$('template-questions').replaceChildren(...t.questions.map(q=>{
   const group=document.createElement('div'),label=document.createElement('label'),input=document.createElement('textarea');input.id='template-answer-'+q.id;input.maxLength=1000;input.rows=2;input.value=answers[q.id]||'';label.htmlFor=input.id;label.textContent=q.label[lang];input.addEventListener('input',()=>{answers[q.id]=input.value;this.$('template-check-status').textContent=tr('Réponses modifiées — vérifiez-les pour actualiser le brouillon.');this.$('template-check-status').className='notice attention';this.$('template-error').hidden=true});group.append(label,input);return group;
  }));
  this.$('template-team').replaceChildren(...t.team.map(role=>{const card=document.createElement('article'),title=document.createElement('h4'),text=document.createElement('p');card.dataset.role=role.role;title.textContent=role.title[lang];text.textContent=role.responsibility[lang];const method=document.createElement('p');method.className='template-role-method';method.textContent=role.method_guidance[lang];card.append(title,text,method);return card}));
  this.status(t.questions.filter(q=>q.required&&!answers[q.id]?.trim()).map(q=>q.id));
  if(Object.values(answers).some(value=>value.trim())){this.$('template-check-status').textContent=tr('Réponses modifiées — vérifiez-les pour actualiser le brouillon.');this.$('template-check-status').className='notice attention'}
 }
 async assess(){
  const t=this.selected();if(!t)return null;
  const generation=++this.generation;this.busy(true);this.$('template-error').hidden=true;this.$('template-check-status').textContent=tr('Vérification des réponses…');
  try{
   const v=await this.api('preparations/template-check',{template_id:t.id,language:this.locale(),answers:{...this.answers.get(t.id)},...(t.work_model?{increment:this.$('template-increment').value}:{}),...(t.subject?{intervention:this.intervention(t).id}:{})});
   if(generation!==this.generation||!this.$('template-dialog').open)return null;
   this.$('template-preview').textContent=v.need;this.status(v.missing);return v;
  }catch(e){if(generation===this.generation){this.error(e.message);this.$('template-check-status').textContent=tr('Vérification indisponible — vos réponses sont conservées.')}return null}
  finally{if(generation===this.generation)this.busy(false)}
 }
 async open(){
  const previous=this.selected()?.id,generation=++this.generation;this.$('template-error').hidden=true;this.$('template-dialog').showModal();this.busy(true);this.$('template-preview').textContent=tr('Chargement des modèles…');
  try{const items=await this.api('preparations/templates');if(generation!==this.generation||!this.$('template-dialog').open)return;this.items=items;const lang=this.locale();this.$('template-choice').replaceChildren(...this.items.map(t=>new Option(t.title[lang],t.id)));const entry=previous||new URL(location.href).searchParams.get('template');if(entry&&this.items.some(t=>t.id===entry))this.$('template-choice').value=entry;this.paint()}
  catch(e){if(generation===this.generation)this.error(e.message)}
 }
}
