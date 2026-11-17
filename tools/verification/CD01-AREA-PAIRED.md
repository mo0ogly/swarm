# C-D01 — harnais apparié proposé (SOURCE SEULE)

Aucun fichier de ce lot n'a été importé, compilé ou exécuté pendant sa préparation. Ce contrat et ses marqueurs PASS sont des attentes futures, pas des résultats. Allocation root/revue, profil, graphe et stockage admis sont obligatoires avant toute commande ci-dessous. CI061b reste gelé par son propriétaire.

## Ce qui est réellement comparé

Un seul libtest `swarm-engine` construit sur la composition R1a plus ce harnais contient les deux branches. La branche `legacy` appelle uniquement `Legacy::resolve`, ancienne closure main230d exactement conservée hors binding root/cache possédé ; elle n'appelle **jamais Context**. La branche `fixed` appelle Context puis le vrai convertisseur `resolved_path` utilisé par le wrapper public. Elles rendent toutes deux PathBuf ou panic au **même** consommateur d'effet inerte. Aucun symbole récent n'est requis pour faire tourner l'ancien algorithme adapté.

Cette méthode compare les comportements de la closure ancienne adaptée et du résolveur R1a dans un environnement de compilation commun et figé. Elle n'exécute pas une ancienne image produit livrée et ne prouve pas le wrapper public global ancien. Le module test `paired_legacy.rs` reprend l'adaptateur c9261c… ; seul son commentaire de statut est adapté à l'inclusion test. Sa provenance textuelle complète est fournie dans le paquet de préparation, pas supposée d'après son nom.

Deux probes Rust nommés, pas une duplication des dix tests Context de R1 :

- `fixture_area::paired_harness::paired_probe` : six scénarios communs, chaque variante dans son propre processus/Root neuf, même source de fixture, euid/egid et modes initiaux rapprochés. Un témoin A lisible, distinct du défaut B, et le même effet précèdent toute faute.
- `fixture_area::paired_harness::global_wrapper_probe` : contrôle, refus/réparation dans le même processus, puis adoption dans un processus neuf. C'est le **vrai configured_area global R1a**, seulement après, sur un snapshot de compilation possédé dont le repo dérivé de CARGO_MANIFEST_DIR correspond exactement au Root déclaré. Aucun override de repo_root ou reset de OnceLock. Ce probe n'est jamais compté comme RED legacy.

Le second probe refuse un checkout/.git, une `.swarm` existante ou un Root non marqué/non frais. Il ne modifie pas le fichier livré/embarqué de compilation. Le pilote conserve, par renommage dans ce même snapshot possédé, le dossier de contrôle et ses rapports avant la faute. Les réglages de fixtures et la source produit partagée sont distincts. Marqueur et hash ne donnent ni autorité, ni authentification, ni confinement contre un autre acteur de même UID ; la propriété/réservation du snapshot vient de l'allocation root séparée.

## Contrats observables

| Scénario commun | Témoin / cible | RED legacy attendu à établir, pas exécuté |
| --- | --- | --- |
| presence | A valide ; changer source en B conserve A dans ce contexte ; contexte neuf adopte B. Effets 1→2→3→4. | Aucun : contrôle commun doit être TARGET_MET des deux côtés. |
| missing | Document présent sans champ : refus non-configuré, aucune action ; réparation explicite et contexte neuf. Effets 1→1→1→2. | Ancienne branche prend défaut B et autorise l'effet. |
| json | JSON tronqué : cause serde_json originale reçue via le vrai payload/catch ; pas d'effet ; refus retenu après réparation. | Panic ancienne opaque, sans canal de cause typée du helper ; pas un symbole manquant. |
| permission | Témoin sous même identité, puis EACCES effectif réel ; pas de défaut ni d'effet ; refus retenu et contexte neuf pour reprise. | Erreur read ancienne convertie en défaut B. Identité privilégiée/lecture encore possible = précondition en échec, jamais RED. |
| cache | Première entrée JSON refusée, puis source réparée ; même contexte reste refusé et sans nouvel effet. | Adoption après panique dans le même ancien contexte ; ici le critère RED **n'exige pas le nouveau type d'erreur**. |
| concurrency | Quatre appels synchronisés sur même contexte invalide ; réparation ; ce contexte conserve son refus. | Même critère comportemental de réadoption après réparation, pas absence d'Arc/Context. Le compteur d'unique accès et l'identité Arc sont contrôles supplémentaires fixed-only. |

