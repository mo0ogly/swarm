# Brouillon LinkedIn Swarm identité mémoire et architecture

23 octobre 2026. Livrable éditorial local en français, non publié. / Local French
editorial draft, not published. No software behavior or shared contract changed;
an English article is not part of this French editorial request.

- **Version Word courante** : [article v6](Swarm_Article2_Identite_Memoire_v6.docx),
  avec les cinq schémas complets intégrés aux sections Rust, identité, mémoire,
  supervision et logs, au fil de l’article. Le texte, les 18 références et la
  page d’accès au film sont conservés. [Texte illustré v6](article-v6-fr.md) et
  [vérification du placement](word-v6-review.json).
  Les versions [v5](Swarm_Article2_Identite_Memoire_v5.docx),
  [v4](Swarm_Article2_Identite_Memoire_v4.docx) et
  [v3](Swarm_Article2_Identite_Memoire_v3.docx) restent figées avec leurs preuves.
  / **Current French Word**: v6, with all five complete views embedded alongside
  their corresponding article sections. Prose, 18 sources and film access retained.
- **Nouveau film** : [MP4 1 min 40](Swarm_Architecture_Film_v2.mp4), motion design
  français, musique originale, sans voix ; [sous-titres FR](video-v2/Swarm_Architecture_Film_v2.fr.srt)
  et [EN](video-v2/Swarm_Architecture_Film_v2.en.srt), [chapitres et sources](video-v2/README.md).
  / French motion-design film with original instrumental audio and FR/EN subtitles.
- [Article](article-fr.md), avec références publiques.
- [Vidéo MP4 précédente](Swarm_Supervision_Identite_Memoire.mp4) : présentation animée de
  deux minutes, 1080p, sous-titres français intégrés, sans voix ni capture du
  logiciel. [Sous-titres SRT](Swarm_Supervision_Identite_Memoire.srt) et
  [chapitres et sources de la vidéo](video/README.md).
- [Document Word](Swarm_Identite_Memoire_LinkedIn.docx), article illustré,
  références cliquables et note de préparation vidéo.
- Diagrammes Mermaid : [architecture](architecture.mmd) et [mémoire](memoire.mmd).
- Exports : [architecture PNG](architecture.png), [architecture SVG](architecture.svg),
  [mémoire PNG](memoire.png), [mémoire SVG](memoire.svg).
- [Storyboard et conditions de démonstration](storyboard-video-fr.md).
- Présentation du dépôt actualisée : [README FR](../../../README.md),
  [README EN](../../../README.en.md), [schémas et maturité](../../diagrams/README.md).
  [Relecture documentaire](readme-refresh-review.json). L’[aperçu vidéo](video/apercu.png)
  provient de l’image à 00:10 du MP4, sans modification du montage.


Les images sont à importer dans l’éditeur de publication ; le code Mermaid
reste modifiable dans les sources. Aucun collage de code Mermaid sur LinkedIn
n’est présenté comme une image déjà rendue. Les couleurs distinguent les rôles :
bleu contrôle, violet mémoire, vert bleuté exécution/preuves, orange travail à
construire. Les textes et pointillés portent aussi les limites ; la couleur
seule n’indique jamais une qualification.

Architecture détaillée consolidée après la reprise utilisateur : [FR](../../architecture/OVERVIEW.md), [EN](../../en/ARCHITECTURE.md), [revue de portée](architecture-followup-review.json).

## Introduction et sources de la version 4

La mémoire influence l’identité fonctionnelle et les décisions futures ; elle
ne crée pas l’identité authentifiée ni les droits de l’agent. Le chapitre relie
injection indirecte, mauvaise commande conservée, résumé transmis et reprise par
une autre instance. Les cinq planches actuelles conservent les branches de
validation humaine. Aucun nouveau contrôle du moteur n’est ajouté par ce Word.

