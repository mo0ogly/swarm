# M19 — sélection extractive reçue par un worker

16 octobre 2026, reprise « ok next ». Les cinq lots de
[l'incrément 15](INCREMENT-MEMOIRE-15.md) sont réalisés et vérifiés localement.
Une sélection explicite de citations de cadrage est maintenant admise dans un
premier lancement manuel de worker hors hiérarchie, après aperçu confirmé.
La vraie CLI/API, le superviseur et un fournisseur scripté vérifient le contexte
effectivement reçu. Le résumé automatique général et sa qualité LLM restent ouverts.

## Décision et réalisation

Importer le reçu de référence Python comme autorité aurait introduit un second
contrat d'admission, ainsi que l'instantané complet destiné à l'opérateur. Le
moteur accepte donc une déclaration `summary_selection` : schéma, empreinte M18,
références, frontière, protections et motif. Les textes sont reconstruits depuis
le `Work` courant ; aucune prose libre de résumé n'est reçue. L'instantané de
tâche est contrôlé mais exclu du texte injecté. Le contexte courant de la tâche,
les règles du projet, la mémoire versionnée et la reprise suivent leurs voies
normales. Le nouveau format n'est pas une conversation fournisseur reconstruite.

`memorySummarySourceForWork` réutilise le producteur M18 sur l'instantané de la
transaction de lancement. Identité, UTF-8 et révision colonne/corps sont contrôlés.
L'admission refuse source changée, référence invalide ou dupliquée, frontière
incorrecte, protection absente/dupliquée/inconnue, état non pris en charge et
motif vide. L'opérateur doit protéger l'instantané et au moins un message de
dialogue du préfixe. Les citations protégées du préfixe et le suffixe sont joints
comme données historiques, avec identifiants des omissions ; les originaux restent
conservés. Les états déclarés ne prouvent ni leur justesse ni un droit d'exécuter.

La CLI `agent context-preview TRAVAIL --input lancement.json` et l'action API
authentifiée `agent-context-preview` utilisent le même préparateur. L'aperçu ne
réserve rien et refuse un stockage ancien sans migration. Le départ exige
`context_hash` ; l'empreinte du choix complet est aussi dans le prompt, afin
de lier motif et annotations à la confirmation. L'API refuse un choix transmis
à une opération qui ne le consomme pas. Aucune commande inconnue n'est traitée
comme un aperçu réussi.

Le manifeste `context.summary` conserve sélection, empreinte, révision préparée,
sources reçues/omises, empreinte et taille de la pièce jointe. La demande et les
événements existants portent le choix ; aucune table ou version de stockage
n'est ajoutée. Le profil réutilisable ne l'hérite pas. La mise en file lie l'état
après les transitions normales de lancement à `queued_revision` et
`queued_source_sha256`. Juste avant `cmd.Start`, le superviseur relit ces sources,
enregistre `checked_before_start_at` / `checked_source_sha256`, puis lance.
Une dérive ou une impossibilité d'enregistrer la preuve interdit le fournisseur.
Le reçu préparé reste conservé ; le manifeste courant lie le prompt complet,
y compris l'ajout de reprise moteur, à ses octets réellement transmis.

`tools/memory-evaluation/admission.py` prépare le choix depuis paquet, annotations
et reçu M17 initial PASS. Il reconstruit la référence attendue et compare le
reçu entier, conserve les entrées et écrit exclusivement un fichier privé. Un
reçu parent, FALLBACK, altéré, périmé ou sans protection de dialogue est refusé.
Code 0 signifie préparation, sans admission moteur. Le choix reste inspectable
et peut être formulé directement par l'opérateur ; son origine Python n'est pas
une autorité spéciale du moteur.

## Preuves

Dossier : [`evidence/m19-20261016/`](evidence/m19-20261016/). Baseline, extension
de périmètre, préimages, arguments, codes, sorties et empreintes sont conservés.

| Critère | Résultat et preuve |
| --- | --- |
| M19.1 contrat public avant réalisation | Test rouge nommé : champ `summary_selection` absent, `selection-red.json/.log`. |
| M19.2 validation et absence de fuite | PASS, onze nouveaux tests Go, vingt variantes de choix invalides, fraîcheur, confirmation et stockage incohérent ; `contract-final.json/.log`. |
| M19.3 aperçu CLI/API et décision liée | PASS, parité, refus sans mutation, authentification/origine, ancien stockage non migré ; vraie confirmation avec empreinte. |
| M19.4 contexte réellement transmis | PASS, sous-processus scripté, réouverture, source périmée en file bloquée, erreur de reçu bloquante et récupération de fixture après retrait de la panne injectée. |
| Concurrence et rejeu | PASS, un seul agent créé ; conflit de révision concurrent possible, rejeu ultérieur idempotent. Deux tests ciblés avec `-race`, `selection-race.json/.log`. |
| M19.5 préparation et non-régression | PASS, 49 tests Python : cinq préparation, 24 référence, huit convertisseur et douze stockage ; `source-reference-regression.json/.log`. |
| Parcours applicatif complet | PASS, cinq contrôles M19 et cinq contrôles M18 réutilisés, `native-final/result.json` et `native-final/source-flow/result.json`. |
| Compilation, statique, configuration, diff et environnement | PASS, `build.json`, `static.json`, `configuration.json`, `audit-final.json` et `candidate.diff`. |

**21 tests Go distincts** passent : onze du nouveau contrat, six M18 et quatre
contrôles existants (contexte, reprise, lancement atomique, cycle mémoire public).
Les deux noms rejoués avec `-race` ne s'y ajoutent pas. Les variantes internes
ne sont pas comptées comme de nouveaux tests. Les **49 tests Python** incluent
douze cas de stockage collectés par le filtre `test_memory_s*.py`, plus large
que les seuls modules de résumé ; leur portée reste distincte de la qualité LLM.
Les essais préliminaires et rouges restent historiques, sans ajout aux comptes.

La recette finale commence par les cinq contrôles M18 inchangés : deux cadrages
réellement enregistrés via le fournisseur scripté, export CLI/API et conversion.
Elle prépare un choix depuis une référence M17 en laissant volontairement une
réponse du préfixe sans protection. L'aperçu contient trois messages de dialogue
reçus et une omission attribuée, sans instantané opérateur injecté. CLI/API
produisent le même prompt. Motif modifié après aperçu et confirmation manquante
sont refusés sans agent ni appel. Une modification publique des critères produit
409 sur l'ancienne requête ; les sources et la sélection sont préparées à nouveau.

Le départ courant via API crée un seul worker. Ses citations, états déclarés et
suffixe correspondent exactement à la référence ; son entrée de sous-processus
égale le prompt relu par CLI et son SHA-256. Le contrôle avant démarrage est
enregistré et le reçu préparé n'est pas réécrit. Le rejeu de la même demande
retrouve l'agent sans nouvel appel. Les tâches source sont identiques, le profil
publié sous `launch_profile` ne contient pas de sélection et la tâche n'est pas
acceptée implicitement.

Cette recette compte **trois invocations scriptées**, deux sources et un worker,
et **zéro modèle réel**. La première recette échouée a également produit trois
invocations scriptées, soit six pour les deux recettes applicatives conservées ;
les sous-processus des tests Go sont des fixtures séparées. Aucun compteur réel
ni les huit appels historiques M6/M7 ne sont réinitialisés.

Le binaire utilisé est celui de `build.json`, construit par `go build ... ./cmd/swarm` ;
son empreinte est liée à `native-final/result.json` et à `audit-final.json`.
Les sources Go et ressources sont inchangées entre tests, construction et recette.
Le changement ultérieur du seul script natif est évalué comme entrée non consommée
par les tests Go/Python et la construction ; les originaux restent conservés.
Pas de suite Go globale, test de navigateur, déploiement ou fournisseur réel.
Le serveur utilisateur demeure prêt au PID 2186583, même adresse, sans migration.

## Replan et RETEX

La revue reproduit deux défauts du candidat, avant leur correction :

- Modifier le motif après aperçu ne modifiait pas les citations, donc l'empreinte
  de prompt. `confirmation-red.json/.log` et préimages conservent cette admission
  incorrecte. L'empreinte du choix complet est maintenant dans la pièce jointe ;
  le même test et la vraie API refusent la modification.
- Le nouveau consommateur du producteur M18 acceptait un `Work` déjà décodé,
  sans ses gardes de représentation. Un octet UTF-8 invalide pouvait devenir le
  même caractère de remplacement et une divergence colonne/corps rester invisible.
  Les deux variantes de `stored-boundary-red.json/.log` reproduisent l'admission.
  La transaction contrôle désormais octets, identité et deux révisions avant de
  reconstruire le contexte. Les mêmes cas refusent sans mutation.

Les échecs de méthode restent distincts. Un script d'édition supposait un seul
`cmd.Start` ; deux points existaient, dont le lancement du superviseur. Il s'est
arrêté après des éditions partielles. L'inventaire puis un ancrage sur le bloc de
supervision réel ont terminé le raccordement, sans modifier les sources pendant
compilation. Un nouveau test employait `Planning` au lieu du type producteur
`PlanningState` : `selection-contract.json/.log` conserve l'échec de compilation,
sans test exécuté. La correction du type précède les contrôles nommés.

Le premier test de concurrence exigeait deux réponses immédiates réussies. Le
contrat existant refuse volontairement une révision devenue périmée entre deux
lectures. `selection-final.json/.log` conserve ce FAIL. Le test vérifie désormais
le seul agent créé, le conflit permis et le rejeu public qui retrouve cet agent ;
aucun garde n'a été affaibli. Les contrôles finaux et `-race` passent.

La première recette native atteint et vérifie l'entrée fournisseur, puis emploie
la clé supposée `profile`. Le producteur publie `launch_profile` ; l'état relu et
`native/failure.json` prouvent cette cause, sans défaut de lancement. La préimage
est conservée. Le nom réel est corrigé et la recette complète repasse dans une
nouvelle fixture, sans réessayer une erreur inchangée. Les lectures initiales
d'un fichier de reçu ou test web supposé absent ont été remplacées par recherche
des symboles ; les recherches trop larges ont été bornées aux fichiers Go.

Apprentissage réutilisable dans la recette commune : confirmer le choix complet,
garder les contrôles des octets et colonnes d’origine et respecter les noms
JSON du producteur. Une référence extractive n'est ni une admission d'agent ni
une preuve d'état courant. La revue est locale, dans le contexte de réalisation,
sans indépendance ni acceptation globale. Docs
[FR](../../PROJECT-MEMORY.md), [EN](../../en/PROJECT-MEMORY.md) et
[recette](../../../tools/memory-evaluation/README.md#sélection-extractive-admise-par-le-moteur)
décrivent le flux, les erreurs et la reprise ; aucun contexte d'agent n'est
modifié par une consigne documentaire seule.

## Limites et prochaine frontière

MEM-C16/MEM-C17 demeurent **PARTIAL**. La sélection extractive est consommée par
un vrai chemin moteur avec fournisseur scripté, sans paraphrase libre, qualité
LLM ou historique fournisseur complet. Le contrôle de fraîcheur est une
observation datée juste avant départ, sans verrou permanent ni révocation après
démarrage. Une panne de stockage qui empêche aussi l'enregistrement de fin reste
une erreur explicite ; sa récupération de fixture n'établit pas une autonomie
de réparation d'un stockage réellement indisponible.

Le lot refuse hiérarchie, conducteur, parent, tentative antérieure, terminal,
dialogue/restauration et références complètes simultanées. Aucun seuil de résumé,
politique automatique ou purge. La suite doit qualifier la portée du contexte
pour les autres parcours et la progression des états/incidents avant de les ouvrir,
puis définir toute politique dans Admin/API/CLI/Go et obtenir les droits d'essai
réel nécessaires. Les acquis, originaux et résultats antérieurs restent conservés.
