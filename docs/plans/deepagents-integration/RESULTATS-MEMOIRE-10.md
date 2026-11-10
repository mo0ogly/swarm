# Reprise explicite d'une conversation arrêtée

16 octobre 2026, autorisation « go » après M13.
Plan : [INCREMENT-MEMOIRE-10.md](INCREMENT-MEMOIRE-10.md).
Codex seul, candidat local ; aucun appel de modèle réel ni déploiement.

## Contrat livré

Une demande explicite crée une nouvelle tentative, une identité et une
réservation propres, dans la même conversation fournisseur. Elle conserve
l'ancienne exécution et ses coûts. Le demandeur fournit la révision, les
empreintes du checkpoint et des faits examinés, une confirmation, un motif
et une nouvelle consigne. La transaction normale applique les permissions,
dépendances, limites et réservations ; elle revérifie la source après la sonde.

La décision est attribuée à l'opérateur local authentifié. L'aperçu ne crée
aucune décision ; le reçu de demande indique `restoration_confirmed: false`.
Le processus recontrôle le contrat et le précontrôle avant démarrage. Seuls la
session attendue et un nouveau tour terminé peuvent confirmer la restauration.
Un refus, une session différente ou une fin manquante n'entraînent pas de
relance automatique. Le redémarrage de l'ancienne ligne d'agent reste interdit.

Les gardes et usages du checkpoint restent cumulatifs, y compris les vingt
échanges déjà autorisés par le wrapper. Les compteurs et usages publics du nouvel
agent décrivent sa seule exécution : les totaux de mission ne recompteront pas
l'histoire. La durée conserve son contrat existant par lancement ; les durées
antérieures restent enregistrées. Les copies Git gérées sont explicitement
refusées et restent dans leur parcours de reprise de production.

Le stockage passe à 35 avec sauvegarde privée : un ancien programme doit refuser
ce candidat plutôt que lancer une restauration comme une conversation neuve.
La migration est testée uniquement dans des racines temporaires. Le serveur
utilisateur est conservé et ne reçoit pas ce binaire.

## Vérifications

Toutes les preuves ci-dessous sont sous `evidence/m14-20261016/`. Les reçus
conservent commande, date UTC, durée, code de sortie, entrées empreintées,
absence de dérive pendant le contrôle et empreinte du journal original.

| Contrat ou contrôle | Résultat | Preuve |
| --- | --- | --- |
| Opération publique absente avant implémentation | FAIL reproduit : un test exécuté | `restore-red.json/.log`, source dans `preimages/` |
| Reprise, gardes, usages, deux adaptateurs scriptés, refus, concurrence, migration, lecteurs, coûts et lancement existant | PASS : 41 tests nommés | `restore-final.json/.log` |
| Restauration, concurrence, confirmation et instantanés sans alias | PASS : huit noms inclus dans les 41 | `restore-race.json/.log` |
| Lectures d'inspection et API authentifiée, après correction de leur classement | PASS : quatre noms, dont un supplémentaire ; 42 noms distincts au total | `storage-final.json/.log` |
| Quatre commandes natives sur un stockage 34 | FAIL avant, PASS après : version 34 conservée, zéro sauvegarde et refus `storage_upgrade_required` | `preview-storage-red-results.json`, `preview-storage-green-results.json` |
| Compilation du candidat et analyse statique | PASS | `build-release.json/.log`, `static.json/.log` |
| Rendu existant, traductions et contrat des méthodes | PASS | `web-unit.json/.log`, `i18n.json/.log`, `config.json/.log` |
| CLI, superviseur, API et navigateur compilés, reprise explicite FR/EN | PASS : neuf contrôles, quatre invocations locales, zéro modèle réel | `browser-final.json/.log`, `browser-final/result.json` |
| Session d'un vrai compte fournisseur, revue indépendante, déploiement | NOT TESTED | Aucun appel réel, agent délégué ou changement du serveur utilisateur |

Les 41 tests et les huit contrôles de concurrence précèdent uniquement les
modifications de `storage_inspection.go` et de son test. Leurs autres entrées
restent identiques, sauf une normalisation finale de trois lignes Go décrite
ci-dessous. Les quatre tests ciblés finaux, le binaire final et les
quatre commandes natives vérifient ce changement de classement. On ne présente
pas les anciens tests comme exécutés sur des sources inchangées dans leur totalité.
Le contrôle de formatage final relève cette écriture compacte dans
`agent_terminal_process.go`. `gofmt` la normalise ; la source antérieure est
conservée et sa forme canonique est identique à la source finale. La preuve
`formatting-equivalence.json` attribue précisément ce changement de présentation.
Le binaire et les tests précèdent ce seul formatage, sans changement de comportement ;
leurs empreintes originales ne sont pas remplacées. L'audit vérifie cette
équivalence et toutes les autres différences d'entrées explicitement.
Le reçu `audit-final.json` vérifie aussi les journaux, les sources des échecs,
les anciens résultats inchangés, les liens et le serveur conservé. Le diff
`candidate.diff` part des préimages M13, sans englober le travail antérieur
sans rapport présent dans le checkout. La revue du candidat reste locale au
contexte de son auteur ; aucun verdict indépendant n'est fabriqué.