Les repères publics couvrent MINJA, MemoryGraft, les recommandations commerciales
empoisonnées décrites par Microsoft, les trajectoires simulées de Yazdinejad et
Karimipour, MemGhost, Emergence World, AgentCorruption, le billet NIST sur
l’identité et un Internet Draft individuel. Les dates de publication et de
révision sont distinguées des reprises de presse. Le délai de 46 heures porte
sur la fin de la campagne simulée, pas une détection attribuée au même agent.
Le rapport Zenity conserve son durcissement observé en septembre et la réponse
AWS. Le dépôt dans le Datatracker ne constitue pas une norme IETF.

Une signature protège une origine et une intégrité sous ses hypothèses de
confiance ; elle ne rend pas un souvenir vrai ou inoffensif. Les idées de détection,
quarantaine et restauration sont conservées comme pistes à spécifier et éprouver,
avec propriétaire et condition de reprise dans la revue des sources, sans extension
ni démarrage implicite d’AC02. Une attribution précise au NIST ou à OWASP doit
s’appuyer sur le passage primaire concerné, pas sur un titre, un résumé ou un
hébergement institutionnel. Cette règle de relecture s’applique aux prochaines
révisions éditoriales ; elle ne modifie aucun contexte natif déjà démarré.

The v4 introduction separates functional identity, authenticated authority and
memory content. Public findings retain their dates, experimental limits and
remediation context. Future detection and quarantine mechanisms remain proposals;
no runtime behavior, Rust qualification or AC02 execution is claimed.

## Contrôle humain et portée de la version 3

L’accroche traite une instruction erronée ou injectée devenue mémoire persistante,
puis transmise à une autre instance. Le scénario est explicitement hypothétique ;
EchoLeak et MINJA sont référencés avec leurs limites respectives. Les cinq vues
reprennent les schémas complets de `docs/diagrams/`, sans simplification des objets
et relations. Le Word embarque des PNG haute résolution rendus depuis les SVG
inchangés ; le premier rendu SVG natif, dont les flèches étaient corrompues, est
conservé dans les preuves de relecture. Les captions des vues 1 et 4 explicitent la
validation humaine exigée par la politique.

