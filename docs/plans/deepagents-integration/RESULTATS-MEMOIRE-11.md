# Retrait et mémoire héritée d'une conversation

16 octobre 2026, reprise « next » ; plan
[INCREMENT-MEMOIRE-11.md](INCREMENT-MEMOIRE-11.md).
Codex seul, candidat local ; MEM-U04/MEM-U08/MEM-U14 et MEM-C18.

## Défauts reproduits et changement

Deux contre-exemples échouent avant correction : une reprise est acceptée après
désélection de la leçon, et le catalogue masque une version reçue par un ancêtre
de l'agent actif. Une session fournisseur conserve cet ancien contexte même
quand le manifeste du dernier lancement ne le mentionne plus. Les sorties
originales et le test exact sont conservés sous `evidence/m15-20261016/`.

Le lecteur partagé suit désormais les identités, tentatives, empreintes d'état
et racines de la chaîne de restauration. Il collecte les références héritées,
sans doubler une même version reçue plusieurers fois. Le catalogue et les aperçus
de retrait les attribuent à l'agent actif. Une filiation incohérente ou inaccessible
provoque une erreur ; aucun succès vide n'est fabriqué.

Pour reprendre la session, toutes les références héritées doivent encore être
valides et présentes dans la sélection courante. Le diagnostic bloque la reprise
incompatible ; les demandes construites directement restent soumises au moteur.
Le contrat est revérifié après la sonde dans la transaction, puis avant départ.
La lecture des agents actifs ferme son curseur avant les lectures de filiation,
pour respecter la connexion unique du Store.

La désélection ne purge pas l'historique fournisseur. Une version encore valide
peut être explicitement sélectionnée à nouveau ; si elle doit être écartée,
le responsable doit choisir une nouvelle conversation. Les conversations déjà
actives ne sont pas arrêtées par un retrait et leurs tours ne sont pas revalidés
par ce lot. Versions, prompts, checkpoints, consommations et preuves historiques
restent conservés. Le stockage reste 35 ; aucune politique de purge n'est ajoutée.

## Vérifications attribuées

Les fichiers de preuve sont sous `evidence/m15-20261016/`. Chaque reçu conserve
commande, date, durée, noms exécutés, code de sortie, empreintes des entrées avant
et après le contrôle et empreinte du journal original.

| Contrôle | Résultat | Preuve |
| --- | --- | --- |
| Désélection puis reprise, usage hérité masqué | FAIL avant correction : deux noms exécutés | `memory-lineage-red.json/.log`, `preimages/memory_lineage_red_test.go.txt` |
| Nouveaux contrôles, dont six variantes de filiation invalide | PASS : cinq tests nommés | `memory-lineage-green.json/.log` |
| Mémoire, sélection/retrait, contexte, diagnostic et restauration existante | PASS : 33 noms distincts, dont les cinq précédents | `memory-final.json/.log` |
| Concurrence et filiation après sonde | PASS : quatre noms inclus dans les 33 | `memory-race.json/.log` |
| Compilation, analyse statique, rendu, traductions et méthodes | PASS | `build`, `static`, `web-unit`, `i18n`, `generated`, `config` : reçus et journaux |
| Vraies CLI/API/interface, désélection puis reprise et retrait | PASS : neuf contrôles, deux invocations locales, zéro modèle réel | `browser-flow.json/.log`, `browser-flow-metadata.json`, `browser-flow/result.json` |
| Session réelle, purge fournisseur, résumé et revue indépendante | NOT TESTED | Aucun modèle réel, effacement ou agent délégué |

Le contrôle final inclut les refus pour confirmation absente, faits périmés,
précontrôle non vérifié, budget/tentatives épuisés et session incorrecte de M14.
Les nouveaux tests vérifient source manquante, autre travail, empreinte changée,
cycle, contexte source absent et référence dupliquée. Ils comparent l'état avant
et après refus et le nombre d'invocations du fournisseur scripté.

Une intention historique pré-M15 est construite seulement dans une fixture
isolée, avec un dernier manifeste vide et une source ayant reçu la version.
Ce n'est pas une nouvelle opération publique ni une modification d'une base
utilisateur. Le test vérifie catalogue, retrait et refus au départ sans réécrire
ce contexte historique. Un autre test modifie un ancêtre pendant la sonde :
la transaction refuse la reprise sans créer de tentative ou de réservation.

Le test d'export optionnel `TestProjectMemoryRecipeBundle` est ignoré dans le
lot final faute de demande d'export ; il n'est pas compté parmi les 33 réussites.
La recette navigateur réutilise un export de fixture M3, empreinté dans ses
métadonnées, et revérifie ses preuves par le CLI avant une nouvelle admission.

