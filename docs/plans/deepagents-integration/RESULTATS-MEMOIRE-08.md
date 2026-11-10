# Le diagnostic de reprise consulte la frontière de conversation

16 octobre 2026, après « next ». M12.1 à M12.4 de
[l’incrément 8](INCREMENT-MEMOIRE-08.md) sont réalisés et vérifiés localement.
Codex seul, sans appel de modèle réel, déploiement ou migration. Le serveur
utilisateur observé à la reprise est conservé.

## Constat, réalisation et consommateurs

Le premier test échoue : `mission recovery` ne consulte pas l’état M11, alors
qu’une fin de dialogue est conservée. `recoveryPreviewFor` décrit une nouvelle
tentative sans qualifier la frontière de conversation. Le journal original et
la source exacte sont dans `evidence/m12-20261016/recovery-red.json/.log` et
`preimages/`. Ce manque de diagnostic est distinct de la persistance M11.

Le nouveau producteur commun `dialogue_recovery.go` distingue contrôles réussis,
refus et inconnus : dernière tentative, fin du processus, frontière confirmée,
résultats d’outils, gardes historiques, commande/modèle/environnement autorisé,
compatibilité des plafonds, références mémoire et attente fournisseur. Il réutilise
`resolveModel`, `configuredRunLimitsWith`, `projectMemoryGuard`,
`providerCooldownGuard` et le lecteur validé de checkpoint M11.

Une absence historique reste inconnue ; un état non vide altéré est refusé.
Six traitements sont distingués : examiner la dernière tentative, attendre la
fin du processus, examiner l’histoire absente, qualifier les effets incertains,
réexaminer le contrat courant ou examiner une frontière confirmée. Les contrôles
de session/accès/réservation et de décision restent inconnus. Aucun fournisseur
ou précontrôle n’est exécuté. `decision_recorded` et `resume_supported` restent
`false`, même sur la frontière confirmée. `facts_sha256` attribue les faits
exposés, sans l’heure de consultation ; aucun endpoint ne le traite comme une
approbation.

La sélection lit l’identité exacte ou la dernière ligne de la tâche en stockage.
Elle ne dépend plus des 200 agents récents de l’affichage. Le test place 201
autres lignes après la tentative, vérifie son absence des 200 lignes affichées,
puis sa sélection correcte avec et sans identifiant. Une tentative ultérieure
empêche ensuite de présenter l’ancienne comme courante.

La CLI JSON/texte, l’API authentifiée et **Avant une relance → Reprise de
conversation** reçoivent le même diagnostic. Les blocages/inconnus restent
visibles ; les contrôles réussis se déplient dans un détail natif. Le catalogue
commun FR/EN est régénéré par son outil. Aucun style ou bouton de restauration
n’est ajouté. Un texte hostile reste du texte.

Les tables des travaux, agents, tours, réservations, événements et journaux
sont identiques avant/après les lectures et refus. Les réponses CLI/API ont
les mêmes contrôles et empreinte. Le fournisseur local n’est pas rappelé.
Une session absente est refusée avec 403 selon le contrat existant ; une autre
tâche ou un autre travail ne permet pas de consulter cette tentative.

## Preuves et limites

| Contrôle | Résultat | Preuve sous `evidence/m12-20261016/` |
| --- | --- | --- |
| Aperçu ignorant la frontière avant réalisation | FAIL exécuté | `recovery-red.json/.log` |
| Qualification, sélection, CLI/API sans mutation, consommateurs M11 | PASS, 11 tests nommés et 17 sous-tests | `recovery-final-ui.json/.log` |
| Concurrence et wrappers locaux | PASS, 5 tests nommés et 8 sous-tests avec `-race` | `recovery-race.json/.log` |
| Outil manquant, fin absente, état ancien/altéré, processus actif, fournisseur changé, plafond resserré, mémoire inaccessible | Huit variantes exécutées | Même lot moteur final |
| Analyse statique et compilation | PASS | `engine-vet.json/.log`, `build-final.json/.log`, `built-binary-final.json` |
| Catalogue et rendu textuel FR/EN | PASS | Tests Node et contrôles finaux dans `final-audit.json` |
| Vraie CLI, superviseur, arrêt public, serveur isolé, API et bouton | PASS | `browser-flow.json/.log`, `browser-flow/result.json` |
| FR/EN, deux thèmes, 1440/760/390/320 px, Échap/focus, détail et contraste rendu ≥ 4,5:1 | PASS | Même recette, huit captures dans `browser-flow/` |
| Décision enregistrée/appliquée et restauration fournisseur réelle | NOT IMPLEMENTED / NOT TESTED | Aucun droit déduit du diagnostic. |

