# Architecture interne — sources et exports / Internal architecture — sources and exports

Les cinq vues historiques et trois vues supplémentaires sont les mêmes en français et en anglais. Le cœur est le
**moteur déterministe Rust** ; l’identité, la mémoire, la supervision et les logs
restent des sous-systèmes Swarm décrits par leurs contrats internes. Leur maturité
native se consulte séparément dans l’[architecture FR](../architecture/OVERVIEW.md)
et l’[architecture EN](../en/ARCHITECTURE.md).

The five historical views and three additional views match across French and English. Their core is the
**deterministic Rust engine**; identity, memory, supervision and logs retain their
Swarm internal contracts. Native maturity is tracked separately in the technical
overviews linked above.

| Vue / View | Français | English | Contenu / Contents |
| --- | --- | --- | --- |
| Architecture interne Swarm : moteur, responsabilités et preuves / Swarm internal architecture: engine, responsibilities and evidence | [SVG](architecture-overview-fr.svg) · [PNG](architecture-overview-fr.png) · [Mermaid](architecture-overview-fr.mmd) | [SVG](architecture-overview-en.svg) · [PNG](architecture-overview-en.png) · [Mermaid](architecture-overview-en.mmd) | Le moteur déterministe Rust relie contrats, identité, politiques, réservations, contextes, contrôles, revue et validation humaine selon la politique. La qualification Rust est suivie séparément. |
| Identité Swarm : source d’autorité, registre et usage exact / Swarm identity: authority source, registry and exact use | [SVG](identity-authority-fr.svg) · [PNG](identity-authority-fr.png) · [Mermaid](identity-authority-fr.mmd) | [SVG](identity-authority-en.svg) · [PNG](identity-authority-en.png) · [Mermaid](identity-authority-en.mmd) | Identité humaine, principal, instance, octrois, registre, checkpoint Trust et demande exacte restent distincts. Les transports et protocoles internes sont séparés des raccords publics non achevés. |
| Mémoire Swarm : connaissances, contexte et reprise de conversation / Swarm memory: knowledge, context and conversation recovery | [SVG](memory-context-fr.svg) · [PNG](memory-context-fr.png) · [Mermaid](memory-context-fr.mmd) | [SVG](memory-context-en.svg) · [PNG](memory-context-en.png) · [Mermaid](memory-context-en.mmd) | La mémoire de projet, les révisions de consigne, les références extractives, le contexte préparé/final et les frontières de dialogue sont des objets distincts. Les droits d’usage portent sur les sources et destinataires exacts. |
| Supervision Swarm : observation, décision, application et reprise / Swarm supervision: observation, decision, application and recovery | [SVG](run-supervision-fr.svg) · [PNG](run-supervision-fr.png) · [Mermaid](run-supervision-fr.mmd) | [SVG](run-supervision-en.svg) · [PNG](run-supervision-en.png) · [Mermaid](run-supervision-en.mmd) | Les observations sont remises au superviseur IA propriétaire. Le moteur contrôle puis applique la conduite ; l’acceptation du résultat conserve la validation humaine requise par la politique. |
| Observabilité Swarm : logs, direct, télémétrie et preuves / Swarm observability: logs, live views, telemetry and evidence | [SVG](logs-observability-fr.svg) · [PNG](logs-observability-fr.png) · [Mermaid](logs-observability-fr.mmd) | [SVG](logs-observability-en.svg) · [PNG](logs-observability-en.png) · [Mermaid](logs-observability-en.mmd) | Les sorties agent, événements du moteur, incidents, décisions, registre d’appels, mesures système et OTLP gardent leurs sources. Les vues sont en lecture seule ; l’export de données ne constitue pas une preuve d’acceptation. |
| Multitâche dans une mission Swarm / Multitasking within a Swarm mission | [SVG](mission-multitasking-fr.svg) · [PNG](mission-multitasking-fr.png) · [Mermaid](mission-multitasking-fr.mmd) | [SVG](mission-multitasking-en.svg) · [PNG](mission-multitasking-en.png) · [Mermaid](mission-multitasking-en.mmd) | Deux workers peuvent se chevaucher après des admissions séquencées. Le successeur attend ses preuves. |
| Concurrence entre missions dans un Root Swarm / Concurrency across missions in one Swarm Root | [SVG](mission-concurrency-fr.svg) · [PNG](mission-concurrency-fr.png) · [Mermaid](mission-concurrency-fr.mmd) | [SVG](mission-concurrency-en.svg) · [PNG](mission-concurrency-en.png) · [Mermaid](mission-concurrency-en.mmd) | Les Work gardent leurs créneaux et budgets. Le même Root ordonne les tours par espace de travail. |
| Refus enfant et reprise dans Swarm / Child scope refusal and recovery in Swarm | [SVG](scope-refusal-recovery-fr.svg) · [PNG](scope-refusal-recovery-fr.png) · [Mermaid](scope-refusal-recovery-fr.mmd) | [SVG](scope-refusal-recovery-en.svg) · [PNG](scope-refusal-recovery-en.png) · [Mermaid](scope-refusal-recovery-en.mmd) | Le moteur conserve le refus de l’enfant. La racine reçoit un avis sûr et acquitte la reprise de planification. |

