#!/usr/bin/env python3
"""Reconstruction reproductible de PAEA v0.3 depuis le manuscrit et les Mermaid.

Usage:
    python PAEA_v0.3_build.py --source PAEA_v0.3_Framework_Methodologique_Complete.docx \
      --mermaid PAEA_v0.3_Sources_Mermaid_Couleur.zip --out PAEA_v0.3_Edition_Lisible.docx

Nécessite python-docx et Pillow. Les diagrammes déjà embarqués sont conservés.
Les sources Mermaid restent disponibles dans le ZIP fourni séparément.
"""
import argparse
import re
import zipfile
import shutil
import subprocess
import tempfile
from pathlib import Path
from docx import Document
from docx.shared import Cm, Pt, RGBColor
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.enum.table import WD_TABLE_ALIGNMENT, WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

NAVY='17344B'; TEAL='14667A'; DARK='26313A'; MUTED='4A6170'; LIGHT='EEF4F6'; CODE='F1F5F7'; SAND='F6F1E7'; WHITE='FFFFFF'

def set_border(p, color='BDD1DA', size=8):
    ppr=p._p.get_or_add_pPr(); borders=ppr.find(qn('w:pBdr'))
    if borders is None:
        borders=OxmlElement('w:pBdr');ppr.append(borders)
    bot=OxmlElement('w:bottom');bot.set(qn('w:val'),'single');bot.set(qn('w:sz'),str(size));bot.set(qn('w:color'),color);borders.append(bot)

def shade(p, fill):
    ppr=p._p.get_or_add_pPr(); el=ppr.find(qn('w:shd'))
    if el is None: el=OxmlElement('w:shd');ppr.append(el)
    el.set(qn('w:fill'),fill)

def textreplace(p, old, new):
    if old not in p.text:return
    for r in p.runs:
        if old in r.text:r.text=r.text.replace(old,new);return
    # fallback when phrase spans runs
    p.text=p.text.replace(old,new)

def style_setup(d):
    sec=d.sections[0];sec.top_margin=Cm(2.0);sec.bottom_margin=Cm(1.9);sec.left_margin=Cm(2.4);sec.right_margin=Cm(2.4)
    normal=d.styles['Normal'];normal.font.name='Liberation Serif';normal.font.size=Pt(11);normal.font.color.rgb=RGBColor.from_string(DARK)
    normal.paragraph_format.line_spacing=1.23;normal.paragraph_format.space_after=Pt(7)
    for nm in ['Lecture','Intertitre','Code PAEA','Légende figure','Étude de cas','Repère']:
        if nm not in d.styles:d.styles.add_style(nm,WD_STYLE_TYPE.PARAGRAPH)
    st=d.styles['Lecture'];st.font.name='Liberation Serif';st.font.size=Pt(11.15);st.font.color.rgb=RGBColor.from_string(DARK)
    st.paragraph_format.line_spacing=1.24;st.paragraph_format.space_after=Pt(8);st.paragraph_format.widow_control=True
    for nm,size,before,after in [('Heading 1',19,23,12),('Heading 2',15,18,9),('Heading 3',12,13,6)]:
        s=d.styles[nm];s.font.name='Liberation Sans';s.font.size=Pt(size);s.font.bold=True;s.font.color.rgb=RGBColor.from_string(NAVY if nm=='Heading 1' else TEAL)
        s.paragraph_format.space_before=Pt(before);s.paragraph_format.space_after=Pt(after);s.paragraph_format.keep_with_next=True
    s=d.styles['Intertitre'];s.font.name='Liberation Sans';s.font.size=Pt(10.5);s.font.bold=True;s.font.color.rgb=RGBColor.from_string(TEAL)
    s.paragraph_format.space_before=Pt(13);s.paragraph_format.space_after=Pt(5);s.paragraph_format.keep_with_next=True
    s=d.styles['Code PAEA'];s.font.name='DejaVu Sans Mono';s.font.size=Pt(8.5);s.font.color.rgb=RGBColor.from_string(NAVY)
    s.paragraph_format.left_indent=Cm(.48);s.paragraph_format.right_indent=Cm(.35);s.paragraph_format.space_after=Pt(0);s.paragraph_format.line_spacing=1.18
    s=d.styles['Légende figure'];s.font.name='Liberation Sans';s.font.size=Pt(9);s.font.italic=True;s.font.color.rgb=RGBColor.from_string(MUTED)
    s.paragraph_format.space_before=Pt(3);s.paragraph_format.space_after=Pt(13)
    s=d.styles['Étude de cas'];s.font.name='Liberation Serif';s.font.size=Pt(12);s.font.italic=True;s.font.color.rgb=RGBColor.from_string(NAVY)
    s.paragraph_format.left_indent=Cm(.45);s.paragraph_format.space_before=Pt(10);s.paragraph_format.space_after=Pt(11)
    s=d.styles['Title'];s.font.name='Liberation Sans';s.font.size=Pt(31);s.font.bold=True;s.font.color.rgb=RGBColor.from_string(NAVY)
    s=d.styles['Repère'];s.font.name='Liberation Sans';s.font.size=Pt(9);s.font.bold=True;s.font.color.rgb=RGBColor.from_string(TEAL)