Les refus couvrent confirmation absente, faits/checkpoint/révision périmés,
source active ou non confirmée, capacités non vérifiées, tentative ou budget
épuisés, source modifiée pendant la sonde et contrat modifié avant départ.
Deux demandes concurrentes créent une seule exécution. Le rejeu du même événement
retourne la même identité ; un contenu différent est refusé. L'API vérifie session,
CSRF, périmètre de travail, schéma strict et absence de mutation du travail lors
de l'aperçu. Un JSON fortement échappé, plus grand que 65536 octets mais sous la
borne de transport existante de 128000, respecte les limites des champs décodés.

Le navigateur demande une reprise en anglais puis en français. Chaque demande
crée une identité distincte, reprend la session attendue et conserve le checkpoint
source. Les usages cumulés passent de 10 à 20 puis 30 jetons d'entrée ; chaque
nouvel agent n'en rapporte que 10. Le rejeu API ne crée pas de doublon. Une
quatrième invocation, demandée par la vraie CLI, atteint la garde cumulée de
quatre outils et s'arrête sans cinquième appel ni frontière faussement confirmée.
Les estimations historiques restent conservées ; le refus pour budget épuisé
précède toute nouvelle invocation. Le changement de budget passe par l'opération
publique de la seule fixture, sans modifier sa consommation.

La recette produit 28 captures : diagnostic positif et erreur de cohérence,
formulaire FR/EN dans les deux thèmes, puis demandes réussies FR claire et EN
sombre. Les largeurs 1440/760/390/320 sont exercées pour le diagnostic, 1440/320
pour le formulaire et les reçus. Échap restaure le focus ; une confirmation
absente focalise la case ; les refus conservent la saisie. Les demandes réelles
sont cliquées à largeur bureau. Aucun débordement horizontal de la modale n'est
observé aux largeurs vérifiées. Les captures FR claires à 320 px et EN sombre
à 1440 px ont été inspectées ; la modale conserve son défilement vertical.

Les sept HTTP 400 attendus sont distingués des erreurs inattendues ; aucune
erreur de page ni requête échouée inattendue n'est observée. Six lectures annulées
pendant une navigation restent attribuées séparément. Le contraste du diagnostic
est mesuré à au moins 4,5:1 ; cette mesure ne constitue pas un audit séparé de
chaque état du nouveau formulaire.

## Replan et RETEX

Le contre-exemple initial confirme l'absence de l'opération. Le premier contrôle
après implémentation échoue dans deux variantes de la fixture : les arguments
du fournisseur étaient consignés deux fois par le script. La correction retire
cette double écriture ; l'assertion exige toujours exactement deux reprises.
Sources et sorties initiales restent dans `restore-first-green*` et `preimages/`.

La recette native révèle ensuite quatre migrations implicites lors de lectures
(`mission recovery`, `mission changes`, `mission spending` et l'aperçu de reprise).
Le classement commun d'ouverture du stockage omettait ces commandes. Les ajouter
au contrat d'inspection corrige tous leurs appels, sans exception d'identifiant.
Les contre-exemples originaux restent conservés, ainsi que les quatre refus
natifs après correction. Aucune base utilisateur ne sert de fixture.

La première recette navigateur échoue sur un bouton recréé par l'actualisation
du cockpit. Le localisateur retrouve le bouton au moment du clic. La suivante
consulte l'accordéon avant son rendu : l'attente explicite de son sélecteur
devient la précondition. Les annulations de lectures pendant une navigation
de cette fixture sont conservées à part, uniquement pour les routes de suivi
identifiées et `net::ERR_ABORTED`. Tout autre échec réseau reste bloquant.
Les échecs, captures et scripts exacts restent dans `browser-preliminary*`,
`browser-current*` et `preimages/`. Les demandes de mutation ne sont pas relancées
automatiquement pour contourner une erreur.

La compilation avec détection de concurrence est longue mais ses processus
de compilation consomment du CPU, puis les huit tests s'exécutent et passent.
Ce constat a évité une relance identique. Quelques lectures ont supposé un nom
de document absent ; les noms réels ont été repris sans toucher au produit.

La leçon réutilisable est appliquée dans les producteurs : attribuer la décision
à des faits frais, revérifier après une opération lente et dans la transaction,
puis distinguer l'état cumulé de conversation de l'usage facturable de chaque
exécution. La garde de version protège aussi les anciens lecteurs. Une lecture
doit refuser une incompatibilité sans migration. Les tests et la documentation
FR/EN portent ces contrats communs ; ils ne prouvent pas une autonomie métier.

## Documentation et limites conservées

[PROJECT-MEMORY FR](../../PROJECT-MEMORY.md) et
[PROJECT-MEMORY EN](../../en/PROJECT-MEMORY.md) expliquent décision, schéma,
commandes, API, panneau, précontrôle, filiation, gardes, consommation et erreurs,
avec un diagramme de séquence. La recette existante
`tests/dialogue_recovery_ui.cjs` accepte `--integrity --restore` et un binaire
isolé ; elle crée ses propres racines et n'utilise pas le serveur utilisateur.

MEM-C15 et MEM-C16 restent **PARTIAL** : les doubles locaux vérifient les
adaptateurs, pas la disponibilité/authentification d'une session réelle, ni la
réconciliation d'effets d'outils externes. L'historique fournisseur est repris
tel quel ; aucune projection ou réduction structurée de ses messages n'est
ajoutée. Une tentative échouée avant une nouvelle frontière ne fabrique pas de
checkpoint et doit être examinée. Résumé, suppression et qualification LLM
restent des travaux ultérieurs du plan mémoire. Les huit évaluations réelles
M6/M7 restent les seules réalisées, avec leurs résultats historiques inchangés.
