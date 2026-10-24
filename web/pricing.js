'use strict';
const trPricing=s=>globalThis.SwarmI18n?.t(s)??s;
/** Build the existing pricing renderer using caller-owned modal/API hooks.
 * It preserves immutable versions, explicit unknown rates and quote semantics.
 * Hooks own authentication, errors, safe rendering and the modal lifecycle;
 * this component never launches providers or changes mission budgets. */
function createPricing({api,$,openModal,button,node,field,preview,closeModal,notice,getContext,setReturnFocus}){return {
 catalogue:null,
 // load observes the project catalogue; API failures propagate without an empty fallback.
 async load(){this.catalogue=await api('/api/v1/pricing');return this.catalogue},
 // open remembers the actual entry control for modal focus restoration; reports read errors.
 async open(opener=$('pricing-open')){this.opener=opener;try{await this.load();this.list()}catch(e){notice(e.message,true)}},
 // list renders retained versions without changing prices or mission budgets.
 list(){
  openModal(trPricing('Tarifs des modèles'),trPricing('Tarifs saisis par vous, par million de jetons. Aucune facturation ni limite de mission ne change ici.'),{action:'pricing-list'});$('confirm').hidden=true;setReturnFocus(this.opener||$('pricing-open'));
  const box=$('modal-fields');box.append(button(trPricing('Ajouter un tarif'),()=>this.edit()));
  if(!this.catalogue.rates?.length)box.append(node('p',trPricing('Aucun tarif configuré. Aucun prix n’est supposé nul.'),'notice info'));
  for(const r of [...(this.catalogue.rates??[])].reverse()){
   const card=node('section',undefined,'card field-wide');card.append(node('h3',r.provider+' · '+r.model),node('p',r.version+' · '+r.reference_date+' · '+r.currency),node('p',r.source));
   for(const [key,label]of [['input_per_million','Entrée hors cache'],['output_per_million','Sortie'],['cache_read_per_million','Cache lu'],['cache_write_per_million','Cache écrit']])card.append(node('p',trPricing(label)+' : '+(r[key]===null?trPricing('Inconnu'):r[key]+' '+r.currency)));
   card.append(button(trPricing('Créer une nouvelle version'),()=>this.edit(r)),button(trPricing('Estimer avec ce tarif'),()=>this.estimate(r)));box.append(card);
  }
 },
 // edit prepares a new version from optional prior data; blank prices remain unknown.
 edit(r={}){
  openModal(trPricing('Enregistrer un tarif'),trPricing('Une nouvelle version sera conservée. Un champ vide signifie inconnu ; saisir zéro signifie gratuit. Aucun prix n’est récupéré automatiquement.'),{action:'pricing-save',pricingDigest:this.catalogue.digest});
  for(const [key,label,value]of [['provider','Fournisseur',r.provider],['model','Modèle',r.model],['currency','Devise — code à trois lettres',r.currency],['source','Source du tarif',r.source],['reference_date','Date de référence — AAAA-MM-JJ',r.reference_date]]){const el=field(key,trPricing(label),value||'');el.required=true;}
  for(const [key,label]of [['input_per_million','Entrée hors cache'],['output_per_million','Sortie'],['cache_read_per_million','Cache lu'],['cache_write_per_million','Cache écrit']]){const el=field(key,trPricing(label)+' / 1 000 000',r[key]??'');el.type='number';el.min='0';el.step='any'}
  $('confirm').textContent=trPricing('Enregistrer un tarif');setReturnFocus(this.opener||$('pricing-open'));
 },
 // estimate requests explicit category volumes for a selected immutable version.
 estimate(rate){
  openModal(trPricing('Estimer avec ce tarif'),trPricing('Saisir des volumes distincts : entrée hors cache, sortie, cache lu et cache écrit. Le résultat est une simulation, pas un coût facturé ni un historique de tentative.'),{action:'pricing-estimate',pricingRate:rate});
  for(const [key,label]of [['non_cached_input_tokens','Entrée hors cache'],['output_tokens','Sortie'],['cache_read_tokens','Cache lu'],['cache_write_tokens','Cache écrit']]){const el=field(key,trPricing(label),key.startsWith('cache_')?'0':'');el.type='number';el.min='0';el.step='1';el.required=true;el.oninput=()=>{$('preview').hidden=true}}
  $('confirm').textContent=trPricing('Calculer l’estimation');setReturnFocus(this.opener||$('pricing-open'));
 },
 // submit validates browser representations, then uses shared save/estimate APIs.
 // Conflicts and API errors propagate to the modal; simulation never records spend.
 async submit(c,f){
  if(c.action==='pricing-save'){
   const rate={provider:f.provider,model:f.model,currency:f.currency,source:f.source,reference_date:f.reference_date};for(const k of ['input_per_million','output_per_million','cache_read_per_million','cache_write_per_million']){const n=f[k].trim()===''?null:Number(f[k]);if(n!==null&&(!Number.isFinite(n)||n<0))throw Error(trPricing('Les tarifs doivent être finis, positifs ou nuls.'));rate[k]=n;}
   const saved=await api('/api/v1/pricing',{schema_version:1,expected_digest:c.pricingDigest,rate});await this.load();if(!this.catalogue.rates?.some(r=>r.version===saved.version))throw Error('pricing-saved-version-not-observed');if(getContext()===c){closeModal();notice(trPricing('Tarif enregistré. Les versions précédentes sont conservées.'))}
  }else{
   const r={rate_version:c.pricingRate.version};for(const k of ['non_cached_input_tokens','output_tokens','cache_read_tokens','cache_write_tokens']){const n=Number(f[k]);if(f[k].trim()===''||!Number.isSafeInteger(n)||n<0)throw Error(trPricing('Saisir un entier positif ou nul représentable exactement dans le navigateur.'));r[k]=n;}
   const q=await api('/api/v1/pricing/estimate',r);if(getContext()!==c)return;
   preview((q.status==='estimated'?trPricing('Estimation : ')+q.estimated_amount+' '+q.rate.currency:trPricing('Estimation inconnue : un tarif nécessaire est absent.'))+'\n'+q.rate.provider+' · '+q.rate.model+' · '+q.rate.version+'\n'+q.rate.source+' · '+q.rate.reference_date);
  }
 }
}};
const Pricing=createPricing(globalThis.SwarmNativePricingHooks||{
 api:(...args)=>api(...args),$:(...args)=>$(...args),openModal:(...args)=>openModal(...args),button:(...args)=>button(...args),node:(...args)=>node(...args),field:(...args)=>field(...args),preview:(...args)=>preview(...args),closeModal:()=>closeModal(),notice:(...args)=>notice(...args),getContext:()=>modalContext,setReturnFocus:value=>{returnFocus=value}
});
document.getElementById('pricing-open').onclick=()=>Pricing.open();

// Administration reuses the existing pricing dialog and authoritative API.
document.getElementById('admin-pricing-open').onclick=()=>Pricing.open(document.getElementById('admin-pricing-open'));
