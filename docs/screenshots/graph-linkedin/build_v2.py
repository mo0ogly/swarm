"""Edit timestamped real browser captures; no synthetic UI or simulated agents."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import json,subprocess,textwrap,hashlib
root=Path(__file__).resolve().parent
source=Path('/home/fpizzi/workspace/rapports-banc/swarm-linkedin-v2-take2')
out=source/'edited';out.mkdir(exist_ok=True)
fdir=Path('/usr/share/fonts/truetype/dejavu')
def font(n,b=False):return ImageFont.truetype(str(fdir/('DejaVuSans-Bold.ttf' if b else 'DejaVuSans.ttf')),n)
logo=Image.open(root.parents[2]/'web/swarm-logo.png').convert('RGBA');logo.thumbnail((92,92))
def scene(seg,t):
 if seg=='01-graph':
  if t<3:return '01  VOIR LE PLAN','Des tâches reliées par des prérequis.',(270,70,1240,690)
  if t<7:return '02  EXPLORER','Zoomer pour lire une étape.',(280,120,1240,690)
  if t<14:return '03  RÉDUIRE LE BRUIT','Replier les branches. Garder le contexte.',(280,120,1240,690)
  if t<18:return '04  OUVRIR LE DÉTAIL','Passer du graphe à une tâche.',(280,120,1240,690)
  return '05  COMPRENDRE L’ATTENTE','Deux prérequis non validés. Deux tâches en aval.',(790,30,1225,660)
 if seg=='02-controls':return '06  DÉFINIR LA VALIDATION','Revue humaine ou contrôles structurés.',(170,30,1090,680)
 if seg=='02b-criterion':return '07  EXPLICITER LA PREUVE','Le critère de réussite est écrit avant le contrôle.',(190,260,1060,550)
 if t<4:return '08  LIER AU CANDIDAT','Déclarer les fichiers examinés.',(195,125,1065,605)
 if t<6.5:return '09  CHOISIR LE PROGRAMME','python3 : un programme autorisé et visible.',(195,125,1065,605)
 if t<9.7:return '10  BORNER L’ACTION','Les arguments sont exacts, un par ligne.',(200,135,1060,590)
 if t<19.5:return '11  CONFIGURER LES LIMITES','Un délai configurable : 60 secondes.',(200,125,1060,605)
 return '12  GARDER UNE PREUVE HONNÊTE','Exemple annulé. Aucun contrôle exécuté.',(270,60,1240,680)
def draw(im,title,caption,crop):
 canvas=Image.new('RGB',(1080,1350),'#101c28');d=ImageDraw.Draw(canvas);canvas.paste(logo,(40,25),logo)
 d.text((147,40),'SWARM',font=font(43,True),fill='#f7fafc');d.text((147,98),'Voir • comprendre • contrôler',font=font(23),fill='#90d1c8')
 d.text((40,188),title,font=font(27,True),fill='#efc878')
 y=245
 for l in textwrap.wrap(caption,36):d.text((40,y),l,font=font(40,True),fill='#f7fafc');y+=54
 pic=im.crop(crop);pic.thumbnail((1000,720),Image.Resampling.LANCZOS)
 scale=min(1000/pic.width,720/pic.height);pic=pic.resize((round(pic.width*scale),round(pic.height*scale)),Image.Resampling.LANCZOS)
 canvas.paste(pic,((1080-pic.width)//2,420+(720-pic.height)//2))
 d.text((40,1195),'Capture réelle de l’interface • montage pédagogique',font=font(23),fill='#90d1c8')
 d.text((40,1240),'Projet isolé Casa Pizza • aucun agent lancé',font=font(23),fill='#ccd6df')
 d.text((40,1280),'Exemple de contrôle non enregistré',font=font(23),fill='#ccd6df')
 return canvas
concat=[];timeline=[];offset=0;num=0;hashes={}
for seg in ['01-graph','02-controls','02b-criterion','03-fields']:
 rows=[json.loads(l) for l in (source/seg/'frames.jsonl').read_text().splitlines()]
 last=rows[-1]['t']
 for i,r in enumerate(rows):
  title,cap,crop=scene(seg,r['t']);p=source/seg/r['file'];im=draw(Image.open(p).convert('RGB'),title,cap,crop)
  target=out/f'{num:05}.png';im.save(target);num+=1
  dur=(rows[i+1]['t']-r['t']) if i+1<len(rows) else .35
  if seg=='02b-criterion':dur=3.5/len(rows)
  concat.extend([f"file '{target}'",f'duration {dur:.6f}']);timeline.append({'t':offset,'duration':dur,'segment':seg,'source':str(p),'title':title,'caption':cap,'crop':crop});offset+=dur
  hashes[str(p.relative_to(source))]=hashlib.sha256(p.read_bytes()).hexdigest()
# Closing card is explicitly editorial, not a product result.
card=Image.new('RGB',(1080,1350),'#101c28');d=ImageDraw.Draw(card);card.paste(logo,(45,70),logo)
d.text((155,90),'SWARM',font=font(58,True),fill='#f7fafc')
y=355
for line in ['Des agents probabilistes.', 'Un cadre de contrôle', 'explicite.']:
 d.text((50,y),line,font=font(49,True),fill='#f7fafc');y+=85
for j,line in enumerate(['Voir les dépendances.', 'Comprendre les blocages.', 'Définir les preuves et les limites.']):d.text((50,700+j*70),line,font=font(34),fill='#90d1c8')
d.text((50,1120),'Démonstration de navigation et de configuration',font=font(26),fill='#ccd6df');d.text((50,1170),'Aucun agent lancé • aucun résultat accepté simulé',font=font(25),fill='#ccd6df')
card.save(out/'end.png');concat.extend([f"file '{out/'end.png'}'",'duration 4',f"file '{out/'end.png'}'"]);offset+=4
(source/'timeline.txt').write_text('\n'.join(concat)+'\n');(root/'timeline-v2.json').write_text(json.dumps(timeline,ensure_ascii=False,indent=2))
mp4=root/'Swarm_graphe_LinkedIn_FR_V2.mp4'
# Resample the timestamped capture onto an 8 fps input sequence. Concat's PNG
# time base can truncate the closing card; fixed input timing preserves it.
import bisect,math
cfr=source/'cfr';cfr.mkdir(exist_ok=True);starts=[r['t'] for r in timeline]
end=timeline[-1]['t']+timeline[-1]['duration']
for i in range(math.ceil(offset*8)):
 t=i/8;src=(out/f'{max(0,bisect.bisect_right(starts,t)-1):05}.png') if t<end else out/'end.png'
 dst=cfr/f'{i:05}.png'
 if dst.is_symlink():dst.unlink()
 dst.symlink_to(src)
subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-framerate','8','-i',str(cfr/'%05d.png'),'-r','25','-c:v','libx264','-threads','2','-preset','ultrafast','-crf','19','-pix_fmt','yuv420p','-movflags','+faststart',str(mp4)],check=True)
# Animated actual graph recording for README.
subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(mp4),'-t','23','-filter_complex','fps=4,scale=480:-1,split[a][b];[a]palettegen[p];[b][p]paletteuse','-threads','2',str(root/'Swarm_graphe_apercu_FR_V2.gif')],check=True)
Image.open(out/'00012.png').save(root/'Swarm_graphe_LinkedIn_couverture_V2.png')
(root/'manifest-v2.json').write_text(json.dumps({'duration':offset,'size':[1080,1350],'encoded_fps':25,'capture':'Timestamped screenshots during actual browser interactions, approximately 3–5 captures/s; edited cuts and crop enlargement','source_hashes':hashes,'outputs':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [mp4,root/'Swarm_graphe_apercu_FR_V2.gif']}},indent=2,ensure_ascii=False))
print(json.dumps({'seconds':offset,'frames':num,'video':str(mp4)}))
