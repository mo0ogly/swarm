"""Reproducible LinkedIn edit from nine actual UI states. No simulated agents."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import subprocess,json,textwrap
root=Path(__file__).resolve().parent
fontdir=Path('/usr/share/fonts/truetype/dejavu')
def font(n,b=False):return ImageFont.truetype(str(fontdir/('DejaVuSans-Bold.ttf' if b else 'DejaVuSans.ttf')),n)
scenes=[
('01-dependances',5,'VOIR LE PLAN','Six tâches.\nDes dépendances explicites.','Les flèches montrent les prérequis, pas une promesse de réussite.',(290,120,1240,590)),
('02-zoom',5,'EXPLORER','Zoomer pour lire\nchaque étape.','Une vue globale, puis le détail utile à la décision.',(290,120,1240,590)),
('04-branche',5,'RÉDUIRE LE BRUIT','Replier une branche.\nGarder les liens utiles.','Le repli réduit la vue ; il ne supprime aucune tâche.',(290,120,1240,590)),
('03-prerequis',5,'INSPECTER','Ouvrir la tâche\ndepuis le graphe.','Consulter le résultat attendu et les actions disponibles.',(785,0,1240,680)),
('05-impact',7,'COMPRENDRE','Pourquoi cette tâche\nne démarre pas ?','Deux prérequis restent non validés. Deux tâches sont en aval.',(785,0,1240,680)),
('06-criteres',5,'DÉFINIR LA PREUVE','Choisir ce qui doit\nêtre vérifié.','Le critère est explicite. La revue humaine reste une option.',(170,30,1090,680)),
('08-entrees',5,'LIER AU CANDIDAT','Déclarer les fichiers\net le programme.','Exemple préparé dans l’interface, non enregistré dans cette démo.',(190,135,1060,645)),
('07-controle',6,'BORNER LE CONTRÔLE','Arguments exacts.\nDélai explicite.','La politique décrit ce qui est autorisé avant toute exécution.',(190,125,1060,600)),
('09-conclusion',5,'SWARM','Des agents probabilistes.\nUn cadre de contrôle explicite.','Voir le plan, comprendre les blocages, définir les preuves.',(290,120,1240,590))]
(root/'renders').mkdir(exist_ok=True)
logo=Image.open(root.parents[2]/'web/swarm-logo.png').convert('RGBA');logo.thumbnail((90,90))
concat=[];srt=[];offset=0
for n,(name,dur,title,lead,caption,crop) in enumerate(scenes,1):
 canvas=Image.new('RGB',(1920,1080),'#101c28');d=ImageDraw.Draw(canvas)
 canvas.paste(logo,(36,24),logo);d.text((138,38),'SWARM',font=font(40,True),fill='#f7fafc')
 d.text((38,150),f'{n:02d} / {len(scenes):02d}',font=font(26),fill='#77bcb4')
 d.text((38,208),title,font=font(30,True),fill='#efc878')
 y=295
 for paragraph in lead.splitlines():
  for line in textwrap.wrap(paragraph,width=19):d.text((38,y),line,font=font(34,True),fill='#f7fafc');y+=48
 y+=44
 for line in textwrap.wrap(caption,width=29):d.text((38,y),line,font=font(25),fill='#ccd6df');y+=38
 d.text((38,934),'DÉMONSTRATION DU COCKPIT',font=font(18,True),fill='#77bcb4')
 d.text((38,970),'Plan pédagogique • aucun agent lancé',font=font(16),fill='#ccd6df')
 im=Image.open(root/'frames'/f'{name}.png').convert('RGB').crop(crop)
 im.thumbnail((1380,940),Image.Resampling.LANCZOS)
 # Increase screenshot size while preserving geometry and text.
 scale=min(1380/im.width,940/im.height);im=im.resize((round(im.width*scale),round(im.height*scale)),Image.Resampling.LANCZOS)
 x=510+(1380-im.width)//2;y=50+(940-im.height)//2;canvas.paste(im,(x,y))
 d.rectangle((510,1024,1890,1030),fill='#293c4e');d.rectangle((510,1024,510+int(1380*n/len(scenes)),1030),fill='#77bcb4')
 out=root/'renders'/f'{n:02d}.png';canvas.save(out)
 concat += [f"file '{out}'",f'duration {dur}']
 def stamp(t):return f'{t//3600:02}:{t//60%60:02}:{t%60:02},000'
 srt.append(f'{n}\n{stamp(offset)} --> {stamp(offset+dur)}\n{lead.replace(chr(10)," ")}\n{caption}\n');offset+=dur
concat.append(f"file '{root/'renders'/'09.png'}'")
(root/'timeline.txt').write_text('\n'.join(concat)+'\n');(root/'Swarm_graphe_LinkedIn_FR.srt').write_text('\n'.join(srt))
(root/'storyboard.json').write_text(json.dumps(scenes,ensure_ascii=False,indent=2))
subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',str(root/'timeline.txt'),'-t',str(offset),'-r','25','-c:v','libx264','-threads','2','-preset','fast','-crf','20','-pix_fmt','yuv420p','-movflags','+faststart',str(root/'Swarm_graphe_LinkedIn_FR.mp4')],check=True)
# README preview shows three useful actions: overview, folded branch, exact blocker.
(root/'preview.txt').write_text(''.join(f"file '{root/'renders'/f'{n:02}.png'}'\nduration 3\n" for n in [1,3,5])+f"file '{root/'renders'/'05.png'}'\n")
subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',str(root/'preview.txt'),'-t','9','-filter_complex','fps=2,scale=960:-1,split[a][b];[a]palettegen[p];[b][p]paletteuse','-loop','0',str(root/'Swarm_graphe_apercu_FR.gif')],check=True)
print(f'Built {offset}s, 1920x1080, 25fps, H264; matching SRT and nine actual UI captures.')
