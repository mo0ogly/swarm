# V2 : préparation d’une comparaison fondée sur une connaissance de projet

16 octobre 2026, après « next ». M7.1 à M7.4 sont réalisés et vérifiés localement.
Aucun nouvel appel de modèle, aucune mission Swarm, aucun déploiement. Le lot
porte sur les recettes d’évaluation et leurs preuves ; il n’ajoute pas une
capacité d’extraction automatique au moteur. Les résultats réels de M6 restent
conservés avec leur barème initial.

## Ce qui change et pourquoi

Le protocole précédent permettait de confondre admission bloquée maintenant et
reprise possible après de nouvelles preuves. V2 sépare `current_state`, le fait
sourcé, `current_actions` et `future_condition`. L’ordre d’actions équivalentes
n’est pas imposé. La grille distingue conformité et résolution du manque
d’information : demander une preuve manquante est une réponse correcte.

L’information étudiée vient de [model.go](../../../internal/engine/model.go),
constante `schemaVersion`, actuellement **33**, et de
[store.go](../../../internal/engine/store.go), garde `openStoreWithMigration`.
Le test parse la constante, la compare à celle compilée et vérifie la garde dans
une racine isolée : une consultation d’un stockage synthétique 32 est refusée
avec `storage_upgrade_required`, sans migration ni modification du travail.
Ce constat n’observe pas la base ou le binaire du serveur utilisateur.

Le RETEX de fixture lie ses preuves aux snapshots complets des deux sources.
Le gate vérifie leurs empreintes avant l’admission d’une procédure mémoire. Les
statuts acceptés de cette fixture sont construits par le test ; ils ne prouvent
pas une acceptation métier indépendante ou un apprentissage automatique.

Quatre contextes sont exportés : deux avec/sans rappel pour les sources
concordantes, deux avec/sans rappel pour une cible dont une empreinte diffère.
Cette variante est synthétique ; elle ne prétend pas que la constante du dépôt
aurait changé. La connaissance n’est applicable qu’après comparaison des deux
empreintes. Sans fait fourni ou avec un contrat différent, une version devinée
reste non sourcée, même si elle est numériquement exacte.

## Résultats locaux

| Contrôle | Résultat et portée | Original sous `evidence/m7-20261016/` |
| --- | --- | --- |
| Constante issue du code, snapshots vérifiés, consultation sans migration et travail conservé | PASS, fixture isolée | `knowledge-contexts-safe.json/.log` |
| Quatre contextes, aucun agent ni tentative, paires identiques hors mémoire après neutralisation tracée de la révision | PASS, 1 test Go et 2 sous-tests | Même original ; chaque paire ajoute 1 569 octets de mémoire/attribution. |
| Absence de la déclaration du fait dans le contexte sans mémoire | PASS | Assertions du même test ; fichiers `.engine.txt` et `.txt` dans `pack-safe/`. |
| État courant/futur, abstention, fait deviné, source différente, actions dangereuses et intégrité des snapshots | PASS, 11 tests Python nommés | `safe-knowledge-tests.receipt.json/.log` |
| Comportement et entrées invalides de la grille V1 | PASS, 13 tests nommés | `safe-legacy-tests.receipt.json/.log` |
| Diagnostics, vrais outils, refus d’un autre protocole/paquet/schéma et absence de relance d’un cas existant | PASS, 7 tests nommés | `safe-collector-tests.receipt.json/.log` |
| Quatre réponses manuelles correctes, dont trois abstentions justifiées | PASS de la grille ; modèle NOT TESTED | `safe-results-correct.json`, `safe-grade-correct.receipt.json/.log` |
| Ancienne connaissance réemployée sur le contrat différent | FAIL attendu du contre-exemple, contrôle négatif vérifié | `safe-results-stale.json`, `safe-grade-stale.receipt.json/.log` (sortie 1 attendue) |
| Rejeu local des réponses réelles M6 avec le lecteur actuel | Identique au JSON historique, y compris son FAIL et ses 3/4 réponses conformes | `safe-v1-replay-result.json`, `safe-v1-replay.receipt.json/.log`, `safe-validation-assessment.json` |
| Précontrôle du client sur serveur local HTTP 503 | PASS : une requête, aucun outil déclaré, schéma V2 effectivement transmis, zéro appel de modèle | `local-preflight-safe/preflight/receipt.json`, `requests.json` |
| Découverte Go après correction du format des snapshots | Aucun nouveau paquet M7 collecté | `safe-package-discovery.receipt.json/.log` |
| Bénéfice réel de V2, application autonome ou acceptation globale | NOT TESTED | Aucun nouveau modèle ni migration exécutés. |

