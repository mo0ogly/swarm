# Bell-LaPadula dans Swarm — état existant

État documenté au 1er novembre 2026 · [English](../en/BELL-LAPADULA.md).

**Bell-LaPadula est partiellement intégré à Swarm. Les contrôles disponibles ne
constituent pas une protection de confidentialité de bout en bout.** La capacité
moteur conserve `runtime_enforced:false` et `declassification_available:false`.
Ces valeurs restent vraies même après l’enregistrement d’une politique.

## Capacités disponibles

| Capacité | Comportement implémenté | Limite de la garantie |
| --- | --- | --- |
| Modèle de confidentialité | Niveaux ordonnés, compartiments, habilitations ; dominance, combinaison conservatrice, domaine de calcul fixe et refus des classifications inconnues | Contrats de données de `swarm-confidentiality` ; ils n’authentifient ni une source ni un processus et ne confinent pas le système |
| Administration | Politique persistée et révisionnée ; consultation, aperçu, application et contrôle par API/CLI ; éditeur de politique dans l’Admin | Administration sous identité Trust I3/admin courante ; aucune activation implicite de la médiation |
| Attribution d’un objet | `blp-object-attribute` enregistre une classification explicite sous les révisions Trust/BLP courantes, pour un digest et un chemin projet exacts ; rejeu et conflits contrôlés | L’acteur doit aussi être habilité à cette classification ; pas de formulaire Admin d’attribution d’objet ni d’identité physique établie par le seul digest/chemin |
| Monotonie des attributions | Une attribution du même digest/projet ne peut baisser de niveau ni perdre de compartiments lors d’un renouvellement ; un historique déjà abaissé refuse à la relecture | Cette règle seule ne suit pas les copies, contenus modifiés, résumés ou objets de nouveau digest |
| Lecteurs PAEA ciblés | L’aperçu et la présentation d’appel Git préparé exigent identité authentifiée, autorité/politique courantes, attribution exacte et habilitation du lecteur | Ces frontières de présentation ne démontrent pas la confidentialité de l’appel Git ou de tous les parcours PAEA |
| Provenance du prompt | Les nouveaux enregistrements natifs v3 couvrent tous les octets du prompt privé : instructions, contexte projet, contrat éventuel, neuf valeurs JSON et cadrage | Preuve de cohérence des octets, sans classification automatique des producteurs ni autorité pour les historiques antérieurs |
| Diagnostic du prompt enregistré | `blp-preparation-check` retrouve les attributions courantes de chaque partie, les combine et vérifie l’habilitation du demandeur ; source inconnue, révision périmée ou provenance invalide refusent | Lecture seule après enregistrement ; principal du diagnostic distinct du principal de l’envoi ; aucune autorisation de dispatch ou attribution persistée à la réponse |

Biba traite séparément l’intégrité. Un niveau I3, un rôle admin, une session web
ou une configuration fournisseur ne remplacent pas une habilitation de
confidentialité. Une authentification Trust locale ne prouve pas une identité
humaine ni un accord de déclassification.

## Interfaces et usage

Les opérations existantes du namespace `trust` sont :

| Opération | Usage et autorité |
| --- | --- |
| `blp-show` | Consulter politique, historique et révisions ; I3/admin |
| `blp-policy-preview` / `blp-policy-apply` | Examiner puis enregistrer une politique explicite ; I3/admin, révisions courantes |
| `blp-check` | Calculer un diagnostic sur des classifications fournies ; identité authentifiée et politique courante ; les labels de la requête restent des déclarations |
| `blp-object-attribute` | Attribuer explicitement un objet ; I3/admin habilité, intention et révisions explicites |
| `blp-preparation-attribute` | Classer une tranche capturée vérifiée ; I3/admin habilité, parent/index/révisions explicites, digest et chemin résolus par le moteur ; aucun droit d’envoi |
| `blp-preparation-check` | Contrôler les sources d’un tour natif enregistré ; identité authentifiée habilitée, politique courante et attributions de toutes les parties |

