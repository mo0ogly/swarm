'use strict';
const trPricing=s=>globalThis.SwarmI18n?.t(s)??s;
const Pricing={
 catalogue:null,
 async load(){this.catalogue=await api('/api/v1/pricing');return this.catalogue},
 async open(){try{await this.load();this.list()}catch(e){notice(e.message,true)}},
 list(){
  openModal(trPricing('Tarifs des modèles'),trPricing('Tarifs saisis par vous, par million de jetons. Aucune facturation ni limite de mission ne change ici.'),{action:'pricing-list'});$('confirm').hidden=true;returnFocus=$('pricing-open');
  const box=$('modal-fields');box.append(button(trPricing('Ajouter un tarif'),()=>this.edit()));
  if(!this.catalogue.rates.length)box.append(node('p',trPricing('Aucun tarif configuré. Aucun prix n’est supposé nul.'),'notice info'));
  for(const r of [...this.catalogue.rates].reverse()){
   const card=node('section',undefined,'card field-wide');card.append(node('h3',r.provider+' · '+r.model),node('p',r.version+' · '+r.reference_date+' · '+r.currency),node('p',r.source));
   for(const [key,label]of [['input_per_million','Entrée hors cache'],['output_per_million','Sortie'],['cache_read_per_million','Cache lu'],['cache_write_per_million','Cache écrit']])card.append(node('p',trPricing(label)+' : '+(r[key]===null?trPricing('Inconnu'):r[key]+' '+r.currency)));
   card.append(button(trPricing('Créer une nouvelle version'),()=>this.edit(r)),button(trPricing('Estimer avec ce tarif'),()=>this.estimate(r)));box.append(card);
  }
 },
 edit(r={}){
  openModal(trPricing('Enregistrer un tarif'),trPricing('Une nouvelle version sera conservée. Un champ vide signifie inconnu ; saisir zéro signifie gratuit. Aucun prix n’est récupéré automatiquement.'),{action:'pricing-save',pricingDigest:this.catalogue.digest});
  for(const [key,label,value]of [['provider','Fournisseur',r.provider],['model','Modèle',r.model],['currency','Devise — code à trois lettres',r.currency],['source','Source du tarif',r.source],['reference_date','Date de référence — AAAA-MM-JJ',r.reference_date]]){const el=field(key,trPricing(label),value||'');el.required=true;}
  for(const [key,label]of [['input_per_million','Entrée hors cache'],['output_per_million','Sortie'],['cache_read_per_million','Cache lu'],['cache_write_per_million','Cache écrit']]){const el=field(key,trPricing(label)+' / 1 000 000',r[key]??'');el.type='number';el.min='0';el.max='1000000000';el.step='any'}
  $('confirm').textContent=trPricing('Enregistrer un tarif');returnFocus=$('pricing-open');
 },
 estimate(rate){
  openModal(trPricing('Estimer avec ce tarif'),trPricing('Saisir des volumes distincts : entrée hors cache, sortie, cache lu et cache écrit. Le résultat est une simulation, pas un coût facturé ni un historique de tentative.'),{action:'pricing-estimate',pricingRate:rate});
  for(const [key,label]of [['non_cached_input_tokens','Entrée hors cache'],['output_tokens','Sortie'],['cache_read_tokens','Cache lu'],['cache_write_tokens','Cache écrit']]){const el=field(key,trPricing(label),key.startsWith('cache_')?'0':'');el.type='number';el.min='0';el.max='1000000000000';el.step='1';el.required=true;el.oninput=()=>{$('preview').hidden=true}}
  $('confirm').textContent=trPricing('Calculer l’estimation');returnFocus=$('pricing-open');
 },
 async submit(c,f){
  if(c.action==='pricing-save'){
   const rate={provider:f.provider,model:f.model,currency:f.currency,source:f.source,reference_date:f.reference_date};for(const k of ['input_per_million','output_per_million','cache_read_per_million','cache_write_per_million'])rate[k]=f[k].trim()===''?null:Number(f[k]);
   await api('/api/v1/pricing',{schema_version:1,expected_digest:c.pricingDigest,rate});await this.load();if(modalContext===c){closeModal();notice(trPricing('Tarif enregistré. Les versions précédentes sont conservées.'))}
  }else{
   const r={rate_version:c.pricingRate.version};for(const k of ['non_cached_input_tokens','output_tokens','cache_read_tokens','cache_write_tokens'])r[k]=Number(f[k]);
   const q=await api('/api/v1/pricing/estimate',r);if(modalContext!==c)return;
   preview((q.status==='estimated'?trPricing('Estimation : ')+q.estimated_amount+' '+q.rate.currency:trPricing('Estimation inconnue : un tarif nécessaire est absent.'))+'\n'+q.rate.provider+' · '+q.rate.model+' · '+q.rate.version+'\n'+q.rate.source+' · '+q.rate.reference_date);
  }
 }
};
$('pricing-open').onclick=()=>Pricing.open();
