"""Encode locally captured UI steps; no provider call, remote asset or live database."""
import hashlib,json,subprocess,tempfile,textwrap
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont,ImageOps
ROOT=Path(__file__).resolve().parent
FONT='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
def step(frame,fr,en,result_fr,result_en):
 return dict(frame=frame,action={'fr':fr,'en':en},result={'fr':result_fr,'en':result_en},seconds=6)
def module(id,fr,en,steps,qfr,qen,afr,aen):
 return dict(id=id,title={'fr':fr,'en':en},steps=steps,question={'fr':qfr,'en':qen},answer={'fr':afr,'en':aen})
MODULES=[
 module('01-plan','Du besoin au graphe','From requirements to the graph',[
 step('00-besoin-{lang}','Écrire puis enregistrer le besoin','Write and save the requirements','Périmètre, règles et preuves attendues sont explicites. Aucun appel IA ici.','Scope, rules and required evidence are explicit. No AI call here.'),
 step('13-graphe-{lang}','Ouvrir Conduite et Vue d’ensemble','Open Overview and Fit overview','Six tâches à faire ; les flèches relient prérequis et tâches dépendantes.','Six tasks remain to do; arrows connect prerequisites to dependent tasks.'),
 step('14-tache-{lang}','Cliquer sur Tests automatiques','Click the automatic tests task','La tâche attend Commande et suivi. Le détail montre critère, dépendances et absence de rapport.','The task waits for Order and tracking. Details show criteria, dependencies and no report.'),
 step('18-graphe-sombre','Comparer le même graphe en sombre','Compare the same graph in dark mode','Un changement de thème ne change ni dépendances ni état des tâches.','Changing the theme changes neither dependencies nor task status.')],
 'Quelle tâche doit précéder les tests ? Le graphe prouve-t-il une exécution ?','Which task precedes tests? Does the graph prove execution?',
 'Commande et suivi précède Tests automatiques. Non : les six tâches sont non exécutées, sans agent actif. Le plan a été créé par le script pédagogique, séparément du besoin enregistré.','Order and tracking precedes Automatic tests. No: all six tasks are unexecuted, with no active agent. The teaching script created this plan separately from the saved requirements.'),
 module('02-controles','Préparer les contrôles Swarm','Prepare Swarm checks',[
 step('15-humaine-{lang}','Tâches puis Configurer les validations','Tasks then Configure validation','La revue humaine reste disponible pour un jugement qualitatif.','Human review remains available for qualitative judgment.'),
 step('19-commande-{lang}','Choisir python3 et saisir un argument par ligne','Choose python3 and enter one argument per line','La commande est exacte : -W error::ResourceWarning -m unittest -v. Pas de shell implicite.','The exact command is -W error::ResourceWarning -m unittest -v. No implicit shell.'),
 step('16-controle-{lang}','Relier le contrôle au critère et régler le délai','Link the check to its criterion and set its timeout','Le délai affiché de 60 secondes est éditable ; il ne constitue pas une limite universelle.','The displayed 60-second timeout is editable; it is not a universal limit.'),
 step('17-apercu-{lang}','Examiner l’effet avant de confirmer','Review the effect before confirming','L’aperçu explique la portée et l’invalidation des anciennes preuves. Nous avons annulé.','The preview explains scope and invalidation of old evidence. We cancelled.')],
 'Une commande réussie suffit-elle à déclarer la mission terminée ?','Does a successful command mean the mission is complete?',
 'Non. Elle apporte une preuve sur les fichiers et critères liés au candidat. La recette visuelle, la revue et les conditions d’acceptation restent distinctes. Dans cet enregistrement, aucune politique n’a été appliquée et aucune tâche n’a été acceptée.','No. It provides evidence for the files and criteria linked to the candidate. Visual testing, review and acceptance conditions remain distinct. No policy was applied and no task was accepted in this recording.'),
 module('03-client','Commander et traiter un refus','Order and handle a rejection',[
 step('01-catalogue','Ouvrir le catalogue local','Open the local catalogue','Six recettes disponibles ; atelier fictif, sans paiement réel.','Six recipes are available; fictitious workshop with no real payment.'),
 step('02-filtre','Cocher Végétariennes uniquement','Check Vegetarian only','Trois recettes restent visibles.','Three recipes remain visible.'),
 step('03-minimum','Commander une Margherita avec des coordonnées fictives','Order one Margherita with fictitious contact details','9,50 euros hors livraison : le serveur refuse le minimum inférieur à 10 euros.','EUR 9.50 excluding delivery: the server rejects an order below EUR 10.'),
 step('04-quantite','Passer la quantité à deux avec Flèche haut','Use Arrow up to set the quantity to two','19,00 euros de pizzas + 2,90 euros de livraison = 21,90 euros.','EUR 19.00 for pizzas + EUR 2.90 delivery = EUR 21.90.'),
 step('05-confirmee','Confirmer puis ouvrir Mon suivi','Confirm then open My tracking','Un numéro de commande apparaît. Le statut initial est Reçue.','An order number appears. The initial status is Received.')],
 'Pourquoi 12,40 euros livraison comprise ne suffisent-ils pas pour une pizza ?','Why is EUR 12.40 including delivery insufficient for one pizza?',
 'Le minimum s’applique aux pizzas seules : 9,50 euros reste inférieur à 10 euros. La livraison de 2,90 euros ne compte pas dans ce minimum.','The minimum applies only to pizzas: EUR 9.50 is below EUR 10. The EUR 2.90 delivery charge does not count towards that minimum.'),
 module('04-restaurant','Du restaurant au suivi client','From restaurant to customer tracking',[
 step('06-connexion','Ouvrir Espace restaurant','Open the restaurant area','Une connexion est nécessaire pour consulter et faire avancer les commandes.','Signing in is required to view and advance orders.'),
 step('07-refus-acces','Essayer un mot de passe incorrect','Try an incorrect password','L’accès est refusé ; le catalogue public reste distinct de l’espace restaurant.','Access is rejected; the public catalogue remains separate from the restaurant area.'),
 step('08-commande-recue','Se connecter avec atelier dans cette démo','Sign in with atelier in this demo','La même commande et le même total sont affichés côté restaurant.','The restaurant shows the same order and total.'),
 step('09-preparation','Passer à En préparation','Move to Preparing','Le restaurant avance manuellement le statut ; aucune autonomie IA n’est démontrée.','The restaurant advances the status manually; no AI autonomy is demonstrated.'),
 step('10-suivi-preparation','Actualiser le suivi côté client','Refresh tracking on the customer side','Le statut En préparation correspond à l’action du restaurant.','Preparing matches the restaurant action.'),
 step('11-livraison','Passer à En livraison','Move to Out for delivery','Les étapes sont parcourues dans l’ordre.','The steps are traversed in order.'),
 step('12-livree','Passer à Livrée puis actualiser le client','Move to Delivered then refresh the customer','Le suivi confirme la livraison fictive du même numéro de commande.','Tracking confirms fictitious delivery of the same order number.')],
 'Qui a fait avancer la commande ? Comment vérifier que c’est la même ?','Who advanced the order? How can you check it is the same one?',
 'Le formateur, via les boutons du restaurant. Comparer numéro, contenu et total entre restaurant et client. Le parcours ne démontre ni livraison réelle ni agent autonome.','The instructor used the restaurant buttons. Compare order number, contents and total between restaurant and customer. This journey demonstrates neither real delivery nor an autonomous agent.')]