Le journal est un fichier inerte append-only d'un octet par retour PathBuf au consommateur. Son IO est hors catch du configurateur : ENOSPC/erreur de journal ne fabrique jamais un RED ancien. Avant réparation, les octets de réglage sont rapprochés ; la lecture de preuve externe ne remplace pas l'accès réel de l'algorithme. On conserve primaire/typed cause/classification/errno, jamais un message arbitraire de panic dans le JSON. Les stdout/stderr bruts restent des preuves privées.

Le pilote exige à chaque enfant : code0, nom exact, `running 1 test`, compte `1 passed; 0 failed`, marqueur de fin et rapport JSON cohérent. Les douze probes appariés doivent donner six fixed TARGET_MET, un contrôle legacy TARGET_MET et cinq contre-exemples legacy BEHAVIORAL_RED. Une compilation échouée, un panic non capturé du consommateur, un témoin/reprise invalide ou zéro test refuse le lot ; aucun ne devient RED. Le mode global exige ensuite trois probes nommés complets. Ces cardinalités découlent des six scénarios/deux variantes/trois contextes décrits ; ce ne sont pas des bornes opérateur ni des résultats acquis.

## Ce qui doit compiler après allocation root

Dans un **snapshot source possédé séparé**, composition exacte R1a plus les fichiers de ce lot :

```sh
cargo test --manifest-path rust/Cargo.toml -p swarm-engine --lib --no-run --locked --offline --message-format=json
```

Commande proposée uniquement, non lancée. Profil Cargo test/libtest, Linux visé par O_PATH, features explicites vides, sans all-features ni feature `test-fixtures` ; pas de cible intégration, exemple, worker ou serveur. Root doit qualifier la version du compilateur, target triple, flags/environnement, unification transitive réelle, dépendances, ressources include_bytes et graphe effectif. Si offline manque une dépendance : prérequis de compilation en échec, pas RED et pas de fetch/retry implicite. Le champ JSON profile n'est pas une preuve de ce graphe.

Conserver les sorties Cargo et sélectionner l'artefact libtest réel depuis `compiler-artifact` (`target.kind` lib, `profile.test` vrai, executable non vide), puis son SHA256 et le record d'entrées complètes. Ne pas deviner son chemin à partir d'un nom. Le pilote **ne compile pas** ; il reçoit ce binaire déjà admis et recontrôle image/entrées avant/après chaque enfant.

Le record v1 doit déclarer scope `cd01-paired-libtest`, base230d, compiler_repository absolu, binary absolu/hash, profil exact attendu par admit(), compiler_inputs absolus/hash complets (pas seulement le sous-inventaire du harnais), harness_source_relpaths, owned_repository_token correspondant au marqueur `.cd01-build-owner` du snapshot neuf, et evidence_storage mountpoint/UUID/fstype. Le template du paquet a volontairement scope TEMPLATE_NOT_ADMITTED : il doit refuser, jamais servir de record vert. Root confirme réserve/capacité/allocation effective des volumes avant lancement ; le pilote contrôle montage réel/rw/device et garde son fd, sans promettre disponibilité après retrait physique. Pas de plafond/test-timeout/retry ajouté.

## Recette permanente proposée

Fichiers proposés : `tools/verification/cd01_pair.py`, `recipes/cd01-area-paired.json`, les deux probes Rust et l'adaptateur test. `SWARM_CD01_PAIR_BUILD_RECORD` pointe vers le record réel admis ; `SWARM_CD01_PAIR_EVIDENCE_PARENT` vers un parent de preuve admis sur le volume déclaré. Les variables héritées ne sont pas une autorisation. Le pilote conserve leur contexte public utile, argv, sorties brutes, code réel, rapport, source/état stable et stockage ; aucun secret ne doit être mis dans le record.