def format_table(table):
    table.alignment=WD_TABLE_ALIGNMENT.CENTER
    for j,row in enumerate(table.rows):
        for cell in row.cells:
            cell.vertical_alignment=WD_CELL_VERTICAL_ALIGNMENT.CENTER
            tcPr=cell._tc.get_or_add_tcPr()
            shd=tcPr.find(qn('w:shd'))
            if shd is None:shd=OxmlElement('w:shd');tcPr.append(shd)
            shd.set(qn('w:fill'),NAVY if j==0 else ('F1F6F8' if j%2 else WHITE))
            for p in cell.paragraphs:
                p.paragraph_format.space_before=Pt(3);p.paragraph_format.space_after=Pt(3)
                for r in p.runs:
                    r.font.name='Liberation Sans';r.font.size=Pt(8.8);r.font.color.rgb=RGBColor.from_string(WHITE if j==0 else DARK)
                    if j==0:r.font.bold=True

def format_recipes(d):
    recipe=False
    for p in d.paragraphs:
        if re.match(r'15\.[2-6] Recette détaillée',p.text):recipe=True
        elif recipe and p.style.name.startswith('Heading'):
            recipe=False
        if not recipe or p.style.name != 'Lecture':continue
        for prefix,fill in [('Précondition.',LIGHT),('Attaque.',SAND),('Oracle.',LIGHT),('Mesure supplémentaire.',LIGHT),('Contre-exemple utile.',SAND),('Variante négative indispensable.',SAND),('Variante positive.',LIGHT)]:
            if p.text.startswith(prefix):
                shade(p,fill);p.paragraph_format.left_indent=Cm(.38);p.paragraph_format.right_indent=Cm(.25)
                p.paragraph_format.space_before=Pt(5);p.paragraph_format.space_after=Pt(6)
                if p.runs and p.runs[0].text.startswith(prefix):p.runs[0].bold=True
                break

def revise_manuscript(d):
    replacements={
      'Édition consolidée, augmentée et pédagogique ; fondée sur les versions v0.2 (spécification scientifique et guide opérationnel).':'Méthode de conception, de contrôle et de qualification des processus agentiques à conséquences.',
      'Ce qui change par rapport aux v0.2':'Principes de conception retenus',
      'La spécification scientifique v0.2 disposait d’invariants et d’un plan d’évaluation, mais les expliquait trop souvent comme une liste de contraintes.':'Une méthode d’assurance doit relier ses exigences à des décisions de conception, à des mécanismes exécutables et à des preuves indépendantes. Les propriétés de sécurité sont présentées ci-dessous au moment où elles deviennent utiles à la démarche.',
      'La v0.3 introduit trois idées éditoriales et méthodologiques nouvelles':'Trois compléments méthodologiques structurent cette proposition',
    }
    for p in d.paragraphs:
        for old,new in replacements.items():textreplace(p,old,new)
        # No development-history narration: publication should read on its own
        if p.style.name=='Lecture' and ('v0.2' in p.text or 'versions v0.2' in p.text):
            textreplace(p,'v0.2','propositions antérieures')
        if p.style.name=='Code PAEA':
            shade(p,CODE)
            p.paragraph_format.keep_together=True
        if p.style.name=='Étude de cas':shade(p,SAND)
        if p.style.name=='Intertitre':set_border(p,'AED0D7',5)
    format_recipes(d)