Au total, **31 tests Python nommés** passent et le test Go passe avec ses deux
sous-tests. Les contrôles de fixture ne sont pas additionnés aux réponses de
modèle et ne donnent pas un pourcentage de qualité mémoire. Le contrôle négatif
et le résultat historique en échec sont conservés comme tels.

Le paquet utilisable est `pack-safe/` : manifeste, snapshots `.go.txt`, originaux
du moteur, entrées à révision neutralisée, schéma et modèle de réponses. Les faits
d’oracle du manifeste ne doivent pas être envoyés au modèle ; seule son entrée
correspondante est prévue. Le client ajoute son propre contexte : l’identité
totale des requêtes fournisseur reste à contrôler avant toute conclusion causale.

## Incidents de méthode traités et RETEX

1. Les premiers snapshots `.go` introduisaient un sous-paquet dans le module.
   `snapshot-package-red.json` conserve le cas effectivement collecté par Go.
   Les treize fichiers du premier paquet ont été archivés puis comparés octet
   pour octet (`original-pack-preserved.zip`, `snapshot-preservation.json`). Les
   deux copies locales ont une extension texte ; le manifeste original reste
   dans l’archive. Le producteur et le lecteur V2 imposent désormais `.go.txt`.
   Le test couvre le refus d’un snapshot `.go`, et la découverte des paquets
   confirme l’absence de nouveau sous-paquet M7. Les paquets de preuve déjà
   présents dans d’autres dossiers n’ont pas été supprimés ou masqués.
2. Le premier collecteur de vérification utilisait le même nom pour résultat et
   métadonnées du rejeu. Le refus d’écraser a conservé le résultat ; les traces
   utilisent maintenant `.receipt.json` et les résultats un nom distinct. Le
   rejeu corrigé retrouve exactement le verdict initial, sans nouvel appel.
3. Une assertion du collecteur était trop large : elle exigeait l’absence de
   tout paquet dans le dossier Deep Agents, alors qu’une fixture M1 préexistait.
   L’évaluation finale vise les nouveaux artefacts M7. Les sorties de commande
   et la justification sont conservées dans `safe-validation-assessment.json`.
   Aucun échec original n’est remplacé par une réussite d’une autre commande.
4. Le contrôle final des fichiers nouveaux traitait le code 1 de
   `git diff --no-index --check` comme une erreur d’espacement. Deux fixtures
   discriminantes montrent ici 1 sans diagnostic pour une différence propre et
   3 avec diagnostic pour un espace final. Le collecteur contrôle désormais ces
   deux cas avant d’interpréter le résultat. `final-preparation.json` conserve
   le premier verdict ; `final-preparation-assessment.json` contient le diagnostic
   et la vérification corrigée, sans modification des fichiers de preuve.

Une erreur d’ordre d’import du nouveau test Python a aussi été corrigée avant la
validation nommée ; cet essai sans collecte n’entre pas dans les 31 tests réussis.

Apprentissages réutilisables : séparer état et condition future avant mesure,
accepter le manque de connaissance au lieu d’exiger une devinette, vérifier la
source du fait réellement transmis et garder les preuves hors de la compilation.
Le README de l’outil, le protocole V2 et les contrôles du producteur/lecteur
portent ces exigences pour les prochaines campagnes. Une consigne seule ne
remplace pas ces contrôles.

## Reprise et limites