Il y a **11 noms de test Go distincts** ; les cinq tests `-race` recoupent le
lot normal. Le repli des contrôles réussis a ensuite changé seulement les
ressources web et le catalogue. Les sources Go testées avec `-race` sont
identiques. Le lot normal et le binaire ont été reconstruits avec les ressources
finales, puis la recette navigateur a été vérifiée. Les manifestes conservent
cette distinction ; une revue locale ne vaut pas approbation indépendante.

La recette crée son projet par la CLI, lance un script Codex local, attend une
frontière confirmée puis arrête cet agent par l’opération publique. Les
consultations suivantes ne créent aucun appel : une invocation locale, aucun
modèle et aucune acceptation. Les captures finales anglaises et mobiles ont été
inspectées. Les états d’erreur navigateur hérités n’ont pas été rejoués ; les
refus moteur et d’authentification gardent leur propre couverture.

## RETEX et récupération des erreurs

Le premier lot de 11 tests échoue sur une assertion HTTP : le test attend 401,
le middleware existant renvoie 403. La déclaration réelle a été lue puis
l’assertion corrigée, sans modifier le middleware ni accepter plusieurs codes.
`recovery-green.json/.log` reste un FAIL malgré son nom. La source du premier
lot est conservée par empreinte ; les résultats finaux sont distincts.

La première recette compare le focus à une instance DOM susceptible d’être
remplacée à l’actualisation. Le contrat restaure une action identifiable. La
seconde attend le bouton avant le détail qui le construit à la demande. Après
lecture de `Mission.renderResults` et de `mission-results`, la recette ouvre
ce détail, clique le vrai bouton et vérifie le retour du focus vers son action.
Les deux FAIL et captures restent dans `browser/` et `browser-final/`.
La capture initiale a aussi motivé le repli des neuf contrôles réussis, en
gardant les inconnus visibles. Le résultat final ne supprime aucun contrôle.

Des recherches ont encore supposé des chemins absents ou rencontré du code
tiers compacté ; elles ont été resserrées sur les producteurs réels. Un patch
documentaire utilisait une ancre absente du plan : il a été refusé avant toute
mutation, puis appliqué après relecture du point de reprise. Aucun de ces
incidents ne constitue un test produit réussi ni une correction du moteur.

Leçon réutilisable : le diagnostic qui oriente le responsable doit consommer
le reçu mémoire, séparer preuves/refus/inconnus et ne transformer aucun avis
en autorisation. Les tests de parcours suivent les détails construits à la
demande et l’identité fonctionnelle du focus. Ces règles sont appliquées par
le code commun, ses lecteurs, leurs tests et les documents
[FR](../../PROJECT-MEMORY.md) et [EN](../../en/PROJECT-MEMORY.md).

## Prochaine frontière

La vue n’est ni un précontrôle d’exécution ni un instantané transactionnel de
tous les systèmes. Les observations peuvent évoluer. La prochaine opération
explicite doit recevoir une décision fraîche, revérifier dernier état, accès,
contrat et réservations, puis appliquer une restauration autorisée sans effacer
compteurs, coûts ou histoire. Les nouvelles tentatives gardent leur contrat.
Codex reste responsable de cette suite. MEM-C15/MEM-C16 restent PARTIAL ; résumé,
suppression et qualité LLM restent ouverts. Aucun résultat de ce candidat n’est
attribué au serveur utilisateur resté en cours.
