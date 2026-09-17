'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const web=path.resolve(__dirname,'../web');
const css=fs.readFileSync(path.join(web,'prephase.css'),'utf8'),js=fs.readFileSync(path.join(web,'prephase-editor.js'),'utf8');
const tokens=[...new Set([...css.matchAll(/--wattson-[a-z-]+/g)].map(m=>m[0]).concat([...js.matchAll(/\bt\('([a-z-]+)'\)/g)].map(m=>'--wattson-'+m[1])))];
module.exports=async page=>{
 const result=await page.evaluate(tokens=>{
  const root=getComputedStyle(document.documentElement);const missing=tokens.filter(t=>!root.getPropertyValue(t).trim());
  const rgba=s=>{const m=s.match(/rgba?\(([^)]+)\)/);return m?m[1].split(',').map(Number):null};
  const luminance=c=>c.slice(0,3).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0);
  const ratio=(a,b)=>{const x=luminance(a),y=luminance(b);return(Math.max(x,y)+.05)/(Math.min(x,y)+.05)};
  const samples=[];
  for(const selector of ['#advance','#save-state','.prep-note p','.document-tabs [aria-selected=true]','#method-status']){
   const el=document.querySelector(selector),s=getComputedStyle(el);let node=el,bg;
   while(node){const cs=getComputedStyle(node),color=rgba(cs.backgroundColor);if(color&&(!color[3]||color[3]===1)&&cs.backgroundColor!=='rgba(0, 0, 0, 0)'){bg=color;break;}node=node.parentElement;}
   const gradient=[...s.backgroundImage.matchAll(/rgba?\([^)]+\)/g)].map(m=>rgba(m[0]));const backgrounds=gradient.length?gradient:[bg];
   const threshold=parseFloat(s.fontSize)>=18.667&&Number(s.fontWeight)>=700?3:4.5;
   const minimum=Math.min(...backgrounds.map(b=>ratio(rgba(s.color),b)));
   samples.push({selector,minimum,threshold});
  }
  return{theme:document.documentElement.dataset.theme,missing,samples};
 },tokens);
 assert.deepEqual(result.missing,[]);assert.equal(result.samples.length,5);
 for(const s of result.samples)assert.ok(s.minimum>=s.threshold,JSON.stringify({theme:result.theme,...s}));
 return result;
};