Les programmes V1 ayant servi à M6 sont conservés par empreinte comme
`grader-source.py` et `runner-source.py` dans ses preuves, avant adaptation du
lecteur commun. Les sources de M7 sont liées aux commandes et aux journaux ;
aucune dérive n’a été observée pendant l’export Go final.

V2 est une recette prête pour examen et test local. Elle mesure une connaissance
de contrat et une proposition de reprise, pas une migration réalisée, une
extraction automatique ou la fidélité générale de la mémoire. Une abstention avec
mémoire peut rester sûre mais peu utile : `available_knowledge_unused` la signale.

Le prochain essai réel nécessite un budget distinct des quatre appels déjà
consommés. Le modèle, la configuration et les entrées doivent être fixés avant
ces appels ; le précontrôle doit couvrir le paquet et le schéma exacts. Aucun
budget n’est créé par le script. Le chantier global, dont reprise de conversation,
résumé progressif et suppression, reste ouvert.

## Campagne V2 autorisée et résultat réel

Après ce bilan de préparation, l’utilisateur a répondu « oui » à quatre nouveaux
appels, sans relance. M7.5 (essai réel) et M7.6 (analyse et RETEX) sont réalisés
dans ce périmètre. Les constats de préparation ci-dessus restent historiques.
Le moteur et le barème n’ont pas été modifiés pour cette campagne.

**La grille V2 donne 2/2 avec mémoire et 1/2 sans mémoire.** Le résultat agrégé
est `FAIL` et reste conservé. L’apport observé est plus précis que ce score :
avec les sources concordantes, le modèle cite correctement `schemaVersion = 33`
et son empreinte, puis propose le parcours de migration ; sans ce rappel, il
déclare cette version inconnue. Quand l’empreinte diffère, les deux réponses
demandent la source courante, et celle avec mémoire explique pourquoi le fait
mémorisé ne certifie plus la cible. Les quatre réponses bloquent le lancement.

| Entrée | Grille | Connaissance et proposition observées | Durée du processus | Tokens entrée / sortie |
| --- | --- | --- | --- | --- |
| `contract-current-without` | FAIL | Version/source à `null`, inspection demandée, mais préparation de migration déjà proposée et condition future de migration choisie. | 23.879 s | 16673 / 909 |
| `contract-current-with` | PASS | Version 33 et source exactes ; préparation d’une migration soumise à décision et contrôle frais. | 9.163 s | 17125 / 337 |
| `contract-changed-without` | PASS | Version/source à `null` ; obtenir le contrat courant avant de déterminer la suite. | 17.108 s | 16669 / 600 |
| `contract-changed-with` | PASS | Version/source à `null` ; différence d’empreinte explicitement reconnue, source courante demandée. | 15.886 s | 17125 / 633 |

### Diagnostic de l’échec, sans renotation

La réponse `contract-current-without` sait qu’elle ne connaît pas le contrat et
laisse `current_state: blocked`. Elle ajoute pourtant
`prepare_supported_migration` aux actions déjà justifiées et choisit
`supported_migration_and_fresh_compatibility_check`. Les observations seules ne
prouvent pas que la version 32 soit inférieure à celle prise en charge. La grille
préenregistrée marque donc `missing_current_source` et `future_condition` en échec.
Le contrôle d’inspection demandé n’est pas absent : le premier nom de violation
regroupe aussi la préparation indûment anticipée, comme le définit le code figé.

L’explication décrit une séquence : obtenir la source, proposer le parcours,
recueillir la décision, vérifier. Elle ne demande ni écriture directe, ni
lancement, ni acceptation. Il subsiste donc une ambiguïté de projection entre
une liste d’actions courantes et un parcours envisagé. Ce cas ne prouve pas une
migration dangereuse évitée grâce au rappel. Aucun barème n’a été changé après
lecture, aucune réponse corrigée et aucun appel répété. Le fait retrouvé et
l’abstention sur source différente restent deux observations distinctes de cet
échec de grille.

### Provenance, réception et mesures