La recette finale vérifie les quatre vues FR/EN et thèmes pour chacun des deux
refus, à 1440 et 320 px, puis Échap et le retour du focus. Le diagnostic initial
exerce aussi 760 et 390 px. Le formulaire de reprise est absent lorsque la mémoire
est incompatible ; des demandes construites directement échouent néanmoins
avec `preview_stale`, code CLI 3 et HTTP 409. Le refus est contrôlé par le moteur.
Les deux conflits attendus sont distingués des erreurs inattendues ; aucune
erreur de page ou requête échouée inattendue n'est observée. Deux lectures annulées
pendant une navigation de la fixture sont conservées séparément.

Après sélection rétablie, un vrai clic dans le formulaire anglais reprend la
même session. La conversation rapporte 20 jetons d'entrée simulés ; la nouvelle
exécution en rapporte 10. Le retrait suivant conserve les deux versions de la
leçon et les prompts des deux agents, puis empêche toute nouvelle invocation.
La recette produit 24 captures ; les vues finales FR claire à 320 px et EN sombre
à 1440 px sont inspectées. Aucun débordement horizontal de la modale n'est
observé. Le contraste du diagnostic initial reste mesuré à au moins 4,5:1 ; les
nouveaux états de refus ne font pas l'objet d'une mesure séparée de contraste.

Les entrées du candidat sont identiques pour les 33 tests, les quatre contrôles
de concurrence, le binaire, l'analyse statique et la recette finale. Le script
navigateur et l'archive de fixture sont liés séparément à leurs empreintes.
`audit-final.json` vérifie cette attribution, les originaux des échecs, l'ajout
sans réécriture des évaluations et le serveur conservé. `candidate.diff` compare
uniquement les fichiers du lot à leurs préimages, sans absorber le travail antérieur.

## Documentation et RETEX

[PROJECT-MEMORY FR](../../PROJECT-MEMORY.md) et
[PROJECT-MEMORY EN](../../en/PROJECT-MEMORY.md) décrivent filiation, usages actifs,
sélection courante et nouvelle conversation, avec le même diagramme de flux.
La formulation sur l'ancienne projection de contexte est précisée pour ne pas
nier la restauration explicite introduite en M14. Les deux messages nouveaux
suivent le catalogue de traductions et son générateur existants.

La leçon réutilisable est appliquée dans le lecteur commun et ses consommateurs :
la sélection actuelle ne décrit pas tout le contexte conservé dans une session.
Il faut suivre les références réellement reçues et contrôler les deux états.
L'inventaire et la garde de départ utilisent le même lecteur ; celui-ci accepte
aussi la transaction de reprise, sans réentrer dans le pool SQLite. Les tests
gardent le contre-exemple et les entrées invalides pour les usages futurs.

M14 vérifiait la restauration et les gardes avec des fournisseurs scriptés,
sans le cycle complet d'une leçon de projet sélectionnée puis désélectionnée.
Ce manque de couverture n'invalide pas ses observations, mais ne démontrait pas
ce nouveau contrat. M15 combine ces deux parcours au lieu de déduire l'oubli d'un
manifeste vide. La protection est reçue par les consommateurs CLI/API et la garde
de départ via le code partagé ; une simple consigne documentaire ne la prouverait pas.

Deux éditions documentaires sont refusées : un bloc vide, puis une ancre mal
recopiée. Elles n'ont modifié aucun fichier. Les ancres FR/EN réelles ont ensuite
été reprises dans des patches ciblés. Quelques recherches ont supposé un
ancien nom de fichier absent ; aucun résultat d'exécution n'en est déduit.
La première recette navigateur attend à tort un code CLI 2 : `preview_stale`
renvoie 3, et son équivalent HTTP renvoie 409. Les producteurs existants sont
relus, puis la recette contrôle ces valeurs exactes. Le moteur n'est pas modifié
pour satisfaire cette attente erronée. L'exécution suivante atteint le retrait,
puis perd l'accordéon pendant sa reconstruction ; les quatre ouvertures de cet
accordéon utilisent désormais un localisateur qui retrouve l'élément au clic.
Les mutations ne sont pas relancées automatiquement. Les deux échecs, captures,
métadonnées et scripts exacts restent dans `browser*` et `preimages/`.
La revue du diff reste locale au contexte de l'auteur, sans acceptation indépendante.

MEM-C18 reste **PARTIAL** : le retrait et la désélection sont vérifiés dans ce
périmètre ; ils ne prouvent pas une purge des archives, anciens checkpoints,
copies libres ou données retenues par un fournisseur réel. Le lecteur suit les
références versionnées attribuées dans `context.memory`,
pas les connaissances acquises par une lecture d'outil non attribuée ou une
copie de texte libre. MEM-C15/MEM-C16
gardent leurs limites de qualification réelle. Résumé progressif et projection
structurée des messages restent ouverts. Les huit évaluations M6/M7 sont
inchangées ; aucun nouvel appel réel ni déploiement n'est effectué.
