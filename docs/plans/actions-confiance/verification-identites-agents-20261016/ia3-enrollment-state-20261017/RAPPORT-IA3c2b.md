# IA3c.2b — état local durable, service protégé encore ouvert

17 octobre 2026 — **PASS du journal isolé ; aucune qualification globale d’identité.**

Le paquet `internal/enrollmentstate` conserve une chaîne d’enrôlements et
révocations signés sous clé/projet épinglés. Une lecture active réserve l’écrivain
SQLite et vérifie toute l’histoire contre un témoin fourni indépendamment.
Le journal est distinct des bases de mission et n’est importé par aucun lanceur.

## Décision et périmètre

Le [plan de reprise](../../identites-agents-replan.md) compare journal fichier
et SQLite ; SQLite réutilise la dépendance et l’atomicité existantes sans protocole
de fichiers artisanal. Le [contrat cible](../../identites-agents-enrolement-IA3c2.md)
et les contrats opérationnels [FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md)
/ [EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) décrivent les cadres, champs,
appels, erreurs et limites. Les corps restent opaques : aucune permission métier
n’est déduite d’un enregistrement actif. La clé privée n’est jamais persistée.
Pas de service, compte, principal humain externe, mission native, fournisseur
payant, déploiement ou revue indépendante.

| Critère | Preuve observée | Résultat et limite |
| --- | --- | --- |
| Lot normal final | [normal-04](normal-04-run.json) : 42 noms, 0 échec/ignoré | PASS local ; parents et sous-cas inclus |
| Même lot race | [race-04](race-04-run.json) : mêmes 42 noms | PASS local, aucune course signalée |
| Compilation et analyse statique | [build](build-04-run.json), [vet](vet-04-run.json) | PASS du paquet isolé, pas du serveur déployé |
| Sources stables | [7 entrées](source-final.json), avant/après chaque contrôle final | PASS ; 3 Go du paquet, 2 Go du protocole, go.mod/go.sum |
| Concurrence | [32 fixtures](concurrent-schedules-count.json), 8 écrivains chacune | Une ligne, au moins un succès par fixture ; contention explicite admise |
| Révocation | Lecture protégée empêche COMMIT concurrent ; après révocation aucune lecture active | PASS local ; pas de livraison ou départ distribué |
| Écriture interrompue par faute SQL | Trigger de fixture refuse la tête ; zéro événement partiel, reprise après suppression de la faute | PASS transactionnel ; pas une panne matérielle |
| Réouverture et retry exact | Création conservée, replays création/révocation reconnus à la seule tête courante | PASS ; ancien événement refusé après un événement plus récent |
| Retour à une histoire valide ancienne | Journal et tête SQL restaurés ; témoin récent conservé : refus | PASS conditionnel à l’indépendance du témoin |
| Limite du témoin restauré aussi | Même ancien état accepté avec son ancien témoin | Limite explicitement testée ; pas une protection acquise |
| Contextes et entrées invalides | Autre clé/projet, champs JSON invalides, types/null, IDs SQL paramétrés, annulation | Refus sans mutation ; SignActive ne retourne aucun octet sur erreur |
| Documentation et schémas | [4 rendus](diagrams.json), [2 PNG inspectés](visual-inspection.json) | FR/EN actualisés ; voir index final pour liens et empreintes |
| Conservation des tranches précédentes | [10 fichiers de référence](preservation.json) inchangés | Tests moteur précédents réutilisés historiquement, non rejoués/requalifiés |

Les commandes originales, heures, sorties JSONL/stderr et empreintes sont dans
ce dossier. Tous les tests ont `-timeout=0` : aucun watchdog global arbitraire.
Le lot de 32 fixtures est un échantillon de calendriers, pas une politique de
retry ni une preuve de tous les ordonnancements. Les sept entrées n’ont pas
changé pendant les contrôles finaux. Le lot concurrent 03 conserve ses propres
empreintes ; le code et le test sélectionnés sont identiques au candidat final,
seul un autre fichier de tests a ensuite reçu les cas SignActive.

## Échec original et récupération

Avant implémentation, [contract-red-01](contract-red-01-run.json) échoue à compiler
car l’API n’existe pas ; zéro test exécuté, aucune preuve comportementale.
Le premier candidat donne [14 noms PASS et 1 FAIL](normal-01-run.json) : huit
écritures concurrentes identiques retournent toutes une erreur, aucune ne réussit.
Les [sources originales](candidate-01-journal.go.txt) et le test original restent
archivés. La première sortie ne détaillait pas ces erreurs individuelles : la
cause précise de ce seul échec ne peut être établie rétrospectivement.

