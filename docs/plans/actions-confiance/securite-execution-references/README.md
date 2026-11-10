# AC-02 — Références conservées / Retained references

23 octobre 2026. [Plan FR](../securite-execution-apex.md) ·
[Plan EN](../../../en/plans/actions-confiance/securite-execution-apex.md) ·
[Provenance et empreintes / Provenance and hashes](manifest.json).

**Plan enregistré, chantier non démarré / Recorded plan, project not started.**
Ces fichiers sont des références documentaires ; aucun n’est une ressource
runtime Swarm ou une politique adoptée. / These are documentary references,
not Swarm runtime assets or adopted policies.

## Quatre références Bromure / Four Bromure references

Source et attribution : Renaud Deraison, Bromure,
[OpenShell policies and network lineage](https://bromure.io/en/blog/openshell-policies-and-network-lineage),
21 octobre 2026, version présentée 5.4. Le manuel consulté annonce 5.0.0.
Ces versions ne sont pas une déclaration de version Swarm. / The article
presents 5.4; the consulted manual says 5.0.0. Neither is a Swarm version claim.

| Référence / Reference | Clair / Light | Sombre / Dark | Usage prévu / Intended use |
|---|---|---|---|
| Éditeur de politique / Policy editor | [Image](sandbox-policy-openshell-light.webp) | [Image](sandbox-policy-openshell-dark.webp) | YAML et formulaires d’une même politique / One policy through YAML and forms |
| Panneau des refus / Denials drawer | [Image](sandbox-drawer-blocked-light.webp) | [Image](sandbox-drawer-blocked-dark.webp) | Comprendre un refus et préparer une décision / Explain denials and prepare a decision |
| Chaîne d’une connexion / Connection lineage | [Image](lineage-denied-light.webp) | [Image](lineage-denied-dark.webp) | Relier processus, flux et décision / Link processes, flows and decisions |
| Timeline de sécurité / Security timeline | [Image](security-timeline-light.png) | [Image](security-timeline-dark.png) | Événements, filtres et accès aux preuves / Events, filters and evidence navigation |

Les six WebP sont les octets téléchargés de l’article, sans transformation.
Les deux PNG de timeline sont les images déjà examinées, extraites à **0,5 s**
des vidéos clair/sombre. L’affiche vidéo montre une chaîne et ne constitue pas
une quatrième vue distincte. / The six WebP files retain the downloaded bytes.
The timeline PNGs are the previously examined frames extracted at **0.5 s**
from the light/dark videos. Their poster duplicates the lineage view.

Le manifeste conserve URL, SHA-256, taille et transformation. Les deux vidéos
originales ne sont pas recopiées : environ 23 Mo ensemble ; URL, taille et SHA-256
restent consignés. Les PNG suffisent aux références visuelles retenues ici.
Aucun fichier temporaire n’est nécessaire pour consulter ces huit images. /
The manifest records URLs, SHA-256, size and transformations. Original videos
are not copied (about 23 MB combined); their URLs, sizes and hashes remain.
All eight retained images can be viewed without temporary files.

## Agent Baseline — texte et YAML / Article and YAML

Source figée : dépôt Agent Baseline, commit
`d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283`.
[Article original](https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/software-factory/software-factory.md),
*The Secure Software Factory: A Reference Architecture*, version 1,
13 octobre 2026. Auteurs : Eli Aleyner, Ranti Familusi, Allie Howe,
Ian Livingstone, William Manning, Andy McMahon, Ahmad Nassri et Aaron Stanley.
Licence du dépôt conservée : [CC BY 4.0](agentbaseline/LICENSE).

| Copie locale inchangée / Unchanged local copy | Rôle / Role |
|---|---|
| [Texte intégral / Full article](agentbaseline/software-factory.source.txt) | Texte source lu, annexes comprises ; figures non recopiées / Source text read, including appendices; figures not copied |
| [README amont / Upstream README](agentbaseline/upstream-readme.source.txt) | Explique les consommateurs et l’autorité des sources / Explains consumers and source authority |
| [boundaries.yaml](agentbaseline/boundaries.yaml) | Données des quatre frontières / Four-boundary presentation data |
| [levels.yaml](agentbaseline/levels.yaml) | Données des trois niveaux / Three-level presentation data |
| [controls.yaml](agentbaseline/controls.yaml) | Catalogue canonique, 35 contrôles, `1.0-draft` / Canonical 35-control catalogue |
| [Versionnement / Versioning](agentbaseline/versioning.source.txt) | Statuts, stabilité et historique des identifiants / Identifier status, stability and history |

Les extensions `.source.txt` préservent les octets des textes originaux sans
présenter leurs chemins de figures comme des ressources locales disponibles.
Les trois YAML restent des copies documentaires, sans chargement dans Swarm.
Le [plan, section 6](../securite-execution-apex.md) distingue catalogue,
correspondance de preuves Swarm et politique exécutable OpenShell. Les adaptations
et traductions sont dans le plan, jamais dans les copies amont. /
The `.source.txt` extensions preserve original text bytes without presenting
uncopied figures as available local resources. The YAML snapshots are not
loaded by Swarm. Section 6 separates the catalogue, Swarm evidence crosswalk
and executable OpenShell policy. Adaptations/translations live in the plan,
not in upstream snapshots.

## Vérification de l’enregistrement / Recording verification

La vérification porte sur liens locaux, copies, empreintes, quatre références
en deux thèmes, traduction et statut non démarré. Aucun test de confinement,
audit de conformité, build, fournisseur ou processus runtime n’est lancé pour
cet enregistrement. Les preuves de ces capacités restent à produire lors du
chantier autorisé. / Checks cover local links, copies, hashes, four references
in both themes, translation and not-started status. No confinement tests,
conformance audit, build, provider or runtime process is started by this save.
Capability evidence remains future work.

Conservation et transmission : liens depuis le suivi après bascule et les
architectures FR/EN. Cela rend les documents retrouvables par les prochains
contextes, sans modifier un processus ou contexte natif déjà démarré. /
Links from after-cutover tracking and FR/EN architecture expose these documents
to future contexts without changing already-running processes/native contexts.
