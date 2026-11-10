# Swarm sécurité et standards pour LinkedIn

25 octobre 2026. Demande utilisateur : intégrer le positionnement dans la
documentation de Swarm et préparer un Word français à copier dans LinkedIn.
Livraison éditoriale locale, sans publication.

## Correction de la fidélité au texte fourni

La première livraison était incorrecte : un article réécrit de 1 355 mots avait
remplacé le texte de l’utilisateur. Sa vérification comparait le Word à cette
réécriture et ne démontrait donc pas la fidélité à la pièce jointe. L’utilisateur
a signalé la troncature ; le critère initial reste en échec dans la preuve
historique `review/review.json`, qui porte uniquement sur cette ancienne livraison.

**Livraison courante : article original intégral de 3 278 mots**, suivi en annexe
des 803 mots d’accompagnement également fournis. Le titre, la ponctuation, les
sections, les huit exigences détaillées, exemples et prises de position sont
conservés. Aucun post abrégé n’est placé avant l’article dans le Word corrigé.

- [Source fournie intégrale](source-integrale-fr.md), copie octet pour octet de la pièce jointe.
- [Article original](article-fr.md), tranche exacte de cette source du titre aux hashtags.
- [Accompagnement original](accompagnement-original-fr.md), préambule et recommandations conservés en annexe.
- [Word corrigé](Swarm_Securite_LinkedIn.docx), article en premier, accompagnement en annexe.
- Reprise, première livraison et contrôles de correction : `review/truncation-incident/` ; la preuve courante est `review/truncation-incident/verification.json` après contrôle.

Le positionnement FR/EN constitue une analyse distincte ; il n’est pas substitué
au texte de l’auteur. Le post court reste une proposition séparée et n’est ni
l’article original ni le livrable principal demandé. La règle de fidélité pour
les prochaines révisions est de comparer le contenu rendu à la pièce jointe
autorisée, et non uniquement à une nouvelle source éditoriale produite par nous.

La suite conserve la description et les preuves historiques de la première
livraison ; leurs compteurs de cinq pages et neuf liens ne décrivent pas le Word
corrigé. Aucune publication ni nouvelle qualification du moteur n’a été effectuée.

## Livrables et plan de vérification

- [Documentation de référence FR](../../SECURITY-POSITIONING.md) et [EN](../../en/SECURITY-POSITIONING.md), reliées aux README et aux matrices R19.
- [Post court](post-fr.txt), prévu pour un collage simple, et [article](article-fr.md), à la première personne de Fabrice Pizzi.
- [Word](Swarm_Securite_LinkedIn.docx) regroupant le post, l’article et les références publiques cliquables.
- Générateur maintenu [build_word.py](build_word.py) ; rendu et relecture dans `review/`, avec revue liée aux octets dans `review/review.json` après vérification.

Critères : couverture des références et de leurs statuts ; limites actuelles
Biba/identité/mémoire/AC-02 explicites ; huit exigences PA présentées comme
proposition ; parité documentaire FR/EN ; liens locaux vérifiés ; contenu Word
conforme aux sources ; rendu de toutes les pages inspecté ; diff propre sur
l’incrément possédé. Le post et l’article sont deux formats éditoriaux du même
contenu, pas deux qualifications de sécurité.

## Portée et décisions

La page centrale est ajoutée plutôt que de transformer R19 en prétendue
évaluation globale. R19 et ses preuves historiques restent inchangées dans
leur portée. Le profil PA-01–08 est proposé, sans nouvelle politique active,
norme adoptée, baseline sélectionnée ou démarrage implicite d’AC-02.

Les originaux des quatre fichiers préexistants modifiés sont conservés dans
`review/before/` avec leurs empreintes dans `review/before.json`. Le checkout
contient des travaux préexistants ; la revue de cet incrément compare ces
originaux, sans attribuer à cette demande le diff historique de la branche.

Les chemins de l’index et du build ne sont pas supposés : les recherches ont
constaté l’absence de `docs/index.md`, de `docs/en/index.md` et d’un build.rs web.
Les liens sont donc ajoutés aux README réels et aux matrices existantes. Le
chargement des ressources HTTP utilise les sources Rust et ressources web ;
aucune de celles-ci n’est modifiée par cet incrément documentaire. Aucun build,
redémarrage, changement de version produit ou note d’évolution logicielle n’est
nécessaire pour ces fichiers documentaires et éditoriaux non embarqués.

## Découvertes et reprise

| Origine | Valeur et périmètre | Propriétaire et critère observable | État |
| --- | --- | --- | --- |
| Texte utilisateur et README ACS | Éviter une revendication de nouveauté sur la séparation déterministe déjà décrite par ACS | Codex éditorial ; attribution explicite et absence de compatibilité annoncée sans essai | Vérifié dans la documentation et les textes éditoriaux |
| Plan BIBA-RUST du 25 octobre | Présenter les acquis Rust récents sans généralisation à toutes les missions | Codex éditorial ; portée `trust-actions`, simulation et limites conservées | Formulation vérifiée ; preuves runtime consignées, non rejouées ici |
| R19 existante | Éviter un nouveau catalogue concurrent et un score de conformité trompeur | Responsable de qualification ; prochaine évaluation avec références figées, applicabilité et preuves récentes | Profil documentaire proposé ; évaluation non exécutée |
| AC-02 enregistré | Rendre visible la frontière entre contrôle de transitions et confinement général | Responsable AC-02 ; reprise après qualification Rust et demande explicite de démarrage | Report historique conservé ; aucun lot lancé |

La qualification future devra relire les sources et l’état effectivement
installé, figer les références externes, conserver les témoins/refus/reprises
et obtenir l’évaluation indépendante requise. Ce livrable ne modifie aucun
contexte agent embarqué ou déjà transmis. La documentation persistante et ses
liens servent aux prochains lecteurs ; leur lecture par un processus déjà
démarré n’est pas établie.

English: local French editorial deliverable with bilingual reference documentation.
No runtime behavior, active policy, product version or historical qualification
changes. Proposed assurance requirements remain distinct from implementation,
verification and independent acceptance.

## Reproduction et relecture

Le générateur requiert Python et `python-docx` dans le runtime documentaire
fourni par Codex. Depuis la racine du dépôt, avec ce Python :

```sh
python3 docs/communication/linkedin-swarm-securite-20261025/build_word.py
```

Le rendu utilise `render_docx.py` de la skill documents et le LibreOffice
headless du même runtime. La vérification structurelle compare tous les
paragraphes aux sources et contrôle les neuf relations de lien. L’audit
d’accessibilité est complémentaire à la lecture des images de chaque page ;
il ne qualifie pas l’accessibilité de LinkedIn ni son comportement au collage.
Les PDF et PNG de `review/` sont des intermédiaires de contrôle, pas des
livrables de publication demandés.

Le premier rendu a révélé une bordure bleue héritée du style `Title`. Le
générateur la retire dans les styles source avant sérialisation. Un paragraphe
à cheval sur deux pages laissait une ligne isolée ; la conservation des
paragraphes est maintenant explicite. Les deux DOCX intermédiaires et leurs
rendus restent conservés, sans remplacer ces preuves par le résultat final.
Prévention : inspecter le style hérité et les coupures réelles, même si tous
les liens et contrôles structurels passent. Cette règle est portée par le
générateur et ce suivi ; aucun changement de méthode globale ou de contexte
natif embarqué n’est annoncé.