La distinction déterminante est documentée dans
[ENGINE-ACCEPTANCE](../../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine)
et le [raccord Rust139](../../architecture/RUST-MIGRATION.md#tâches-ordinaires-et-validation-humaine--raccord139-en-qualification).
En mode `human`, un succès des contrôles produit des preuves et conserve l’attente
de décision humaine ; il ne libère pas les tâches suivantes. Une revue IA ne remplace
pas cette décision. Le contrôle humain fait partie du contrat de sécurité du moteur.
La qualification locale du contrôleur reste distincte de l’authentification humaine
protégée et du raccord global de l’identité.

Les [ajouts de consigne](../../architecture/INSTRUCTION-LEARNING.md), validés par
défaut et rejetables avant réservation, suivent un autre parcours. Leur existence
ne permet pas de déduire l’absence de validation humaine des résultats. Pour une
prochaine révision éditoriale, relire ces deux contrats et le contrôleur natif, en
plus du protocole de supervision. L’erreur de lecture et la correction utilisateur
restent attribuées dans `word-v3-review.json` ; aucun contexte d’agent embarqué ni
contrat du moteur n’est modifié par cet incrément éditorial.

Human result validation, AI process supervision, instruction revision and protected
human authentication are separate contracts. A successful control in `human` mode
does not replace the required human decision. No runtime qualification or AC02
execution is claimed by this French document edit.

## Mise à jour des README et schémas après la version 3

Les [README FR](../../../README.md) et [EN](../../../README.en.md), les vues
techniques et les exports Mermaid/SVG/PNG explicitent maintenant le contrôle
humain prévu par la politique `human`. Les cinq familles et tous les composants
antérieurs sont conservés. La [revue liée aux octets](readme-human-control-review.json)
distingue contrôles de documentation et qualification du moteur. La version Word3
reste un livrable figé avec ses propres empreintes, sans remplacement de ses
preuves historiques. / The entry points and current diagram exports now explicitly
retain policy-required human validation; Word v3 remains a frozen editorial artifact.

## Base factuelle et portée des affirmations

| Sujet | Sources locales inspectées | Ce que le brouillon peut affirmer | Limite |
|---|---|---|---|
| Supervision | [Conduite](../../architecture/RUN-SUPERVISION.md), résultats Rust [213](../../plans/migration-rust/evidence/20261022-root-instance-201/watch-public213/result.json), [224](../../plans/migration-rust/evidence/20261022-g3-watch-restart/public224/result.json), [244](../../plans/migration-rust/evidence/20261022-g3-watch-planning/public244/result.json) | Contrat Go observation/décision/application ; parcours CLI Rust de lancement, suivi, reprise queued et planification→worker vérifiés sur fixtures | Pas une supervision multi-agent globale acceptée ; autres rôles/web ouverts et répétition du binaire filmé nécessaire |
| Identité/autorité | [Contrat](../../architecture/AGENT-IDENTITY-CONTRACT.md), [replan](../../plans/actions-confiance/identites-agents-replan.md) | Contrats et composants internes, essais isolés historiques, délégation et révocation spécifiées | Pas de raccord global public, de source humaine ou de confinement fournisseur qualifié par ces documents |
| Mémoire | [Documentation](../../PROJECT-MEMORY.md), [usages](../../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md), `internal/engine/project_memory.go` | Socle historique Go versionné, sélection explicite, contexte reçu, limites du retrait | Droits d’usage globaux ouverts ; pas de promesse d’apprentissage autonome ni de parité Rust |
| Rust | [Plan actif](../../plans/migration-rust/PLAN.md), `rust/crates/swarm-engine/src/identity_plan.rs`, `rust/crates/swarm-engine/src/supervisor_identity.rs` | Migration en cours et parcours CLI ciblés documentés | G7 confiance, G8 mémoire et qualification web globale restent ouverts dans l’état consulté ; projection/digest/processus ne prouvent pas une autorité authentifiée |
| Sécurité future | [AC-02](../../plans/actions-confiance/securite-execution-apex.md) et ses références conservées | Architecture proposée et sources, sous-ensemble OpenShell visé, spécifications futures | A0–A7 non démarrés ; aucune sécurité AC-02 disponible affirmée |
| Vidéo | [Montage et sources](video/README.md), [storyboard](storyboard-video-fr.md) | MP4 explicatif de 120 s produit ; décodage des 3 600 images vérifié | Sans voix ni capture du logiciel ; démonstration de l’application encore à répéter et filmer |

Reprise utilisateur : la vidéo inclut désormais la supervision comme séquence
concrète, avec un scénario mixte d’environ deux minutes. Le Word et l’article
la présentent aussi ; la démonstration reste à répéter et filmer. Le minutage
est éditorial et ne promet aucune performance du moteur.

Les sources publiques citées établissent leurs concepts, pas la conformité
Swarm. NVIDIA v0.1.5 concerne OpenShell. Agent Baseline est un référentiel en
brouillon, sans certification de Swarm. Bromure fournit des références visuelles
et de parcours. LangChain fournit une classification de mémoire ; aucun usage
de sa bibliothèque dans Swarm n’est déduit de cette citation.

## Relecture avant publication ou tournage

Relire l’état réel au jour de publication, garder les qualificatifs de maturité,
vérifier lisibilité des images sur le support retenu et tester toute séquence
filmée sur une fixture isolée. La présente préparation ne publie rien et ne lance
aucun lot AC-02, test applicatif, build, fournisseur ou redémarrage.

L’enregistrement est distinct de l’acceptation éditoriale par l’utilisateur.
Les versions produit et `version_history.json` ne changent pas pour ce brouillon.
Les processus et contextes natifs déjà démarrés ne sont pas modifiés.
