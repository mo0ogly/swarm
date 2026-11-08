# Guides F-12 FR et EN

Révision éditoriale du 30 octobre 2026. Source commune :
`PAEA_F12_Guide_Finance_build.py` et `PAEA_F12_editorial.json`.
Le JSON contient les ajouts bilingues relus après proposition GLM ; la remise
originale est conservée sous `revisions/20261030-f12/glm/`.

Depuis la racine du dépôt, avec Python et python-docx disponibles :

```bash
python3 docs/PAEA/PAEA_F12_Guide_Finance_build.py \
  --out /tmp/paea-f12-output \
  --reuse-diagrams-from docs/PAEA
```

Cette option conserve exactement les onze images de chaque DOCX de référence,
dans leur ordre. Une référence incomplète est refusée. Sans cette option, Graphviz
`dot` est nécessaire ; les données, couleurs et graphes des schémas sont inchangés.
Les dimensions d'insertion respectent désormais la largeur imprimable.
Le kit complet comprend les deux DOCX, les sources Mermaid, les images,
le générateur et le JSON éditorial. Il ne comprend pas de preuve de paiement.

Les pieds de page utilisent PAGE et NUMPAGES, évalués à la mise en page par
Word ou LibreOffice. Après modification, actualiser les champs puis exporter
par le moteur de document, sans capturer l'interface de prévisualisation.
La pagination peut varier selon les polices et le logiciel : les exports vérifiés
ici utilisent LibreOffice fourni par le runtime Codex 26.1017.22136.

La révision ajoute une synthèse, les limites des scénarios R1–R5, le protocole
de reproduction, une matrice des niveaux de preuve et des questions aux experts.
Elle corrige les blocs de code, les en-têtes répétés des tableaux et les encadrés
coupés. Les propriétés bancaires restent proposées ; aucune certification,
recette bancaire exécutée ou acceptation indépendante n'est ajoutée.

Les originaux et les preuves de révision sont sous `revisions/20261030-f12/`.
Les documents principaux corrigés gardent leur nom. Le manuel général
`PAEA_v0.3_English.docx` n'est pas modifié dans cet incrément.
Cette livraison porte exclusivement sur les documents ; aucun serveur Swarm
ne doit être reconstruit ou redémarré pour les consulter.

# F-12 guides in French and English

Editorial revision 30 October 2026. Both editions are generated from the same
Python source and reviewed bilingual JSON. The command above preserves all eleven
embedded pictures in each reference guide; omit the reuse option only when
Graphviz is available and regeneration is intended. The full kit includes the
DOCX files, diagram sources, pictures, generator and editorial JSON.

PAGE and NUMPAGES fields are evaluated by Word or LibreOffice during layout.
Update fields and export directly from the document renderer after changes;
do not capture a preview UI. Page counts depend on fonts and renderer. Verified
exports here use the bundled Codex runtime and contain 18 French / 17 English pages.

The revision adds an opening summary, reproducibility requirements, evidence
status distinctions and expert review questions. Code alignment, repeated table
headers and unsplit callouts are corrected. Existing diagram bytes are preserved.
Originals and revision evidence are retained in `revisions/20261030-f12/`.
This is a document delivery, not bank qualification, independent acceptance or a
Swarm runtime change. The general English handbook is outside this increment.

## Diffusion aux experts — 30 octobre 2026 / Expert outreach

Cinq messages personnalisés ont été envoyés à Inria Évaluation IA, Rock Lambros
(OWASP ACS), Buck Shlegeris, Edoardo Debenedetti et Philippe Mocquard (IFACI).
Le guide DAF français accompagne les messages Inria et IFACI ; le manuel anglais
et le guide CFO anglais accompagnent les trois autres. Les copies Gmail portent
SENT ; destinataires, textes et noms/tailles des pièces jointes ont été relus.
Cela ne prouve ni réception effective ni lecture ni validation scientifique.
La description GitHub et huit topics ont été publiés et relus. La licence reste
PolyForm Noncommercial 1.0.0 : présenter le dépôt comme public sous licence
non commerciale, sans le qualifier d’open source au sens OSI.

Five tailored emails were sent with the GitHub link and the appropriate French
or English documents. Sender-side SENT status, content and attachment names/sizes
were checked; delivery, reading and independent acceptance remain unverified.
The public GitHub description and eight topics were updated and read back.
The unchanged PolyForm Noncommercial license requires accurate noncommercial
wording. No code push, license change, runtime update or automated follow-up.

Envois, sources des contacts, limites et revue / Messages, contact sources,
limits and review: [rapport](../communication/paea-outreach-20261030/report.json).

Mise à jour du même jour : le serveur Inria a rejeté `evalia@inria.fr`
(`550 #5.1.0 Address rejected.`), malgré sa publication sur le site. Cet envoi
est donc en échec de livraison. Le dossier a été adressé une seule fois à
Jean-Michel Loubes, directeur du programme, via son adresse professionnelle
publiée dans le RNSR. Copie SENT, lien GitHub et PDF DAF vérifiés ; réception
et lecture restent inconnues. Les identifiants et sources figurent au rapport.

Same-day update: Inria rejected the published generic address. That original
message failed delivery. A separate message was submitted once to programme
director Jean-Michel Loubes at his officially listed professional address.
Sender-side content and attachment were checked; receipt and reading are unknown.
A published address and SENT status do not establish deliverability. Preserve
later delivery failures separately rather than overwriting the original send.