Le test reçoit un diagnostic conservant les erreurs lorsqu’aucun écrivain ne
progresse ; le [diagnostic instrumenté](contention-diagnostic-01-run.json) passe.
Ce résultat indique une dépendance à l’ordonnancement, pas une disparition du
besoin. Inspection du pilote installé : `BeginTx` est différé par défaut et
supporte `_txlock=immediate`. Le candidat initial réservait via un UPDATE no-op
sur un journal rollback : promotion et lecteurs peuvent empêcher la progression.
Cette explication reste partielle pour l’incident original faute d’erreurs détaillées.

Correction du protocole local : `BEGIN IMMEDIATE` avant toute observation, WAL
pour découpler lecteurs et COMMIT, `synchronous=FULL` sur chaque connexion.
Le mode WAL est exigé explicitement ; aucune dégradation silencieuse. Pas de
mutex bloquant global, boucle de retry, `busy_timeout` ou délai arbitraire.
Le cas initial passe désormais, ainsi que les 32 fixtures fraîches. Cela vérifie
l’effet attendu sur ces calendriers, sans promettre de disponibilité face à un
verrou hostile, une panne disque ou tous les interleavings possibles.

Deux lectures de fichiers ont aussi employé un chemin de preuve ou un glob
supposé inexistant ; l’inventaire réel `rg --files` a corrigé les chemins avant
lecture. Elles n’ont ni modifié les sources ni servi de preuve de test.

## Garanties et limites à transmettre

- Le témoin `Head` vient du contrôle et doit rester hors import/journal. Le paquet
  ne le persiste pas indépendamment ; restaurer aussi ce témoin retire la détection.
  Le transfert atomique journal/témoin externe et son bootstrap restent ouverts.
- L’identifiant n’est créé qu’une fois ; révocation irréversible dans cette histoire.
  Une nouvelle génération nécessite un nouvel ID et une nouvelle décision.
- `WithActive` sérialise uniquement le journal local. Son callback ne doit pas
  réentrer, attendre un autre écrivain ni produire d’effet externe irréversible :
  une erreur de fin de transaction reste possible après l’appel du callback.
- `SignActive` signe `head` et `record` pour le challenge ; une réponse authentique
  peut devenir ancienne après libération du verrou. Pas de garantie de fraîcheur
  au moment d’une publication, d’une création ou d’un démarrage moteur.
- Le système de fichiers local doit prendre en charge WAL ; protéger aussi ses
  fichiers auxiliaires. Champs privés Go et permissions du même UID ne constituent
  pas une isolation. Aucun confinement ou service réel n’a été installé.
- Une lecture vérifie toute l’histoire : coût croissant, aucune mesure de grande
  échelle ni coupure de volume inventée. La séquence int64 est une représentation,
  pas un budget métier. La signature ne chiffre pas les corps.

## Améliorations, propriétaire et condition de reprise

| Découverte / origine | Valeur et périmètre | Propriétaire / état / critère |
| --- | --- | --- |
| I2b-01, premier candidat concurrent | Éviter absence de progression et promotion de verrous ; journal local | Codex ; réservation immédiate/WAL réalisés et vérifiés sur scénarios retenus ; cause initiale partiellement isolée |
| I2b-02, première erreur peu détaillée | Conserver la cause de chaque refus quand tous échouent ; méthode de test | Codex ; diagnostic enrichi réalisé ; ne pas reconstruire les erreurs historiques absentes |
| I2b-03, restauration cohérente | Distinguer chaîne authentique et actualité indépendante ; futur contrôle | Codex/opérateur ; limite vérifiée, témoin externe protégé à réaliser ; reprise après choix de frontière et protocole durable |
| I2b-04, relecture intégrale | Préserver validation sans plafond caché ; coût croissant | Codex ; constat documenté, optimisation différée jusqu’à mesures représentatives et invariants équivalents |
| I2b-05, callback avant fin de transaction | Éviter confondre observation et effet irréversible | Codex ; limite documentée et annulation testée ; futur raccord doit fournir son point de linéarisation |

Le prochain contrat commun est le schéma strict de l’autorité (principal OIDC
complet, portée, permissions, délégation, génération, dates), vérifié des deux
côtés ; il ne doit pas être remplacé par un booléen actif. L’activation requiert
le choix opérateur de frontière et du principal initial, la clé réellement épinglée,
le témoin durable et le raccord final au moteur. F01–F07, IDA, IA3 public,
IA4 mémoire, IA5 et RETEX global restent ouverts. Ce rapport clôt seulement les
critères locaux du journal, pas le besoin global ni l’acceptation indépendante.