def lines(text,font,width):
 words=text.split(); out=[]; row=''
 for word in words:
  candidate=(row+' '+word).strip()
  if font.getlength(candidate)>width and row:out.append(row);row=word
  else:row=candidate
 return out+[row]

def card(mod,st,lang,n):
 frame=st['frame'].replace('{lang}',lang)+'.png'; im=Image.open(ROOT/'frames'/frame).convert('RGB')
 canvas=Image.new('RGB',(1280,900),'#f7f5ef'); dr=ImageDraw.Draw(canvas)
 bold=ImageFont.truetype(FONT,24); small=ImageFont.truetype(FONT,18); body=ImageFont.truetype(FONT,23)
 dr.rectangle((0,0,1280,72),fill='#17392f');dr.text((24,12),'CASA PIZZA  /  '+mod['title'][lang],font=bold,fill='white')
 dr.text((24,43),('Captures réelles montées · sans audio' if lang=='fr' else 'Edited real screenshots · no audio'),font=small,fill='#dbe9e1')
 # Crop tall client captures to the relevant area; preserve proportions.
 if im.height>900:
  top={'01-catalogue':0,'02-filtre':100,'03-minimum':450,'04-quantite':450,'05-confirmee':480,'10-suivi-preparation':480,'12-livree':480}.get(st['frame'],0)
  crop_height=round(im.width*712/1264);top=min(top,im.height-crop_height);im=im.crop((0,top,im.width,top+crop_height))
 im=ImageOps.contain(im,(1264,712));canvas.paste(im,((1280-im.width)//2,76+(712-im.height)//2));dr.rectangle((0,790,1280,900),fill='#17392f')
 dr.text((22,802),f'{n+1:02d}/{len(mod["steps"]):02d}   '+st['action'][lang],font=bold,fill='white')
 for i,line in enumerate(lines(st['result'][lang],body,1228)):
  dr.text((22,837+27*i),line,font=body,fill='#dbe9e1')
 # Editorial progress bar. It does not masquerade as a captured pointer.
 dr.rectangle((0,897,int(1280*(n+1)/len(mod['steps'])),900),fill='#e6a474')
 return canvas

def timestamp(n):return f'{n//3600:02d}:{n//60%60:02d}:{n%60:02d}.000'

def main():
 data={'recorded':'2026-10-09','method':'Real UI screenshots captured during browser interactions; edited still sequences, not continuous screencasts. No AI provider invoked.','modules':MODULES}
 (ROOT/'storyboard.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')
 (ROOT/'media').mkdir(exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='encode-',dir=ROOT) as td:
  td=Path(td)
  for mod in MODULES:
   for lang in ('fr','en'):
    cards=[]; cues=['WEBVTT','']; transcript=[]
    for i,st in enumerate(mod['steps']):
     img=card(mod,st,lang,i); f=td/f'{mod["id"]}-{lang}-{i}.png';img.save(f);cards.append(f)
     cues += [f'{timestamp(i*6)} --> {timestamp((i+1)*6)}',st['action'][lang],st['result'][lang],'']
     transcript.append(f'{i*6:02d}s — {st["action"][lang]}\n{st["result"][lang]}')
    listing=td/'concat.txt';listing.write_text(''.join(f"file '{f}'\nduration 6\n" for f in cards)+f"file '{cards[-1]}'\n")
    output=ROOT/'media'/f'{mod["id"]}-{lang}.mp4'
    subprocess.run(['ffmpeg','-nostdin','-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',str(listing),'-t',str(len(cards)*6),'-vf','fps=12','-c:v','libx264','-preset','fast','-crf','26','-pix_fmt','yuv420p','-movflags','+faststart',str(output)],check=True)
    Image.open(cards[0]).resize((960,675)).save(ROOT/'media'/f'{mod["id"]}-{lang}-poster.jpg',quality=85)
    # Small preview, captioned; full video and step-by-step reader remain primary.
    previews=[card(mod,st,lang,i).resize((640,450)).quantize(colors=96) for i,st in enumerate(mod['steps'])]
    previews[0].save(ROOT/'media'/f'{mod["id"]}-{lang}.gif',save_all=True,append_images=previews[1:],duration=2200,loop=0,optimize=True)
    (ROOT/'media'/f'{mod["id"]}-{lang}.vtt').write_text('\n'.join(cues))
    (ROOT/'media'/f'{mod["id"]}-{lang}.txt').write_text('\n\n'.join(transcript)+'\n')
 (ROOT/'manifest.json').write_text(json.dumps({'recorded':data['recorded'],'method':data['method'],'files':[{'path':str(p.relative_to(ROOT)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in sorted(list((ROOT/'frames').glob('*.png'))+list((ROOT/'media').glob('*')))]},indent=2)+'\n')
 print('Encoded 8 videos, 8 GIF previews, 8 posters and bilingual transcripts/subtitles.')
if __name__=='__main__':main()
