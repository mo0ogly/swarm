/** Render Swarm's source-mapped film with continuous vector animation.
 * Inputs: local scenes.json, canonical unmodified logo, bundled canvas/fonts.
 * Outputs: review stills or a silent H.264 MP4. No remote request or Swarm
 * operation is performed. Invalid text geometry, encoder failure or a broken
 * pipe refuses delivery; the caller retains the candidate and original media.
 * Illustrations explain contracts and are never presented as runtime capture.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
const require=createRequire('/home/fpizzi/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/@napi-rs/canvas/package.json');
const {createCanvas,GlobalFonts,loadImage}=require('@napi-rs/canvas');
const here=path.dirname(fileURLToPath(import.meta.url));
const root=path.resolve(here,'../../../..');
const qa=path.join(root,'.cache/editorial-swarm-20261023/media-refresh');
await fs.mkdir(qa,{recursive:true});
const spec=JSON.parse(await fs.readFile(path.join(here,'scenes.json'),'utf8'));
GlobalFonts.registerFromPath('/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf','Swarm Sans');
GlobalFonts.registerFromPath('/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf','Swarm Bold');
GlobalFonts.registerFromPath('/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf','Swarm Serif');
GlobalFonts.registerFromPath('/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf','Swarm Mono');
const logo=await loadImage(path.join(root,'web/swarm-logo.png'));
const W=1920,H=1080,FPS=spec.fps;
const C={bg:'#081b29',surface:'#102b3b',navy:'#062b49',cyan:'#93d4df',gold:'#f8c650',text:'#f6f3e9',muted:'#9eb6c3',red:'#ed8c85'};
const canvas=createCanvas(W,H),ctx=canvas.getContext('2d');
const base=createCanvas(W,H),b=base.getContext('2d');
b.fillStyle=C.bg;b.fillRect(0,0,W,H);
let grad=b.createRadialGradient(1300,530,30,1300,530,900);grad.addColorStop(0,'#15394a');grad.addColorStop(1,C.bg);b.fillStyle=grad;b.fillRect(0,0,W,H);
b.strokeStyle='#93d4df10';b.lineWidth=1;
for(let x=0;x<W;x+=80){b.beginPath();b.moveTo(x,0);b.lineTo(x,H);b.stroke()}
for(let y=0;y<H;y+=80){b.beginPath();b.moveTo(0,y);b.lineTo(W,y);b.stroke()}
const clamp=(x,a=0,z=1)=>Math.min(z,Math.max(a,x));
const ease=x=>{x=clamp(x);return 1-(1-x)**3};
const easeInOut=x=>{x=clamp(x);return x*x*(3-2*x)};
const mix=(a,b,t)=>a+(b-a)*t;
const warnings=new Set();
function font(size,bold=false,serif=false){ctx.font=`${size}px "${serif?'Swarm Serif':bold?'Swarm Bold':'Swarm Sans'}"`;}
/** Draw measured type; reject instead of silently clipping meaningful text. */
function text(s,x,y,size=28,color=C.text,opts={}){
 ctx.save();font(size,opts.bold,opts.serif);ctx.fillStyle=color;ctx.textAlign=opts.align||'left';ctx.textBaseline='alphabetic';
 const width=ctx.measureText(s).width;
 if(opts.maxWidth&&width>opts.maxWidth+1)warnings.add(`Text too wide ${Math.round(width)}>${opts.maxWidth}: ${s}`);
 ctx.fillText(s,x,y);ctx.restore();
}
/** Wrap caption prose to the declared display width with no text loss. */
function wrapped(s,x,y,maxWidth,size=30,color=C.text,lineHeight=41){
 font(size);const words=s.split(' ');let line='',lines=[];
 for(const word of words){const next=line?line+' '+word:word;if(ctx.measureText(next).width>maxWidth&&line){lines.push(line);line=word}else line=next}if(line)lines.push(line);
 lines.forEach((l,i)=>text(l,x,y+i*lineHeight,size,color));return lines.length;
}
function rr(x,y,w,h,r=8){ctx.beginPath();ctx.roundRect(x,y,w,h,r);}
function glowCircle(x,y,r,color,alpha=1){ctx.save();ctx.globalAlpha=alpha;ctx.fillStyle=color;ctx.shadowColor=color;ctx.shadowBlur=24;ctx.beginPath();ctx.arc(x,y,r,0,Math.PI*2);ctx.fill();ctx.restore()}
function ring(x,y,r,alpha=.3,color=C.cyan){ctx.save();ctx.globalAlpha=alpha;ctx.strokeStyle=color;ctx.lineWidth=1;ctx.beginPath();ctx.arc(x,y,r,0,Math.PI*2);ctx.stroke();ctx.restore()}
/** Animate a cubic link and its moving signal; it denotes a conceptual flow. */
function link(a,z,t,delay=0,color=C.cyan,curve=0,dashed=false){
 const p=ease((t-delay)/1.3);if(p<=0)return;
 const dx=z[0]-a[0],dy=z[1]-a[1],c1=[a[0]+dx*.34,a[1]+dy*.1+curve],c2=[a[0]+dx*.66,a[1]+dy*.9+curve];
 ctx.save();ctx.globalAlpha=.45*p;ctx.lineWidth=2;ctx.strokeStyle=color;if(dashed)ctx.setLineDash([7,8]);ctx.beginPath();ctx.moveTo(...a);ctx.bezierCurveTo(...c1,...c2,...z);ctx.stroke();ctx.setLineDash([]);
 const u=((t-delay)*.16)%1;if(u>=0){const v=1-u;const px=v**3*a[0]+3*v*v*u*c1[0]+3*v*u*u*c2[0]+u**3*z[0],py=v**3*a[1]+3*v*v*u*c1[1]+3*v*u*u*c2[1]+u**3*z[1];glowCircle(px,py,4,color,p)}
 ctx.restore();
}
/** Reveal an explanatory node with optional contract/future status. */
function node(x,y,w,h,title,sub,t,delay=0,tone='cyan'){
 const p=ease((t-delay)/.9);if(!p)return;ctx.save();ctx.globalAlpha=p;ctx.translate(0,(1-p)*18);const color=C[tone]||tone;
 rr(x-w/2,y-h/2,w,h);ctx.fillStyle=C.surface;ctx.fill();ctx.strokeStyle=color;ctx.globalAlpha=.75*p;ctx.lineWidth=tone==='gold'?2:1.2;ctx.stroke();ctx.globalAlpha=p;
 if(tone==='gold'){ctx.fillStyle=C.gold;ctx.fillRect(x-w/2+1,y-h/2+1,Math.min(w-2,70),3)}
 const titles=[];font(27,true);
 for(const raw of Array.isArray(title)?title:[title]){let line='';for(const word of raw.split(' ')){const next=line?line+' '+word:word;if(ctx.measureText(next).width>w-24&&line){titles.push(line);line=word}else line=next}titles.push(line)}
 if(titles.length>2)warnings.add('Node title needs more height: '+title);
 const lineh=32;let yy=y-(titles.length-1)*lineh/2-(sub?8:-8);
 titles.forEach((s,i)=>text(s,x,yy+i*lineh,27,C.text,{bold:true,align:'center',maxWidth:w-24}));
 if(sub)text(sub,x,y+h/2-20,21,C.muted,{align:'center',maxWidth:w-22});ctx.restore();
}
function chip(s,x,y,color=C.gold){font(18);const w=ctx.measureText(s).width+30;ctx.save();rr(x,y-25,w,38,4);ctx.fillStyle=C.surface;ctx.fill();ctx.strokeStyle=color;ctx.lineWidth=1;ctx.stroke();text(s,x+15,y,18,color,{bold:true});ctx.restore()}
function background(t){
 ctx.drawImage(base,0,0);
 for(let i=0;i<48;i++){const x=(i*397.53+t*(5+i%3))%W,y=(i*173.71+Math.sin(t*.2+i)*18)%H;ctx.fillStyle=i%6===0?'#f8c65040':'#93d4df24';ctx.beginPath();ctx.arc(x,y,i%6===0?2:1,0,Math.PI*2);ctx.fill()}
 ctx.drawImage(logo,92,42,55,55);text('SWARM',164,80,28,C.text,{bold:true});
 text('IDENTITÉ  /  MÉMOIRE  /  CONTRÔLE',1820,75,17,C.muted,{align:'right'});
}
/** Draw a chapter with room for the caption and explicit maturity/footer notes. */
function scene(s,t){
 ctx.save();const alpha=easeInOut(t/.55)*easeInOut((s.duration-t)/.55);ctx.globalAlpha=alpha;
 const p=ease(t/1.1);ctx.save();ctx.translate((1-p)*-18,0);
 text(s.kicker,104,145,20,C.gold,{bold:true});
 s.title.forEach((line,i)=>text(line,100,240+i*82,76,C.text,{serif:true,maxWidth:1730}));
 text(s.subtitle,104,365,29,C.cyan,{maxWidth:1720});ctx.restore();
 const T=t-.5;
 if(s.id==='hook'){
  const center=[1240,640];ring(...center,190,.20);ring(...center,230,.10);ring(...center,280,.07);
  const points=[[470,650],[850,650],[1240,650],[1590,650]];
  for(let i=0;i<3;i++)link([points[i][0]+120,650],[points[i+1][0]-120,650],T,i*.8,i>0?C.red:C.gold);
  node(470,650,280,126,'UNE COMMANDE','Source extérieure',T,.2,'gold');node(850,650,280,126,'UN SOUVENIR','Conservé puis relu',T,1.1,'gold');node(1240,650,280,126,['UNE AUTRE','INSTANCE'],'Contexte hérité',T,2,'red');
  glowCircle(1590,650,12,C.red,ease((T-3)/1.2));text('ET ENSUITE ?',1590,718,21,C.muted,{align:'center'});
  text('SCÉNARIO ILLUSTRATIF',104,830,18,C.muted);
 }else if(s.id==='memory'){
  const xs=[330,750,1170,1590];const titles=['SOURCE','RÉSUMÉ','MÉMOIRE','INSTANCE SUIVANTE'];const subs=['Document ou outil','Sens et réserves','Version et provenance','Nouveau contexte'];
  for(let i=0;i<3;i++)link([xs[i]+144,580],[xs[i+1]-144,580],T,i*.6,i===1?C.gold:C.cyan);
  xs.forEach((x,i)=>node(x,580,290,118,titles[i],subs[i],T,i*.6,i===2?'gold':'cyan'));
  ['Lire','Résumer','Transmettre'].forEach((s,i)=>{ctx.save();rr(550+i*310,735,200,70,35);ctx.strokeStyle=C.gold;ctx.globalAlpha=.5;ctx.stroke();ctx.restore();text(s,650+i*310,779,24,C.gold,{align:'center'});});
  text('Un souvenir ne crée aucun droit.',104,833,25,C.text);
 }else if(s.id==='identity'){
  const pts=[[380,590],[900,590],[1440,590]];
  link([535,590],[745,590],T,.5);link([1055,590],[1285,590],T,1.1);
  node(380,590,310,130,['MANDANT','Humain ou service'],'Source de l’autorité',T,.1,'gold');node(900,590,310,130,'AGENT LOGIQUE','Rôle et périmètre',T,.8);node(1440,590,310,130,'INSTANCE','Exécution attribuée',T,1.5);
  link([900,655],[900,710],T,2,C.gold);node(900,770,630,94,'OPÉRATION · RESSOURCE · PORTÉE','Expiration, révocation et état courant',T,2,'gold');
  chip('SIGNATURE ≠ VÉRITÉ',1390,812,C.gold);
 }else if(s.id==='rust'){
  const cx=1020,cy=635;ring(cx,cy,204,.23,C.gold);ring(cx,cy,244,.12,C.cyan);
  const nodes=[['Objectif','Plan adopté',cx-580,cy-165],['Droits','Autorité et portée',cx,cy-180],['Budgets','Réservations',cx+580,cy-165],['Contexte','Versions reçues',cx-580,cy+165],['Preuves','Candidat et fraîcheur',cx,cy+190],['Transitions','État durable',cx+580,cy+165]];
  nodes.forEach(([a,b,x,y],i)=>{link([x,y],[cx,cy],T,i*.25,C.cyan,i%2?25:-25);node(x,y,300,92,a,b,T,i*.25,'cyan')});
  node(cx,cy,450,152,['MOTEUR DÉTERMINISTE','RUST'],'swarm-engine · WorkEngine · RootStore',T,.1,'gold');
 }else if(s.id==='human'){
  const xs=[300,700,1100,1550];for(let i=0;i<3;i++)link([xs[i]+130,570],[xs[i+1]-145,570],T,i*.6,i===2?C.gold:C.cyan);
  node(300,570,260,115,'CANDIDAT','Résultat à examiner',T,.1);node(700,570,280,115,'CONTRÔLES','Preuves produites',T,.7);node(1100,570,280,115,'REVUE IA','Examen indépendant',T,1.3);node(1550,570,300,130,['DÉCISION','HUMAINE'],'Requise en mode human',T,2,'gold');
  link([1550,640],[1550,728],T,2.5,C.gold);node(1550,785,320,88,'AVAL EN ATTENTE','Aucun déblocage par un PASS',T,2.5,'gold');
  text('PASS de contrôle',280,790,29,C.cyan);text('≠',580,790,46,C.gold);text('validation du résultat',650,790,29,C.text);
 }else if(s.id==='supervision'){
  const pts=[[460,520],[1390,520],[1390,790],[460,790]];const texts=[['INCIDENT','Tâche · agent · tentative'],['SUPERVISEUR','Diagnostic et décision'],['MOTEUR RUST','Droits puis application'],['EFFET OBSERVÉ','Reprise ou nouveau diagnostic']];
  for(let i=0;i<4;i++)link(pts[i],pts[(i+1)%4],T,i*.6,i===2?C.gold:C.cyan,(i===0?-100:i===2?100:0));
  pts.forEach(([x,y],i)=>node(x,y,390,105,texts[i][0],texts[i][1],T,i*.6,i===2?'gold':'cyan'));
  ring(925,655,105,.28,C.gold);text('RACINE',925,649,28,C.gold,{bold:true,align:'center'});text('Arbitrages transversaux',925,684,18,C.muted,{align:'center'});
 }else if(s.id==='logs'){
  const xs=[260,610,960,1310,1660];const ts=[['CONTEXTE','Version reçue'],['TENTATIVE','Agent et outils'],['DÉCISION','Auteur et motif'],['APPLICATION','Révision contrôlée'],['EFFET','Preuve observée']];
  for(let i=0;i<4;i++)link([xs[i]+130,605],[xs[i+1]-130,605],T,i*.6,C.cyan);
  ts.forEach(([a,b],i)=>{node(xs[i],605,270,125,a,b,T,i*.6,i===2?'gold':'cyan');text(String(i+1).padStart(2,'0'),xs[i],738,24,C.gold,{align:'center'})});
  text('Tracer les faits. Conserver les incertitudes.',104,837,28,C.text);
 }else if(s.id==='future'){
  chip('AC02  ·  ENREGISTRÉ  ·  NON DÉMARRÉ',104,433,C.gold);
  const xs=[330,750,1170,1590],labels=[['POLITIQUE','Règles et révisions'],['REFUS','Cause et règle'],['CHAÎNE','Processus et connexion'],['TIMELINE','Décisions de sécurité']];
  for(let i=0;i<3;i++)link([xs[i]+142,630],[xs[i+1]-142,630],T,.5+i*.5,C.gold,0,true);
  labels.forEach(([a,b],i)=>node(xs[i],630,294,124,a,b,T,.5+i*.5,'gold'));
  text('Confinement Linux   /   Réseau TCP et REST   /   Usage des secrets',104,820,30,C.cyan);
 }else if(s.id==='closing'){
  ctx.drawImage(logo,1060,430,340,340);
  text('SWARM',1440,615,104,C.text,{bold:true});
  text('Un cadre vérifiable pour le travail des agents',104,520,35,C.gold);
  const names=['Moteur Rust','Identité et autorité','Mémoire et contexte','Supervision','Logs et preuves'];
  names.forEach((n,i)=>text(String(i+1).padStart(2,'0')+'   '+n,110,597+i*42,26,C.cyan));
  text('MINJA · MemoryGraft · Microsoft · MemGhost · Emergence World',104,851,21,C.muted);
 }
 const cue=s.captions[Math.min(2,Math.floor(t/s.duration*3))].fr;
 ctx.fillStyle='#081b29e8';ctx.fillRect(90,900,1740,112);ctx.fillStyle=C.gold;ctx.fillRect(102,914,3,74);
 const n=wrapped(cue,126,950,1640,31,C.text,42);if(n>2)warnings.add('Caption >2 lines: '+cue);
 ctx.restore();
}
/** Render a deterministic frame at a known media time, including full fade transitions. */
function render(time){
 background(time);let idx=spec.scenes.findIndex(s=>time>=s.start&&time<s.start+s.duration);if(idx<0)idx=spec.scenes.length-1;
 const s=spec.scenes[idx];scene(s,time-s.start);
 ctx.fillStyle='#93d4df24';ctx.fillRect(104,1048,1716,2);ctx.fillStyle=C.gold;ctx.fillRect(104,1048,1716*clamp(time/spec.duration),2);
 text('Architecture explicative  ·  Qualification Rust par parcours  ·  AC02 futur',104,1030,17,C.muted);
 text(String(idx+1).padStart(2,'0')+' / 09',1820,1030,17,C.muted,{align:'right'});
 if(warnings.size)throw new Error([...warnings].join('\n'));
}
const mode=process.argv[2]||'--stills';
if(mode==='--stills'){
 await fs.mkdir(path.join(qa,'stills'),{recursive:true});
 for(const s of spec.scenes)for(const [j,ratio]of[.22,.56,.86].entries()){
  render(s.start+s.duration*ratio);await fs.writeFile(path.join(qa,'stills',s.id+'-'+j+'.png'),canvas.toBuffer('image/png'));
 }
 // Poster uses the same renderer, not an invented product screenshot.
 render(40.5);await fs.writeFile(path.join(here,'apercu.png'),canvas.toBuffer('image/png'));
 console.log(JSON.stringify({stills:27,poster:'apercu.png',geometryWarnings:0}));
}else if(mode==='--encode'||mode==='--benchmark'){
 const count=mode==='--benchmark'?90:spec.duration*FPS;
 const target=path.join(qa,mode==='--benchmark'?'benchmark.mp4':'film-silent.candidate.mp4');
 const args=['-hide_banner','-loglevel','warning','-y','-f','rawvideo','-pix_fmt','rgba','-s',W+'x'+H,'-r',String(FPS),'-i','pipe:0','-an','-c:v','libx264','-preset','fast','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart',target];
 const proc=spawn('/usr/bin/ffmpeg',args,{stdio:['pipe','ignore','pipe']});let err='';proc.stderr.on('data',d=>{err+=d.toString()});
 const complete=new Promise((resolve,reject)=>{proc.on('error',reject);proc.on('close',code=>code===0?resolve():reject(new Error(`FFmpeg ${code}: ${err}`)))});
 proc.stdin.on('error',()=>{});const began=Date.now();
 for(let f=0;f<count;f++){
  render(f/FPS);if(!proc.stdin.write(canvas.data()))await once(proc.stdin,'drain');
  if(f%300===0&&f>0)console.log(JSON.stringify({frame:f,total:count,seconds:Math.round((Date.now()-began)/1000)}));
 }
 proc.stdin.end();await complete;
 const result={frames:count,seconds:(Date.now()-began)/1000,target,encoderArgv:args,stderr:err,geometryWarnings:0};
 await fs.writeFile(path.join(qa,mode==='--benchmark'?'benchmark.json':'encode.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}else throw new Error('Unknown mode '+mode);
