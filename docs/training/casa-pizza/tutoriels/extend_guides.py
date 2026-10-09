"""Append verified animated tutorial lessons to the retained 36-chapter guides.
Run with --originals-dir pointing to the preserved original DOCX files.
"""
import argparse,json
from pathlib import Path
from docx import Document
from docx.shared import Inches,Pt,RGBColor
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.opc.constants import RELATIONSHIP_TYPE as RT
ROOT=Path(__file__).resolve().parent

def link(doc,label,target):
 p=doc.add_paragraph(); h=OxmlElement('w:hyperlink');h.set(qn('r:id'),p.part.relate_to(target,RT.HYPERLINK,is_external=True));r=OxmlElement('w:r');pr=OxmlElement('w:rPr');u=OxmlElement('w:u');u.set(qn('w:val'),'single');pr.append(u);r.append(pr);t=OxmlElement('w:t');t.text=label;r.append(t);h.append(r);p._p.append(h)
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--originals-dir',type=Path,required=True);args=ap.parse_args();data=json.loads((ROOT/'storyboard.json').read_text())
 for lang,name in [('fr','Formation_Swarm_Casa_Pizza.docx'),('en','Swarm_Casa_Pizza_Training_EN.docx')]:
  original=Document(args.originals_dir/name);d=Document(args.originals_dir/name)
  if any(p.text.startswith('37 ') for p in d.paragraphs):raise ValueError('Supply the original 36-chapter guide, not an already extended copy.')
  for p in d.paragraphs:
   for r in p.runs:
    if '8 octobre 2026' in r.text or '8 October 2026' in r.text:
     r.text=r.text.replace('8 octobre 2026','9 octobre 2026').replace('8 October 2026','9 October 2026')
  d.add_page_break();d.add_heading('37 Tutoriels animés pour refaire le parcours' if lang=='fr' else '37 Animated tutorials to repeat the journey',level=1)
  d.add_paragraph('Ce complément conserve les 36 chapitres précédents. Ouvrez le lecteur hors ligne, choisissez un module et reproduisez une étape dans votre atelier avant de regarder la suite.' if lang=='fr' else 'This addition preserves the previous 36 chapters. Open the offline reader, choose a module and repeat a step in your workshop before proceeding.')
  link(d,'Ouvrir le lecteur pas à pas et les vidéos' if lang=='fr' else 'Open the step reader and videos','tutoriels/index.html'+('?lang=en' if lang=='en' else ''))
  d.add_picture(str(ROOT/'media'/f'03-client-{lang}-poster.jpg'),width=Inches(6.1));d.add_paragraph('Légendes intégrées aux vidéos. Aucun son ni lancement automatique.' if lang=='fr' else 'Captions are burned into the videos. No audio or automatic playback.',style='Caption')
  for m,chap in zip(data['modules'],['8–11 et 17–20','21–26','27–30','31–34']):
   seconds=len(m['steps'])*6
   link(d,f'{m["title"][lang]} · {seconds} s · '+('chapitres ' if lang=='fr' else 'chapters ')+chap.replace(' et ',' and ' if lang=='en' else ' et '),'tutoriels/index.html'+('?lang=en' if lang=='en' else '')+'#'+m['id'])
  d.add_paragraph('Extraire entièrement le ZIP avant ouverture. Les MP4 peuvent aussi être ouverts directement dans tutoriels/media. Le GIF est un aperçu réduit ; le mode pas à pas garde chaque image à l’écran et la transcription fournit les explications.' if lang=='fr' else 'Extract the entire ZIP before opening. MP4 files can also be opened directly in tutoriels/media. GIFs are reduced previews; step mode holds each screenshot on screen and the transcript provides explanations.')
  d.add_page_break();d.add_heading('38 Exercices corrigés et portée des images' if lang=='fr' else '38 Answered exercises and evidence scope',level=1)
  d.add_paragraph('Mettre en pause, répondre, puis vérifier. Les corrigés sont également accessibles dans le lecteur.' if lang=='fr' else 'Pause, answer, then check. The reader also contains the answers.')
  for i,m in enumerate(data['modules']):
   d.add_heading(str(i+1)+' '+m['title'][lang],level=2)
   p=d.add_paragraph();p.add_run(m['question'][lang]).bold=True
   d.add_paragraph(m['answer'][lang])
  d.add_heading('Ce qui a réellement été enregistré' if lang=='fr' else 'What was actually recorded',level=2)
  d.add_paragraph('Le 9 octobre 2026, des actions ont été effectuées dans un cockpit et une application locale isolés. Les captures successives ont été assemblées en vidéos fixes et GIF avec légendes ; il ne s’agit pas d’une capture vidéo continue. Aucun fournisseur IA n’a été appelé pour cette démonstration.' if lang=='fr' else 'On 9 October 2026, actions were performed in an isolated cockpit and local application. Successive screenshots were edited into still videos and captioned GIFs; these are not continuous screencasts. No AI provider was called for this demonstration.')
  d.add_paragraph('Le besoin a été enregistré. Le plan pédagogique a été créé séparément par le script public, sans exécution des six tâches. Les politiques de contrôle ont été prévisualisées puis annulées. Le parcours client et les transitions restaurant ont réellement été joués avec des données fictives. Les onze tests de la référence ont réussi ; cela ne prouve pas une exécution autonome de la mission.' if lang=='fr' else 'Requirements were saved. The teaching plan was created separately by the public script, without executing its six tasks. Check policies were previewed and cancelled. The customer flow and restaurant transitions were actually performed using fictitious data. The eleven reference tests passed; this does not prove autonomous mission execution.')
  assert len(d.inline_shapes)==len(original.inline_shapes)+1
  for old,new in zip(original.paragraphs,d.paragraphs):
   assert new.text==old.text.replace('8 octobre 2026','9 octobre 2026').replace('8 October 2026','9 October 2026')
  d.save(ROOT.parent/name)
  print(name,len(d.paragraphs),'paragraphs;',len(d.inline_shapes),'images; original content preserved')
if __name__=='__main__':main()