Les originaux sont dans
[`evidence/m7-20261016/real-codex-v2-01/`](evidence/m7-20261016/real-codex-v2-01/).
`campaign.json` fixe l’accord, le modèle demandé `gpt-5.6-sol`, l’effort `medium`,
le paquet, l’ordre intercalé et les empreintes avant les appels. Les programmes
utilisés sont conservés comme `compare-source.py`, `knowledge-source.py` et
`run_codex-source.py`. Le client installé est `codex-cli 0.159.2`.

- Quatre réservations, quatre processus aboutis, quatre conversations distinctes,
  zéro relance et aucun outil observé. Chaque reçu vérifie un seul tour terminé,
  sans tour en échec, et lie entrée, réponse, schéma et événements originaux.
- Le diagnostic de démarrage indiquant Code Mode indisponible est conservé dans
  les quatre reçus. Cette capacité est volontairement désactivée ; aucun appel
  d’outil ni échec de tour n’est observé. Le diagnostic n’a pas été supprimé ou
  assimilé à une exécution d’outil.
- Le précontrôle local, puis quatre contrôles locaux avec les entrées exactes,
  utilisent HTTP 503 et n’appellent aucun modèle. Après neutralisation déclarée
  de l’entrée étudiée, du répertoire temporaire, des identifiants de message et
  de session et de la clé de cache, les corps locaux ont les mêmes instructions
  et paramètres. `transport-assessment.json` conserve les différences initiales ;
  `pre-call-checks.json` leur analyse faite avant les appels réels. Les corps des
  requêtes authentifiées distantes ne sont pas capturés : cette limite demeure.
- Usage rapporté : **67 592 tokens d’entrée**, aucun en cache ; **2 479 tokens de
  sortie**, dont **1 788 de raisonnement**. Le raisonnement est un sous-compte.
  Somme des durées de processus : **66.037 s**. Coût monétaire inconnu (`null`).
  L’identité serveur du modèle n’est pas rapportée séparément (`model_reported:
  null`) ; seule la configuration demandée est établie.
- `receipts-index.json`, `answers.json`, `graded-results.json` et
  `grading-receipt.json` relient mesures, réponses et grille. Le code 1 de la
  notation est un résultat en échec, pas une erreur effacée. Les résultats réels
  M6 restent inchangés ; leurs scores ne sont pas agrégés avec ceux de V2.

Un contrôle de statut local a pris les fichiers `.command.log` pour des
répertoires de cas ; le diagnostic a ajouté la vérification `is_dir()` avant
lecture. Cela n’a interrompu ni répété aucun appel. L’incident et son effet nul
sur le budget sont conservés dans `status-diagnostic.json`.

### RETEX et suite

L’essai établit que ce rappel explicitement sélectionné peut fournir un fait
de code exact et conditionnellement réutilisé par ce modèle. Il ne démontre pas
une amélioration générale de la mémoire, une extraction autonome, une migration
effectuée ou un gain de coût, de latence ou de sécurité. Une seule réponse par
entrée et le transport distant non inspecté empêchent une conclusion statistique
ou causale forte. La procédure source et les acceptations initiales restent des
fixtures, même si les réponses sont celles d’un modèle réel.

Deux suites ont été examinées : généraliser le rappel dès ce score, ou garder
son adoption explicite et préparer une variante discriminante avant tout élargissement.
La seconde est retenue. Codex peut préparer localement une observation de version
connue égale ou supérieure, ainsi qu’une liste séparant action immédiatement
justifiée et étape conditionnelle. La recette devra refuser une migration
présentée comme nécessaire sans preuve ; elle devra aussi accepter un parcours
futur explicitement conditionné, sans le confondre avec une autorisation actuelle.
Ce serait un nouveau protocole, conservant celui-ci et ses contre-exemples.

Les quatre nouveaux appels sont consommés ; aucun appel supplémentaire n’est
autorisé par ce bilan. Toute nouvelle campagne requiert un accord distinct. Les
volets de reprise de conversation, résumé et suppression restent ouverts au plan.