Après allocation seulement :

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/cd01-area-paired.json --output /CHEMIN/NEUF
python3 tools/verification/recipe.py verify --evidence /CHEMIN/NEUF
```

Le manifeste déclare les entrées de contrôleur/harnais/config directes, pas tout le graphe Cargo. Le record admis et son archive indépendante doivent couvrir les autres sources et profils effectifs. Un snapshot minimal de cette recette n'est pas une compilation complète. La recette exécute d'abord paired puis global ; le premier échec arrête la suite. Les deux phases conservent des preuves neuves ; aucune relance automatique.

## Limites et suite

Typecheck, build, exécution, documentation compilée et réception indépendante sont non établis. Les dix gardes R1 ne sont pas rejoués par ces deux probes. FIFO, parent remplacé, close/observer combinés, close noyau incertain et les 231 appelants aval restent dans leurs périmètres propres ; aucun transfert de couverture depuis ce harnais. Les effets sont locaux/inertes, sans paiement, capacité financière, appel modèle/fournisseur, acteur authentifié ou service. Pas de clôture C-D01/BF-L0.

Prochaine étape : revue source du paquet figé et admission par root du montage paired legacy/root-cache et des champs de sortie, puis compilation ciblée seulement au créneau propre libéré par CI. Conserver original, refus de précondition et reprise neuve ; ne pas demander une nouvelle autorisation utilisateur pour ce qui est déjà alloué, ni lancer avant le séquencement root.


### Relecture de preuves internes sans rejouer

Chaque phase conserve les stdout/stderr exacts, argv/code, JSON original/copie'safe, entrées fautives avant/après et journal d'effet, puis un inventaire hashé et un reçu terminal. La publication finale n'affiche PASS qu'après relecture des octets ; une erreur de clôture/publication garde un état incomplet et ses causes. Les causes primaires et secondaires de stockage ne deviennent jamais RED du candidat.

Le vérificateur extérieur de recette ne parcourt pas automatiquement ces sous-preuves. Les chemins neufs sont rendus par le pilote ; une relecture ultérieure peut être allouée sans relancer les probes :

```sh
python3 -B tools/verification/cd01_pair.py verify --evidence /PREUVE_INTERNE_NEUVE
```

Ce mode relit uniquement l'inventaire/receipt et les hashes, sans construire ou tester. Intégrité locale n'est ni authentification ni réception indépendante. Aucun de ces modes n'a été exécuté pour préparer le présent lot.


Précondition de transport du payload : le record doit déclarer `panic: unwind` dans le profil et root doit le rapprocher des flags réellement employés. Un binaire panic=abort ne permet pas ce catch ; son arrêt/nonzero serait un prérequis en échec, jamais un RED du comportement. Cette exigence technique du harnais n'ajoute pas de politique produit ni de fallback automatique.


## Correction de custody proposée après revue04 — source seulement

Les deux P2 du paquet précédent sont traités dans ce nouveau candidat privé, sans exécution ni acceptation. Le contrôleur réutilise **sans modification** native_build_outcome.py27b2bf5… présent dans main230d : CompletedProcess reste détenu avant les fermetures protégées ; l'outcome typé returncode/signal est construit avant les callbacks de sync/hash/publication. Les canaux du producteur sont non bufferisés ; chaque fermeture et chaque hash est tenté indépendamment. sync_digest ajoute fsync/close/hash indépendants après la capture de l'issue ; aucun échec ne remet un code connu à None.

Le producteur effectue sa propre admission avant/après, puis checked_phase tente de nouveau les entrées/image après la validation/copie du rapport, **même si** le child est nonzero, un parseur/receipt échoue, une source change ou devient illisible. Chaque source et l'image sont observées indépendamment, avec attempted/ok/hash inconnu explicites. Le contrôleur entier et la publication du reçu final ont aussi leur phase post-observée ; aucune qualification ne repose sur le seul chemin heureux.

Le final failure_document réutilise le producteur commun et conserve les frames réelles imbriquées, causes primaires/secondaires, issue enfant connue/inconnue, observations de canaux, admissions et échecs de publication. Une erreur après écriture visible reste publication refusée/incertaine, pas rollback. Les erreurs du dernier diagnostic peuvent empêcher sa persistance : le fallback terminal garde l'arbre complet. Le seal de fichiers ou le mode verify ne prouve pas, seul, que le contrôleur a terminé avec code0 ; son reçu externe réel/marker et l'état final doivent être rapprochés. Aucun marker de succès d'exécution ne suit une erreur retenue.

Cinq gardes Python **inertes proposés**, recette séparée cd01-paired-custody-inert.json : (1) nonzero7+source changée ; (2) rapport malformé+source illisible après completion ; (3) premier fsync refusé, second canal encore observé ; (4) receipt visible refusé+fermeture après libération réelle, code-15/signal15 conservé ; (5) publication tardive+observation terminale de dérive. Chaque faute a témoin avant et reprise dans un essai neuf. Toutes les exécutions subprocess sont remplacées entièrement ; le texte de faux test est une donnée INERT, jamais une preuve de Rust. Les assertions inspectent les vrais nœuds typés et champs, pas une mention de classe dans un message. L'enveloppe unittest conserve ses comptes avant sync/close et post-observe aussi ses propres entrées. Pas de replay des anciennes campagnes stdio8/8 ou des10gardesR1.

Ces cinq gardes n'ont pas été importés ou lancés ; ils ne sont pas un RED d'API absente du contrôleur précédent. Avant une comparaison d'exécution entre versions Python, une adaptation de l'ancien appel/canaux devrait être admise séparément ; un échec du témoin ou de signature n'est jamais un RED. Les six sources Rust/probes/algorithmes sont strictement inchangés. Revue04 puis allocation root des seuls gardes inertes, et compilation/libtest à un autre créneau admis. Dernière information root : hôte sous la réserve20GiB, donc **aucune exécution/copie de snapshot/build actuellement admise**, même avec target sur DD. Le plan de build est une annexe séparée ; il ne retarde ni n'autorise ce correctif source.


## Complément P2 reçu terminal — source privée proposée, non exécutée

Origine : review04-cd01-custody-source-20261116-f6q42c79/REPORT.json,
REQUEST_CHANGES_BOUNDED_TERMINAL_RECEIPT. L'ancien sceau précède le reçu tardif ;
sa présence seule ne prouve ni lecture ni intégrité. Ce complément ne remplace
aucune preuve historique et ne déclare aucune réparation fonctionnelle vérifiée.

Contrat proposé : conserver le sceau interne (artifacts.json/seal.json), puis
publier exclusivement controller-postcheck.json après le postcheck du travail.
checked_phase observe séparément les entrées après cette publication ; un échec
reste incomplet avec causes/custody, sans nouvel enfant ni nouvelle publication
identique. Après réussite, terminal-integrity.json lie exactement les octets de
controller-postcheck.json, seal.json, artifacts.json, admitted-record-copy.json
et result.json, et conserve le second postcheck. Schéma version1 et portée
cd01-retained-terminal-integrity sont des invariants de format, pas une politique.

verify_retained lit l'enveloppe et les cinq documents liés ; il vérifie leurs
empreintes, le lien sceau→inventaire et inventaire→record/résultat, puis la phase
controller-work-and-publication et les deux phases postcheck exactes. Chaque
attempted/ok doit être le booléen true, sans erreur ; sources exactes (compiler_inputs,
_controller_record, _controller_source, _outcome_source), empreintes et image
correspondent à la copie admise. record_sha256/binary_sha256 du résultat doivent
correspondre aux pins admis. Inconnu, source manquante/supplémentaire, valeur
modifiée, échec ou erreur IO/JSON refusent. Lecture et comparaison utilisent les
mêmes octets retenus ; les causes originales de lecture/parse sont propagées.

L'enveloppe est un contrat d'intégrité distinct et vérifié, pas un sceau récursif
impossible de sa propre publication. Elle lie deux snapshots historiques réussis,
pas une stabilité future illimitée. Les écritures/flush/sync/readback restent
faillibles : publication visible incertaine n'est ni rollback ni PASS. Une
réécriture cohérente de tout le dossier n'est pas authentifiée par des hashes
locaux ; rapprocher reçu externe/empreinte ancrée lorsque ce droit est nécessaire.
Aucune preuve de code0 du contrôleur, identité d'acteur, réception indépendante,
acceptation ou graphe compilateur n'en découle. Aucun lancement pour réparer une
preuve ; refus/incomplet conservé et reprise dans une sortie neuve seulement.

Cinq gardes source séparées : témoin sealed sain+terminal valide avant chaque
mutation, puis reprise neuve. Vide et JSON invalide sont aussi rebondés dans
l'enveloppe de fixture pour atteindre le parseur ; métadonnées fausses testent
phase, source absente/modifiée, image différente/inconnue, post-publication échouée
et booléen comme schema_version. Octets JSON valides altérés sans re-scellage
refusent par hash. EACCES synthétique au read_bytes exact conserve la même cause,
sans dépendre des privilèges OS. subprocess.run remplacé fail-closed et compteur
zéro ; aucune probe Rust. Le dossier de fixture est une preuve inert-schema, pas
un snapshot compilateur ni une admission de montage. Ces gardes ne sont ni
importées ni lancées ; les cinq anciennes gardes sont inchangées en octets.
Recette future distincte cd01-terminal-receipt-inert.json, sans exécution allouée.

La construction/publication/relecture de l’enveloppe passe aussi par checked_phase : erreur de hash/écriture/relecture conserve les causes et earlier_custody, visibilité de publication inconnue si refus, puis observation indépendante des entrées même en échec. Son postcheck final ne se scelle pas récursivement ; sur refus il atteint failure.json/fallback.


## Révision du 17 novembre — sélection explicite des probes

Les deux probes exigent un contexte de fixture admis ; elles sont marquées `#[ignore]` afin que les sélections ordinaires libtest ne les exécutent pas. Le contrôleur utilise le nom exact avec `--ignored`. La compilation initiale seule ne vérifiait pas ce contrat : son image listait deux probes mais aucune dans `--list --ignored`. Ce défaut de sélection a été constaté sans exécuter leur corps. Le garde inerte du nouvel argv a d’abord échoué puis est corrigé avec le contrôleur ; les cinq anciennes campagnes de faute ne sont pas rejouées. Les douze probes appariés/trois globaux restent non alloués et exigent le record effectif admis.


### Réception R2 : compilation et métadonnées vérifiées

Image `91ac8c1913e38b38aba0c287c58344ec7c37dc3bb0ec535f6d6d8ef9ca3626b1`, Cargo0,949candidats archivés relus et117artefacts. Les candidats diffèrent de R1 seulement par paired_harness.rs et l’enregistrement de la nouvelle cible. Le listing sain contient les deux probes ; le listing `--ignored` les contient désormais toutes deux. Aucun corps exécuté. Code/source du contrôleur et garde modifiés : un healthy inerte ciblé1PASS ; pas de rejeu des cinq fautes historiques. Ce critère de sélection est vérifié ; les quinze probes comportementales, la fermeture effective complète et l’admission du record restent ouverts. Les résultats originaux et toutes les erreurs sont conservés.

Apprentissage : une compilation de libtest ne prouve pas l’isolation d’une probe manuelle. Rapprocher les attributs de sélection de l’image et les options effectives du consommateur exact avant d’allouer les corps ; un zéro-test ne devient jamais PASS. Le contrôleur exige toujours nom/compteur/rapport réels et refuse l’ancienne image sous `--ignored`. Cette consigne documentée n’est pas une garantie sur un passage futur.