def append_generator_note(d, mermaid_zip):
    d.add_paragraph('Note de reproductibilité des figures',style='Heading 2')
    p=d.add_paragraph(style='Lecture');p.add_run('Les treize figures colorées sont intégrées dans ce document. ').bold=True
    p.add_run('Leurs sources Mermaid (.mmd) sont fournies dans une archive distincte, avec ce script Python de composition. Les figures sont conservées dans le fichier Word même sans installer Mermaid. Pour les modifier, éditer les fichiers .mmd, régénérer les visuels avec Mermaid CLI puis mettre à jour les figures dans le manuscrit source avant de relancer le script.')
    p=d.add_paragraph(style='Lecture');p.add_run('Exemple de commande : ')
    p.add_run('mmdc -i mermaid/03_architecture.mmd -o architecture.png -b white').italic=True
    p=d.add_paragraph(style='Lecture');p.add_run('L’archive de référence contient '+str(len([n for n in zipfile.ZipFile(mermaid_zip).namelist() if n.endswith('.mmd')]))+' fichiers Mermaid. Les contrôles de sûreté restent liés aux exemples et au modèle de menace : aucune figure ne constitue, à elle seule, une preuve d’implémentation.')

def regenerate_mermaid(d, zip_path, enabled=False):
    """Optionally rerender Mermaid sources and replace embedded Word figures in order.

    Mermaid CLI is optional: npm install -g @mermaid-js/mermaid-cli
    Without it, previously embedded full-color diagrams are preserved.
    """
    if not enabled:return
    command=shutil.which('mmdc')
    if not command:raise SystemExit('Mermaid CLI (mmdc) not installed. Use without --render-mermaid, or install it.')
    with tempfile.TemporaryDirectory() as t:
        with zipfile.ZipFile(zip_path) as z:
            sources=sorted(x for x in z.namelist() if x.endswith('.mmd'))
            if len(sources)!=len(d.inline_shapes):
                raise SystemExit(f'Diagram count mismatch: {len(sources)} sources / {len(d.inline_shapes)} figures')
            for i,source in enumerate(sources):
                inp=Path(t)/f'{i:02d}.mmd';out=Path(t)/f'{i:02d}.png'
                inp.write_bytes(z.read(source))
                subprocess.run([command,'-i',str(inp),'-o',str(out),'-b','white','-w','1500'],check=True)
                rel_id,_=d.part.get_or_add_image(str(out))
                blip=d.inline_shapes[i]._inline.xpath('.//a:blip')[0]
                blip.set(qn('r:embed'),rel_id)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',default='PAEA_v0.3_Framework_Methodologique_Complete.docx')
    parser.add_argument('--mermaid',default='PAEA_v0.3_Sources_Mermaid_Couleur.zip')
    parser.add_argument('--out',default='PAEA_v0.3_Edition_Lisible.docx')
    parser.add_argument('--render-mermaid',action='store_true',help='Recompiler les Mermaid avec mmdc et remplacer les 13 figures dans Word')
    a=parser.parse_args()
    d=Document(a.source);style_setup(d);revise_manuscript(d)
    for t in d.tables:format_table(t)
    append_generator_note(d,a.mermaid)
    regenerate_mermaid(d,a.mermaid,a.render_mermaid)
    d.core_properties.title='PAEA v0.3 — Méthode d’assurance de l’exécution agentique'
    d.core_properties.subject='Guide technique, schémas Mermaid, recettes de qualification et annexes pédagogiques'
    d.save(a.out)
    print('Créé :',a.out,'; figures :',len(d.inline_shapes),'; tableaux :',len(d.tables))
if __name__=='__main__':main()