## Validation humaine / Human validation

L’architecture et la supervision montrent la branche `human` : résultat et preuves
à examiner, attente humaine sans déblocage aval, puis contrôles d’acceptation.
La branche `automatic` conserve ses contrôles et sa revue IA. Les logs rappellent
la validation humaine si requise ; les révisions de consigne gardent leur parcours
distinct, avec ajouts actifs par défaut et rejet avant réservation.

Architecture and supervision show the `human` branch: examine the result and
evidence, preserve the pending human decision without releasing downstream work,
then enforce acceptance guards. `automatic` retains its checks and AI review.
Logs retain policy-required human validation; instruction revisions remain a
separate path, active by default and rejectable before reservation.

[Contrat FR](../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine) · [EN contract](../en/ENGINE-ACCEPTANCE.md#checks-as-evidence-for-human-review).

## Limites de confiance / Trust limits

Les libellés du journal signé et de la mémoire versionnée explicitent deux limites :
une signature ne prouve pas la vérité du contenu ; un souvenir ne crée aucun droit.
Les cinq familles, tous leurs nœuds et toutes leurs relations sont conservés dans
les deux langues. Les animations du [nouveau film](../communication/linkedin-swarm-identite-memoire-20261023/video-v2/README.md)
sont des extraits explicatifs ; elles ne remplacent pas ces schémas complets.

Signed-journal and versioned-memory labels retain two boundaries: signatures do
not prove content truth, and memories grant no authority. All original bilingual
families, nodes and relationships are preserved. The film explains selected flows;
it does not replace these complete views or qualify the runtime.

## Provenance et lecture / Provenance and interpretation

- [Correspondance de tous les composants avec les sections internes / Complete node-to-source map](CONTRACT-MAP.md).
- [Catalogue FR/EN, sources, empreintes, nœuds et échanges / FR/EN source, hash, node and edge catalogue](architecture-contracts.json).
- [Relecture de la reconstruction / Reconstruction review](../communication/linkedin-swarm-identite-memoire-20261023/internal-architecture-review.json).
- [Correction du contrôle humain et vérifications courantes / Human-control correction and current checks](../communication/linkedin-swarm-identite-memoire-20261023/readme-human-control-review.json).

Le catalogue conserve les entrées des cinq vues initiales et leurs empreintes
historiques. Certaines diffèrent du checkout actuel. Les trois vues de
multitâche et de reprise indiquent séparément leur base, leurs sources relues
et les empreintes de leurs exports.

The catalogue retains the entries and historical hashes for the first five
views. Some differ from the current checkout. The three multitasking and
recovery views separately identify their base, reviewed sources and export
hashes.

Les traits pleins décrivent les échanges du contrat. Les pointillés signalent un
raccord expressément ouvert ou futur. Aucun trait ni couleur ne prouve un
parcours Rust livré. Un bloc fonctionnel n’est pas nécessairement une crate,
un processus ou un service séparé. Les protocoles d’identité internes et les
consommateurs publics restent distincts. **AC-02 A0–A7 n’est pas démarré.**

Solid arrows describe contract relationships. Dashed arrows identify explicitly
pending or future integration. No line or color proves delivery of a native
journey. A functional block is not necessarily a crate, process or separate
service. Internal identity protocols remain separate from public consumers.
**AC-02 A0–A7 is not started.**

Les anciennes vues `memory-identity-fr/en` sont remplacées dans les entrées
courantes par `identity-authority` et `memory-context`. Elles restent sur disque
pour les liens et reçus historiques ; elles ne sont plus la référence active.
The older `memory-identity-fr/en` views remain for historical links/receipts and
are superseded by the separate identity and memory views.

### Reçus de qualification / Qualification receipts

Les chemins d’archive ci-dessous identifient les originaux retenus le
22 novembre 2026. Le [catalogue](architecture-contracts.json) contient leurs
empreintes. Les reçus de recette, l’observation du processus et le résultat
navigateur portent des périmètres distincts.

The archive references identify originals retained on 22 November 2026. The
[catalogue](architecture-contracts.json) records their hashes. Recipe receipts,
process observations and browser results have distinct scopes.

| Archive et reçu / Archive and receipt | Portée / Scope |
| --- | --- |
| `scope-escalation-20261122.oriJSr`, index `verification-index.json`, entrée `escalation/result.json` | Refus enfant, avis racine, décision et reprise avec agents inertes / Child refusal, root notice, decision and recovery with inert agents |
| `cockpit-launch-diagnostics-20261122.PlYsgz/recipe-final/result.json` | Serveur HTTP isolé, motif fournisseur et restauration, pause, retrait du consentement, FR/EN / Isolated HTTP service, provider fault and restoration, pause, consent withdrawal, French and English |
| `cockpit-launch-diagnostics-20261122.PlYsgz/local-version-evidence.json` | Identité du processus local et API de version / Local process identity and version API |
| `cockpit-launch-diagnostics-20261122.PlYsgz/installed-cockpit/result.json` | Motif de pause et rendu FR/EN sur un Root isolé avec le binaire installé / Pause reason and French/English rendering on an isolated Root with the installed executable |

## Rendu et réemploi / Rendering and reuse

Chaque famille a deux sources `.mmd` et leurs exports SVG/PNG. Les SVG sont
vectoriels, à taille native ; les liens ouvrent le dessin détaillé. Utiliser les
sources pour modifier les objets ou les échanges ; ne pas retoucher seulement
l’image exportée. Les PNG sont fournis pour insertion dans un document.

Each family has two `.mmd` sources with matching SVG/PNG. SVGs retain native size
and vector detail; sources remain editable. PNGs support insertion in documents.

Rendu vérifié avec Mermaid 11.12.0, Chrome local et `mermaid-config.json`, sans
accès réseau pendant le rendu ni dépendance ajoutée au produit. Exemple avec
Mermaid CLI et une configuration Puppeteer pointant vers votre Chrome :

```sh
mmdc -i docs/diagrams/identity-authority-fr.mmd \
  -o docs/diagrams/identity-authority-fr.svg \
  -c docs/diagrams/mermaid-config.json \
  -p /chemin/vers/puppeteer-local.json -b '#fffefa' -w 4000 -H 3000
```

Répéter pour les familles du catalogue, les deux langues, puis en PNG. Conserver les
mêmes octets sources pendant rendu et contrôle. Les tailles sont des entrées de
présentation, pas des politiques Swarm. La palette suit [UI-DESIGN](../UI-DESIGN.md).
Fond explicite, titres/descriptions accessibles et `htmlLabels: false` empêchent
l’usage de contenu HTML dans les images. Inspecter textes, liaisons, légendes,
titres et états avant livraison ; un parseur vert seul ne vérifie pas la fidélité.

Repeat for the catalogue families, both languages and PNG outputs with frozen sources.
Rendering dimensions are presentation inputs, not Swarm policy. Explicit
background, accessible title/description and `htmlLabels: false` are retained.
Inspect text, links, titles and status labels; parsing alone does not establish
faithfulness to the source contracts. No binary or running agent context is
updated by these documentation assets.