CLI : `swarm-rust trust OPERATION --root RACINE --input INTENTION.json --json` ;
le jeton vient de `SWARM_TRUST_TOKEN`, jamais d’un champ d’identité déclaré.
API : `POST /api/v1/trust-actions?action=OPERATION` ; dans le parcours local
owner-key vérifié, sous Host/session/Origin/CSRF
puis authentification Trust indépendante, notamment `Authorization: Bearer …`.
La session seule ne confère pas l’autorité de ces opérations. Ces recettes BLP
ne qualifient pas les modes d’accès Authority/enrôlement développés dans d’autres lots.

Le corps de `blp-preparation-check` contient uniquement `schema_version:1`,
`expected_revision` (révision BLP observée), `preparation_id` et `turn_id`.
Son résultat comporte acteur, révision, digest du prompt, nombre de parties et
classification combinée, avec `diagnostic_only:true` et `runtime_authorized:false`.
Il ne rend aucun texte du prompt. Chaque digest concerne les octets transmis,
y compris l’échappement JSON ; attribuer un document brut ne suffit pas.
Les valeurs structurées restent des parties agrégées et aucun écran d’inventaire
de leurs digests n’est livré.

Les refus HTTP du diagnostic distinguent intention invalide (400), identité ou
habilitation (403), tour absent (404) et précondition à corriger (409). Provenance
incohérente, racine privée incompatible et erreurs de stockage/fermeture restent
des erreurs serveur. Un historique sans provenance ne gagne aucun droit : créer
un nouveau tour par le parcours pris en charge, sans réécrire les anciennes lignes.
Les formats d’attribution, erreurs, révisions et reprises sont détaillés dans
le [contrat technique](BIBA-RUST.md#diagnostic-des-sources-de-préparation).

## Quarantaine des préparations livrée

La version locale 299 ajoute `preparation_quarantine`, éditable dans la politique JSON
de l’Admin et les mêmes opérations API/CLI. True bloque les nouveaux sends et
les émissions contrôlées des workers en attente. Son application attend la fin
d’une émission déjà engagée. False ou absence reste le mode historique sans
garantie BLP ; ce n’est pas un chemin d’envoi classifié autorisé. Le parcours public vérifie 28 assertions et 59 contrôles Admin FR/EN, thèmes et
largeurs 1280/390 ; la livraison vérifie 56 contrôles de version et ressources.
La politique réelle n’a pas été activée par ces recettes. Voir le [contrat](BIBA-RUST.md#quarantaine-explicite-des-envois-de-préparation).

## Travaux non terminés

- **BLP-01** : compléter l’inventaire dynamique des sources, lecteurs et sorties.
  Les 25 flux recensés restent ouverts pour leur contrôle BLP transversal.
- **BLP-03** : migrer explicitement les objets existants non classifiés.
- **BLP-04** : établir l’attribution par les producteurs et l’identité des sources,
  lier le principal authentifié de l’envoi au domaine fixe et au rejeu, puis
  conserver la classification sur les réponses, erreurs, résumés et mémoires.
- **BLP-05** : médiation effective fichiers/réseau/fournisseurs/agents descendants,
  avec liaison des octets au destinataire, compte et politique courants.
- **BLP-06** : déclassification avec identité humaine établie et approbation des
  octets/destinataires exacts ; aucune commande de déclassification disponible.
- **BLP-07** : recette, revue indépendante et RETEX transversaux, après les raccords.

La prochaine chaîne de raccordement reste BF-05 → BF-07 → BF-16 : sources du
prompt → engagement fournisseur → héritage du résultat. Un diagnostic positif
ne peut servir de ticket d’envoi ou de preuve d’acceptation globale.

## Preuves disponibles et portée

La quarantaine est livrée en **v0.1.1-dev.299**, image
`75ad879faf4807d33b6b9c3ff5a71e570aa76d252f394da3b309d95ecadda5e9`,
commit compilé `d4e219eeafcae5f90307a105ddaa93ba9874dc4f`, modifié=true.
[Preuve du processus et de la livraison](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-installed-01/delivery-scope.json).
Une entrée compilée plus récente reste en attente, comme annoncé par le lanceur.

La précédente livraison du diagnostic était **v0.1.1-dev.296**, image
`e4a802c4dc4d32d90d036169604168c64db15196695d692885369b748686ca72`.
La version locale a été relue pendant cette mise à jour : même numéro, commit
compilé `b4cbfa22552140fe9a7bcf46ac91165b0abe120d`, modifications annoncées.
C’est une observation datée, pas une promesse sur une future instance.

| Preuve conservée | Ce qu’elle établit |
| --- | --- |
| [Parcours public](../plans/actions-confiance/verification-blp/20261101-preparation-check-public-01/result.json) | 26 assertions CLI/HTTP sur image normale qualifiée, prompt réel produit par double local inerte ; aucun appel payant |
| [Régression moteur](../plans/actions-confiance/verification-blp/20261101-preparation-check-engine-02/result.json) | Résolution des attributions, combinaison, refus et reprise sur fixture de consommateur |
| [Statuts HTTP](../plans/actions-confiance/verification-blp/20261101-preparation-http-status-01/result.json) | Refus métier distincts des défauts internes et des erreurs de nettoyage |
| [Livraison locale](../plans/actions-confiance/verification-blp/20261101-preparation-check-installed-01/delivery-scope.json) | Image exécutée, 13 sources conformes, 56 contrôles navigateur FR/EN/thèmes/version/ressources et deux aides CLI |
| [Inventaire](../plans/actions-confiance/verification-blp/20261101-preparation-flow-inventory-01/result.json) | Fraîcheur de 25 flux, 50 ancrages et 46 fichiers ; aucun inventaire exhaustif ni contrôle de divulgation déduit |

Les tests sur fixtures, le contrôle du profil de production et l’installation
locale sont des preuves distinctes. Aucun bootstrap Trust réel, classement des
données vivantes, fournisseur réel, déclassification humaine ou acceptation
indépendante globale n’est établi par ces résultats.

[Plan et reprises BLP](../plans/actions-confiance/BLP-APEX.md) ·
[Flux et frontières](BLP-FLOWS.md) ·
[Positionnement de sécurité](../SECURITY-POSITIONING.md).

Ancrages d’implémentation : [modèle](../../rust/crates/swarm-confidentiality/src/lib.rs),
[politique et attributions](../../rust/crates/swarm-engine/src/trust/confidentiality.rs),
[diagnostic du prompt](../../rust/crates/swarm-engine/src/root_store/preparation_call.rs),
[lecteur PAEA](../../rust/crates/swarm-engine/src/root_store/reader_guard.rs),
[éditeur Admin de politique](../../web/trust-confidentiality.js).

Preuves de quarantaine : [moteur et ordre des verrous](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-locks-03/result.json), [CLI/HTTP et Admin réels sur fixture](../plans/actions-confiance/verification-blp/20261101-preparation-quarantine-public-02/result.json). Les refus historiques sont conservés dans le suivi APEX.

Le producteur `blp-preparation-attribute` attribue une tranche capturée vérifiée,
sous I3/admin habilité et révisions courantes : le moteur résout lui-même son
digest et son chemin, sans texte source en réponse. C’est un classement explicite
après capture, sans autorisation d’envoi ni domaine persistant. Contrat et exemple :
[BIBA-RUST](BIBA-RUST.md#attribution-depuis-une-source-de-préparation-capturée).

Cette attribution depuis la capture est livrée en **v0.1.1-dev.304**, image
`341e71bb2e67d47d0c468c04f8c7ecd38f313fb0686b60af88ba675d50cbade6`.
Le parcours public vérifie103 assertions CLI/HTTP, le moteur2 tests, l’adaptateur
HTTP1 test et la livraison56 contrôles navigateur FR/EN. Le profil natif est sans
fixture ;21 fichiers runtime concordent avec l’image active. Les données et
politiques réelles n’ont pas été classées/modifiées par cette recette. Preuves :
[plan APEX](../plans/actions-confiance/BLP-APEX.md#livraison-du-producteur-de-sources-capturées--1er-novembre-2026).

## Domaine fixé avant transport

La309 ajoute l’attribution des tranches avant admission, l’admission sous le
principal Trust authentifié et une capsule privée immuable du prompt complet.
Toutes les tranches exigent une classification courante ; le domaine est leur
jointure, pas l’habilitation maximale. La capsule n’est ni un tour exécutable ni
un droit fournisseur ; aucun budget n’est réservé. Métadonnées sous propriétaire
courant, politique/Root revalidés, sans remise à jour implicite d’une ancienne capsule.

Les opérations `blp-preparation-input-attribute`, `blp-preparation-admit` et
`blp-preparation-admission-show` sont détaillées dans le
[contrat technique](BIBA-RUST.md#admission-immuable-avant-transport).
74 assertions publiques et103 régressions CLI/HTTP passent sur l’image309.
L’image active et22 fichiers runtime sont vérifiés. Les56 contrôles navigateur
concernent une fixture de la même image ; le dialogue réel derrière Authority
reste à confirmer dans une session authentifiée. Ni cette admission ni les
preuves de fixture ne clôturent la confidentialité de bout en bout.

## Liaison explicite du destinataire et décision au dernier usage

Le chantier BLP-05 ajoute deux opérations qui relient le compte/endpoint à un
destinataire contrôlé, puis arbitrent l’envoi au dernier usage — sans émettre.
Leur contrat technique complet est décrit dans le
[contrat technique](BIBA-RUST.md#liaison-du-destinataire-et-transport-au-dernier-usage).

- **`blp-preparation-destination-attribute`** (administrateur habilité) :
  enregistre un label de destination pour une connexion `api-` identifiée par
  son triple `base_url`/`model`/digest de configuration, avec écho obligatoire
  de `base_url` et `model`. La clé de connexion n’est jamais une entrée ;
  le digest canonique est calculé par le Root courant. La liaison est
  ré-enregistrable (dernière écriture gagnante) et meurt avec l’Entry de
  politique qui la porte : tout renouvellement de politique efface les
  liaisons comme les attributions de tranches.
- **`blp-preparation-transport`** (principal propriétaire d’une capsule admise) :
  sous le verrou egress partagé, re-vérifie intégralement la capsule, la
  fraîcheur connection/provider_source/settings depuis l’admission, la liaison
  courante pour ce digest exact, la dominance no-write-down du label de sortie
  vers le label de destination au dernier usage, puis observe en lecture seule
  les enveloppes de budget. Réponse fixe et non émissive :
  `runtime_authorized:false`, `runtime_enforced:false`, état
  `transport-authorized-not-sent`. Aucun tour, réservation, IO fournisseur ou
  modification legacy.

Refus typés : grammaire (`blp-transport-invalid`,
`blp-destination-invalid`), écho (`blp-destination-echo-mismatch`),
attribution absente (`blp-destination-not-attributed`, 404), fraîcheur
(`blp-transport-connection-changed|blp-transport-provider-source-changed|
blp-transport-settings-changed`, 409), write-down (`blp-access-denied`),
budget (`preparation-budget-exhausted|work-budget-exhausted`).

Portée et limites inchangées : la décision n’engage pas le transport réel
(BF-07 reste OPEN), n’hérite pas des résultats, ne confine pas les descendants
et ne protège pas les envois legacy. La vérification s’appuie sur le test
d’intégration `preparation_transport.rs` et le parcours public
`tests/blp_transport_public.py` via la recette
`tools/verification/recipes/blp-preparation-transport-public.json`.

## Hook travailleur d’émission (BF-07)

Le travailleur API (`_prepare_turn`, branche Api de `swarm-cli/src/
preparation_worker.rs`) appelle désormais la décision au dernier usage avant
toute émission classifiée, via
`RootStore::preparation_worker_transport_gate(id, token, base_url, model)`.
La gate tourne dans la réservation egress partagée déjà tenue par le
travailleur et refuse avant la construction de la requête HTTP : aucun socket
ne s’ouvre sans décision.

- Zéro receipt d’admission correspondant au tour
  (preparation_id, provider, request_hash) → `Ok(())` immédiat avant toute
  lecture d’état Trust : le chemin legacy reste inchangé.
- ≥1 receipt : re-vérification complète de chaque capsule (principal lu dans
  le receipt produit par le producteur, jamais fourni par l’appelant),
  fraîcheur settings/provider_source, attribution de destination pour le
  digest résolu de la connexion enregistrée dans le tour, no-write-down au
  dernier usage, observation budget lecture seule. La connexion vient du
  champ `provider` du tour ; `base_url`/`model` effectifs du travailleur sont
  l’écho obligatoire (fermeture du TOCTOU claim→envoi).
- Possession : relecture token=SHA256(nonce) + PID/stamp/host du tour
  (`preparation-worker-registration-mismatch` sinon). Le travailleur ne
  détient aucun principal ; l’identité reste celle du receipt re-vérifié.
- Balayage fail-closed : un receipt correspondant illisible/indécodable
  refuse (`blp-admission-not-found`/`blp-admission-record-invalid`), jamais un
  silence. La gate ne contacte rien, ne réserve rien, n’écrit rien en SQL.

Vérification : test d’intégration `preparation_worker_transport.rs` (refus
token/destination/écho, passage exact, zéro réservation de gate, fixture
auto-nettoyée) et recette
`tools/verification/recipes/blp-preparation-worker-gate.json` (gate, suites
voisines, compilation CLI). Portée : branche Api du travailleur Rust ; le
chemin CLI garde son propre contrôle egress. Héritage des résultats (BF-16) et
confinement OS/descendants restent ouverts ; rien n’est déclaré activé en
production ni accepté.

## Contrôle des lecteurs du dialogue (BF-16a)

La lecture publique du dialogue d’une préparation
(`RootStore::preparation_dialogue(id, reader_token)` ; HTTP
`GET /api/v1/preparations/dialogue` ; CLI `preparation dialogue` avec
`SWARM_TRUST_TOKEN`) applique désormais, **uniquement quand au moins un
receipt d’admission correspond au tour lu**, le contrôle no-read-down :

- Décision en deux temps : la lecture SQL non contrôlée d’abord (entrée de
  décision seule, rien n’est servi avant le verdict), puis balayage des
  receipts d’admission (`blp-admission-{64 hex}.receipt`, lectures
  fail-closed) filtrés sur `captured.preparation_id` et le triplet
  (provider, request_hash) du tour.
- Zéro receipt correspondant → chemin legacy servi tel quel, aucun état
  Trust lu : continuité des préparations sans admission (test dédié).
- ≥1 receipt : identité d’abord (`ReaderIdentityState` de PA-09, observée
  sans réservation — ordre de verrous Trust→settings préservé) :
  session nue → `blp-dialogue-authentication-required` ; journal absent →
  `blp-dialogue-policy-required` ; jeton inconnu/désactivé → refus typé ;
  politique périmée → `blp-dialogue-policy-stale`. Puis récupération en
  trois passes : (1) chaque receipt apparié est re-vérifié en entier
  (`preparation_admission::verify` dans son ordre d’origine, principal lu
  dans le receipt produit) et le résultat est classifié une seule fois —
  le refus exact `blp-admission-policy-changed` marque un receipt
  identité-valide mais périmé, tout autre refus (enregistrement invalide,
  mauvaise Root, re-jointure des sources, stockage) est propagé inchangé
  et refuse la lecture entière même si un équivalent courant existe ;
  (2) un receipt périmé n’est couvert que par une admission courante
  pleinement vérifiée de même principal, même instance de Root ouverte,
  étiquette de sortie strictement égale et lignée capturée identique à
  l’octet (`equivalent_current_covers` ; admission_id/event_id/
  request_digest légitimement redéfinis par un renouvellement ne sont pas
  comparés ; une erreur de sérialisation de provenance refuse avec sa
  cause typée, jamais silencieusement) ; (3) chaque admission appariée,
  courante ou couverte-périmée, repasse l’habilitation lecteur
  `check_read(sujet, classification)` du receipt →
  `blp-dialogue-access-denied`. Une couverture exige l’égalité stricte de
  l’étiquette : un renouvellement qui élève l’étiquette garde le 403 de
  stalèté jusqu’à ré-admission à étiquette égale. Un refus ne sert aucun
  octet du dialogue (pas de filtrage partiel).
- Web : extracteur `reader_bearer_token` (une en-tête `Authorization:
  Bearer <valeur non vide>`, sinon refus typé avant toute ouverture de Root)
  ; les quatre codes `blp-dialogue-*` renvoient 403 FORBIDDEN. CLI :
  variable `SWARM_TRUST_TOKEN` (vide = session nue).
- Le chemin producteur interne (`preparation_call`, capture de réponse)
  conserve la lecture non contrôlée : le producteur est le détenteur
  légitime du domaine ; BF-16a ne change rien côté capture.

Vérification : test d’intégration `preparation_dialogue_readers.rs`
(matrice de refus session nue / jeton inconnu / lecteur non habilité,
récupération par lecteur habilité, continuité legacy sans admission) et
recette `tools/verification/recipes/blp-preparation-dialogue-readers.json`
(dialogue, gate BF-07 voisine, compilation CLI ; preuve
`blp-dialogue-readers-proof-20261105b-2479227`, re-run sur l’arbre évolué
après réception staleness, archive vérifiée).
La frontière HTTP publique et sa récupération staleness sont couvertes par
`tests/blp_dialogue_reader_public.py` et la recette
`tools/verification/recipes/blp-dialogue-reader-public.json`
(reçues les 3–4 novembre 2026 sur les images qualifiées 325/331 :
fraîcheur GET/HEAD 403 `blp-dialogue-policy-stale` après renouvellement
public, ré-admission du même intent, récupération 200 vers le même DTO,
gardes de corruption stockée refuse-500 avec oracle d’erreur exact).
Portée : lecture présentation seulement. Restent ouverts dans BF-16 :
héritage du domaine sur erreurs/propositions, médiation
fichiers/réseau/descendants, migration des données existantes,
déclassification humaine (BLP-06). Rien n’est déclaré activé en production
ni accepté.

### BF-16b — lecteurs de show, history et release-review (6 novembre 2026)

Le même verdict lecteur s’applique désormais aux autres frontières de
présentation de la préparation : `RootStore::preparation_show(id,
reader_token)`, `RootStore::preparation_history(id, kind, before,
reader_token)` et `RootStore::preparation_release_review(id, reader_token)`
(HTTP `GET /api/v1/preparations/show`, `/history`, `/release-review` ; CLI
`preparation show|history|release-review` avec `SWARM_TRUST_TOKEN`) :

- Le verdict est extrait en une fonction partagée
  `preparation_dialogue::reader_verdict` (pub(super)) appelée par
  `read_controlled` (dialogue) et par show/history : mêmes codes
  `blp-dialogue-*`, même balayage scan-then-decide, même récupération en
  trois passes, même habilitation ; aucun octet de préparation ni de
  document n’est servi avant `Ok(())`.
- Show/history lisent les clés de décision via l’aide
  `recorded_turn_keys` : les tours enregistrés de CETTE préparation sont
  décodés (provider/request_hash vivent dans le corps enregistré) sous la
  même capture de lecture, prompts effacés avant restitution — entrée de
  décision seule, rien n’est servi par l’aide. Une incohérence d’identité
  de ligne refuse comme la frontière dialogue
  (`preparation-dialogue-row-identity-mismatch`). Contrairement à la
  lecture dialogue, aucune observation d’historique actif ne s’y exécute
  (show/history observent des documents, pas des appels vivants).
- Zéro receipt apparié → chemin legacy inchangé : session nue conservée
  sur les préparations sans admission (tests de continuité dédiés pour
  show et history).
- Les documents exposés par show/history héritent du domaine du tour
  source (`source_turn` : brief/plan adoptés) : un lecteur sous-habilité
  ne peut plus lire par show/history ce que le dialogue lui refuse.
- `preparation_release_review` propage le jeton vers `preparation_show` :
  une revue de conversion d’une préparation portant une sortie classifiée
  admise refuse un relecteur non habilité avec les mêmes codes.
- Web : les trois routes extraient `reader_bearer_token` comme dialogue ;
  les quatre codes partagés renvoient 403 FORBIDDEN (mapping inchangé).
  CLI : `SWARM_TRUST_TOKEN` (vide = session nue), résolu une fois pour
  show/release-review/dialogue/history.

Vérification BF-16b : tests d’intégration étendus
`preparation_dialogue_readers.rs` — `show_and_history_inherit_the_
dialogue_reader_verdict` (session nue et lecteur sous-habilité refusés sur
show et sur history pour chaque kind, récupération par le sender habilité)
et `show_and_history_without_any_matched_admission_keep_legacy_bare_read`
(continuité session nue sans admission, les deux frontières) ; suites
web/CLI (`service`, `confidentiality_http`, `swarm-cli`) repassées. Portée
toujours : présentation seulement ; l’exécution, l’adoption et les
autres canaux restent hors de ce verdict. Rien n’est déclaré activé en
production ni accepté.

### BF-16c — lecteurs de send (rejeu et première admission) et de stop (tour terminal) (8 novembre 2026)

Le verdict lecteur couvre désormais les deux dernières frontières qui
servaient un tour enregistré sans contrôle : `RootStore::preparation_send(raw,
environment, reader_token)` et `RootStore::preparation_stop(preparation_id,
turn_id, environment, reader_token)` (HTTP `POST /api/v1/preparations/send`
et `/stop` avec `Authorization: Bearer` ; CLI `preparation send|stop` avec
`SWARM_TRUST_TOKEN`) :

- Send — rejeu : un appel dont le triplet (provider, request_hash) existe
  déjà servait le `PreparationTurn` enregistré (réponse classifiée incluse ;
  le prompt n’est jamais sérialisé) comme les autres lectures de
  présentation, sans aucun contrôle. La rejeu décide maintenant le verdict
  partagé (`reader_verdict` sur les clés `recorded_turn_keys`) avant de
  rendre le tour : mêmes codes `blp-dialogue-*`, aucun octet servi avant
  `Ok(())`.
- Send — première admission : le tour tout juste enregistré rend le domaine
  classifié de la préparation appariable par les receipts d’admission
  (mêmes clés provider/request_hash) et le prompt assemblé embarque
  l’historique classifié antérieur. Le verdict est donc décidé **dans la
  transaction d’écriture**, après `save` et avant la réservation de budget :
  un refus annule toute l’admission — pas de ligne de tour, pas de
  réservation, pas d’octet servi, et un appelant sous-habilité ne peut pas
  consommer le budget d’une admission qu’il ne peut pas lire.
- Stop — tour terminal : les deux branches de service du tour terminal
  (ligne legacy et native) servent la réponse enregistrée ; chaque branche
  décide le verdict partagé quand le tour n’est plus actif, avant le
  retour. Un stop sur un tour actif reste une transition d’état du
  producteur, pas une lecture : pas de verdict.
- Chemin producteur interne : le ràz-frais du moniteur (`preparation_stop`
  depuis la boucle de settle) passe explicitement `None` avec le commentaire
  d’exemption — producteur interne, aucun lecteur servi, miroir de
  `preparation_call` resté non contrôlé en BF-16a. `None` n’est jamais une
  autorisation : une session nue est refusée comme avant dès qu’un receipt
  apparié rend le domaine classifié.
- Web : `preparation_calls` extrait `reader_bearer_token` comme les
  frontières GET (les requêtes à capability restent en lecture seule et
  hors contrôle, choix BF-16a inchangé) ; le mapping 403 FORBIDDEN de
  `service.rs` couvre les quatre codes partagés sous l’opération
  `preparation-call`. CLI : `SWARM_TRUST_TOKEN` résolu une fois pour send
  et stop dans `calls()`.

Vérification BF-16c : tests d’intégration
`preparation_dialogue_readers.rs` portés à 6/6 — les deux nouveaux tests
`send_replay_and_stop_serve_classified_turn_without_reader_control`
(session nue et lecteur sous-habilité refusés sur la rejeu ET le premier
envoi admis, aucun octet servi et admission annulée, puis service du tour
terminal refusé à la session nue et au jeton inconnu, récupération par le
sender habilité) et
`send_replay_and_stop_without_any_matched_admission_keep_legacy_bare_path`
(continuité session nue sans admission sur send et stop). Preuve
rouge→vert : avec les signatures antérieures la rejeu servait le tour là
où le refus est affirmé. Régresseurs : `preparation_calls` 24/24,
`confidentiality` (1/1), `preparation_worker_transport` (chemins
legacy/natifs adaptés), `confidentiality_http` + `service` web (11/11),
gate CLI 2/2, check workspace 0 erreur. Appelants adaptés : exemples et
suites (sends/stops legacy en `None` explicite). Portée toujours :
présentation seulement ; l’exécution, l’adoption et les autres canaux
restent hors de ce verdict. Restent ouverts dans BF-16 : héritage du
domaine sur erreurs/propositions, médiation fichiers/réseau/descendants,
migration des données existantes, déclassification humaine (BLP-06).
Rien n’est déclaré activé en production ni accepté ; pas de commit/push
sans autorisation explicite.

### BF-16d — lecteurs de l'adoption de proposition, de la rejeu de commande et de la liste (8 novembre 2026)

Le verdict lecteur couvre les trois frontières restantes qui servaient le
domaine classifié d'une réponse admise sans contrôle, conformément à
BLP-APEX (« BF-16 doit hériter ce domaine sur réponses/erreurs/
propositions et lecteurs »). L'axe erreurs était déjà couvert par BF-16a/b/c
(les DTO sous contrôle de frontière, codes statifs sûrs uniquement) ;
celui-ci ferme les propositions et la liste :

- Adoption (`use-proposal`) : `RootStore::preparation_command(raw,
  environment, reader_token)` gagne `reader_token` comme dernier paramètre
  (miroir de `preparation_send`). Une adoption copie la réponse classifiée
  du tour source dans `documents` (et `adopted_brief`) — le document hérite
  du domaine classifié de sa source. Le verdict partagé `reader_verdict`
  (clés `recorded_turn_keys` de la préparation) décide **dans la
  transaction Immediate, après `apply` et avant le commit** : un refus
  annule toute la mutation — document, incrément de révision et ligne de
  reçu — par le rollback existant. Un seul appel couvre ainsi la première
  adoption ET la rejeu du reçu (la branche early-return de `apply` sert un
  corps de `Preparation` historique, qui est une lecture de présentation
  comme les autres).
- Liste (`preparation_list`) : `preparation_list(reader_token)` juge
  chaque ligne comme `preparation_show` (même `reader_verdict` sur les
  tours enregistrés de la préparation listée). La première ligne à
  receipt apparié sans lecteur suffisant refuse **toute** la liste avant
  qu'un octet soit servi — une liste qui servirait les lignes non
  classifiées et refuserait les autres révélerait par sa réponse même
  l'existence d'un contenu classifié.
- Continuité legacy : zéro receipt apparié garde le chemin nu existant
  (adoption, rejeu à révision courante, liste) — la décision reste
  attachée aux admissions appariées, jamais à la présence d'une policy.
- Surfaces publiques : HTTP `POST /api/v1/preparations/command` et
  `GET /api/v1/preparations` extraient `reader_bearer_token` avec
  l'extracteur partagé (en-tête malformé refusé avant toute lecture de
  Root) ; le mapping 403 FORBIDDEN de `service.rs` couvre les codes
  partagés. CLI : `SWARM_TRUST_TOKEN` résolu dans `write()` (commande) et
  dans le chemin read-only pour `list`. Vues budget : compteurs
  uniquement, aucun octet de préparation — hors périmètre (choix BF-16a
  inchangé).

Vérification BF-16d : tests d'intégration `preparation_dialogue_readers.rs`
portés à 8/8 — le nouveau
`use_proposal_replay_and_list_adopt_classified_answer_without_reader_control`
(session nue refusée à l'adoption ET à la rejeu ET à la liste, aucun octet
classifié dans aucun refus, lecteur sous-habilité refusé
`blp-dialogue-access-denied`, jeton inconnu refus typé Authorization,
preuve qu'un refus ne laisse aucune trace — révision inchangée, pas de
document brief, pas de ligne de reçu —, puis récupération par le sender
habilité qui adopte, rejoue et liste) et le jumeau
`use_proposal_replay_and_list_without_any_matched_admission_keep_legacy_bare_path`
(continuité session nue sans admission : adoption, rejeu identique à
révision courante, liste). Preuve rouge→vert : signature threadée d'abord
(compilation rougi sur les 10 sites d'appel du test), puis verdict
ajouté — le rouge runtime établi (`bf16d-red2.log`) montre la session nue
ADOPTER le brief classifié là où le refus est affirmé. Régresseurs :
`preparation_writes` 5/5, `preparation_calls` 7/7, `preparation_plan` 1/1,
`preparation_transport` 1/1, `preparation_worker_transport` 1/1,
`confidentiality` 16/16, `server_mutations` web 1/1, check workspace
0 erreur. Appelants adaptés : CLI `write()`/`list`, web
`preparation_command`/liste, exemple `preparation_quarantine_fixture`,
suites transport (sites pré-admission en `None` explicite). Portée
toujours : présentation seulement ; l'exécution, la conversion et les
autres canaux restent hors de ce verdict. Restent ouverts dans BF-16 :
médiation fichiers/réseau/descendants, migration des données existantes
(BLP-03), déclassification humaine (BLP-06). Rien n'est déclaré activé en
production ni accepté ; pas de commit/push sans autorisation explicite.
