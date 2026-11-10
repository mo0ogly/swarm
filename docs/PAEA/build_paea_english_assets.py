from pathlib import Path
from docx import Document
from docx.shared import Inches,Pt
from docx.enum.text import WD_ALIGN_PARAGRAPH
import re, zipfile, subprocess, json
root=Path('/mnt/data'); src=root/'paea_en_diagrams/mermaid'; out=root/'PAEA_v0.3_Mermaid_EN';out.mkdir(exist_ok=True)
translations={
'01_risque': ['Supplier invoice PDF<br/>untrusted source','Inter-agent summary<br/>derived content','Payment proposal<br/>no authority','Authorization<br/>mandate + approval','Sink commitment<br/>observable effect'],
'02_risque':['Business objective','Reachable effects<br/>read, write, A2A','Consequences<br/>impact + accumulation','Limits<br/>reversibility + oversight','Consequence class<br/>C0…C4','Delegation decision<br/>with rationale'],
'03_architecture':['Authorized business owner<br/>mandate','Isolated agents<br/>proposals','Authenticated broker<br/>verified identity','Rules guardian<br/>PDP','Approval<br/>if required','Keys guardian<br/>bounded capability','Enforcement gate<br/>PEP / connector','Business service<br/>sink + oracle'],
'04_flux':['External data<br/>PDF, web, MCP','Labels and provenance<br/>untrusted','LLM analysis<br/>proposal','Command schema<br/>defined by trusted authority','Authority check<br/>no promotion','Protected action<br/>if authorized'],
'05_memoire':['Observed fact<br/>source + date','Proposed lesson<br/>no authority','Admission / quarantine<br/>policy + checks','Stored memory version<br/>trace and scope','Authorized read<br/>for defined task','Prepared context<br/>digest / receipt'],
'06_autorite':['SPIFFE / OIDC<br/>who is it?','Mandate<br/>what and for whom?','Attenuated delegation<br/>fewer privileges','Sealed capability<br/>one action, one sink','Sink verifies<br/>audience + epoch','Observed effect'],
'07_canon':['Verified invoice<br/>version 7','Canonicalization<br/>EUR · 420000 cents','Digest<br/>version + parameters','Human approval<br/>exact digest','Sink: final check<br/>equality + mandate','COMMITTED<br/>or DENIED'],
'08_unknown':['Intention + controls','Request committed<br/>response lost','UNKNOWN<br/>no new payment','Reconciliation<br/>query sink status','Already committed<br/>COMMITTED','Proven not committed<br/>new decision possible'],
'09_revocation':['Active mandate<br/>epoch 19','Authorization<br/>for epoch 19','Revocation recorded<br/>epoch 20','Sink rereads epoch<br/>before commit','Old capability denied'],
'10_contournement':['Agent + descendants<br/>effective privileges','Files / env / FDs<br/>secret access forbidden','Direct network / DNS<br/>forbidden','Private socket / other tool<br/>forbidden','Broker + proxy<br/>only approved route','Effect service<br/>independent ledger'],
'11_tests':['Adversarial scenario<br/>preconditions + fault','Pinned version<br/>TCB / policies / data','Execution and attack<br/>original traces','Sink-side oracle<br/>before / after state','Outcome<br/>supported / refuted','Evidence package<br/>limits + statistics'],
'12_deploiement':['Business scope<br/>and criticality','TCB + paths<br/>inventoried','No-real-effect pilot<br/>fault injection','Independent review<br/>counterexamples','Business decision<br/>deploy / restrict / reject','Monitoring + requalification<br/>controlled changes'],
'13_approbation':['Proposal P<br/>EUR 4,200.00','Human display<br/>beneficiary + context','Authenticated approver<br/>four-eyes principle','Sign exact digest<br/>420000 cents','IBAN or amount<br/>modification','Sink revalidates<br/>rejects discrepancy']}
edge={'provenance':'provenance','contexte':'context','contrôle':'check','barrière':'enforcement','inventaire':'inventory','analyse':'analysis','stratégie':'strategy','décision':'decision','validation':'approval','mandat':'mandate','intention':'intent','évaluer':'evaluate','si nécessaire':'when required','approbation':'approval','capacité':'capability','état courant':'current state','labelliser':'label','valeurs non fiables':'untrusted values','structure de confiance':'trusted structure','extraire':'extract','soumettre':'submit','admettre':'admit','sélection':'selection','préparer':'prepare','lier':'bind','déléguer':'delegate','autoriser':'authorize','présenter':'present','engager':'commit','résoudre':'resolve','encoder':'encode','preuve':'evidence','envoyer':'send','perte réponse':'lost response','interroger':'query','preuve positive':'positive evidence','preuve négative':'negative evidence','délivrer':'issue','action en vol':'in-flight action','ordre autoritatif':'authoritative ordering','epoch périmé':'stale epoch','route admise':'approved route','effet':'effect','fixer':'pin','lancer':'execute','observer':'observe','comparer':'compare','archiver':'archive','concevoir':'design','tester':'test','prouver':'demonstrate','autoriser ?':'authorize?','exploiter':'operate','expliquer':'explain','signer':'sign','conflit':'conflict','TEST':'TEST','commit':'commit','permit':'permit'}
for f in sorted(src.glob('*.mmd')):
 s=f.read_text(); name=f.stem
 labels=translations[name];nodes=list(re.finditer(r'(?m)^\s*([A-Z])\["(.*?)"\]',s))
 assert len(nodes)==len(labels),(name,len(nodes),len(labels))
 for m,new in reversed(list(zip(nodes,labels))):
  s=s[:m.start(2)]+new+s[m.end(2):]
 for old,new in edge.items():s=s.replace('"'+old+'"','"'+new+'"')
 (out/f.name).write_text(s)
 # produce Graphviz rendered png from nodes/edges and styles
 nodes2=re.findall(r'(?m)^\s*([A-Z])\["(.*?)"\]',s)
 edges=re.findall(r'(?m)^\s*([A-Z])\s*-->\s*(?:\|"?[\w ?-]+"?\|\s*)?([A-Z])',s)
 # Mermaid edge labels encoded as -->|"text"| ; alternative
 edges=re.findall(r'(?m)^\s*([A-Z])\s*-->.*?([A-Z])\s*$',s)
 colors={'source':'#F7F0E1','agent':'#E8EDF0','rule':'#DDECEB','keys':'#E3EDF3','gate':'#FCEBE6','effect':'#DFEEE3','refuse':'#F9DDD7'}
 classes=dict(re.findall(r'(?m)^\s*class ([A-Z]) (\w+)',s))
 dot=['digraph G {','graph [rankdir=LR,bgcolor="white",pad="0.3",nodesep="0.35",ranksep="0.6",dpi=160];','node [shape=box,style="rounded,filled",fontname="DejaVu Sans",fontsize=12,penwidth=1.5,margin="0.24,0.18"];','edge [color="#536D7C",penwidth=1.5,arrowsize=0.7];']
 for ident,l in nodes2: dot.append(f'{ident} [label={json.dumps(l.replace("<br/>","\\n"))},fillcolor="{colors.get(classes.get(ident),"#F8F8F8")}",color="#5A7787"];')
 for a,b in edges:dot.append(f'{a} -> {b};')
 dot.append('}')
 (out/(name+'.dot')).write_text('\n'.join(dot));subprocess.run(['dot','-Tpng',str(out/(name+'.dot')),'-o',str(out/(name+'.png'))],check=True)
zipdest=root/'PAEA_v0.3_Mermaid_EN.zip'
with zipfile.ZipFile(zipdest,'w',zipfile.ZIP_DEFLATED) as z:
 for f in sorted(out.glob('*.mmd')):z.write(f,arcname='mermaid/'+f.name)
# preserve existing English adaptation and append translated images with accurate note
base=Document(root/'PAEA_v0.3_English_Methodological_Guide.docx')
base.add_page_break();base.add_heading('Appendix — Thirteen translated process diagrams',level=1)
base.add_paragraph('The diagrams below preserve the logical structure and color categories of the French Mermaid originals. Editable Mermaid sources accompany this document. This English manuscript is a condensed adaptation, not a full translation of the 25,700-word French handbook.')
for f in sorted(out.glob('*.png')):
 p=base.add_paragraph();p.alignment=WD_ALIGN_PARAGRAPH.CENTER
 p.add_run().add_picture(str(f),width=Inches(6.1))
 cap=base.add_paragraph(f.stem.replace('_',' ').capitalize());cap.style='Caption'
base.save(root/'PAEA_v0.3_English_Guide_With_13_Diagrams.docx')
print('created',zipdest,root/'PAEA_v0.3_English_Guide_With_13_Diagrams.docx')
