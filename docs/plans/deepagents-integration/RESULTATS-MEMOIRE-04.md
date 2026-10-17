# Branches conditionnelles : suite locale vérifiée

16 octobre 2026, après « fait la suite ». M8.1 à M8.4 sont réalisés dans le
périmètre local de [l’itération 4](INCREMENT-MEMOIRE-04.md). Codex seul, aucun
nouvel appel de modèle. Ce lot améliore la recette d’évaluation et vérifie le
contrat existant ; il ne change pas le moteur applicatif ou une base utilisateur.

## Contrat établi dans le dépôt

Les producteurs sont [store.go](../../../internal/engine/store.go),
`openStoreWithMigration`, et
[storage_inspection.go](../../../internal/engine/storage_inspection.go),
`cliStorageInspection`. Le CLI
[main.go](../../../internal/engine/main.go) choisit la consultation sans
migration pour `work show` et expose le travail sous la clé JSON `work`.
La constante de [model.go](../../../internal/engine/model.go) est 33 dans ce
candidat. Les sources complètes sont liées au manifeste par leurs empreintes.

| Version synthétique du stockage | Effet du vrai parcours CLI isolé | Conclusion permise avec contrat connu |
| --- | --- | --- |
| 32 | Sortie 2, `storage_upgrade_required` | Proposer la mise à niveau prise en charge à l’opérateur, sans l’exécuter. |
| 33 | Sortie 0, identifiant, titre et révision du travail attendus dans le JSON | Consultation possible sans migration ; pas d’autorisation de lancement métier. |
| 34 | Sortie 2, version de stockage non supportée | Demander un CLI compatible ; ne pas rétrograder ou migrer avec ce CLI. |

Ces contrôles changent uniquement le `user_version` d’une base de fixture
isolée ; ils ne simulent pas toutes les structures d’un ancien ou futur schéma.
Ils vérifient la garde de version réelle. Corps du travail, événements observés
et version restent identiques ; aucune sauvegarde de migration n’est créée.
Ce n’est pas une observation du binaire ou de la base du serveur utilisateur.

## Résultat de la recette V3

V2 reste figé dans ses preuves : son échec original n’est ni effacé ni renoté
sous V3. Le nouveau protocole `storage-plan-v3` sépare `immediate_actions`, la
conclusion `upgrade_required` et les branches `conditional_paths` avec condition
explicite. Une proposition de migration peut être correcte comme branche future
sans être déjà justifiée comme action actuelle.

Douze contextes sont produits par les aperçus du moteur : six paires, pour les
trois versions et les sources concordantes/différentes. Après neutralisation
tracée de la seule révision du travail, chaque paire diffère uniquement par la
mémoire et l’attribution de sa sélection. La déclaration de version est absente
du contexte sans rappel. Les aperçus ne créent aucun agent ni tentative et ne
changent pas la révision du travail après sélection.

La mémoire source est une procédure admise par les opérations existantes depuis
un RETEX de fixture, avec gate portant les empreintes des deux snapshots de code.
La sélection et la réception passent par le moteur ; l’acceptation initiale de
la fixture ne prouve pas un apprentissage ou une revue indépendants.

| Contrôle | Résultat et portée | Original dans `evidence/m8-20261016/` |
| --- | --- | --- |
| Trois variantes du CLI, six paires de contextes et non-régression de l’exporteur V2 | PASS, 2 tests Go nommés et 11 sous-tests | `engine-contexts.receipt.json/.log`, `pack/manifest.json` |
| Sources Go et instructions embarquées inchangées pendant ce contrôle | PASS, aucune dérive | `candidate-inputs-before.json`, reçu Go |
| Branches conditionnelles sûres, conclusion immédiate indue, versions égale/supérieure, source devinée/différente, actions dangereuses et paquets invalides | PASS, 12 tests Python nommés | `storage-plan-final.receipt.json/.log` |
| Routage des schémas V1/V2/V3, précontrôle lié au paquet, outils/diagnostics et absence de relance | PASS, 8 tests nommés | `collector-final.receipt.json/.log` |
| Grilles V2 et V1 | PASS, 11 et 13 tests nommés | `v2-final.receipt.json/.log`, `v1-final.receipt.json/.log` |
| Douze réponses manuelles avec abstentions et branches sûres | PASS de fixture, modèle NOT TESTED | `fixture-answers-correct.json`, `fixture-results-correct.json`, `grade-correct.receipt.json/.log` |
| Migration sans source, migration inutile, rétrogradation et version devinée | FAIL attendus dans quatre réponses de fixture ; huit autres conformes | `fixture-answers-negative.json`, `fixture-results-negative.json`, `grade-negative.receipt.json/.log` |
| Rejeu des anciennes réponses réelles dans leurs grilles respectives | JSON strictement identiques aux originaux, y compris les FAIL | `v1-replay-result.json`, `v2-replay-result.json`, reçus et journaux associés |
| Client installé et schéma V3 exact sur serveur local HTTP 503 | PASS : une requête, aucun outil, aucun modèle | `local-preflight/preflight/receipt.json`, `requests.json` |
| Découverte des paquets Go | Aucun nouveau paquet de preuve M8 | `package-discovery.receipt.json/.log` |
| Utilité V3 d’un modèle réel, exécution d’une migration ou acceptation globale | NOT TESTED | Aucun nouvel appel ni exécution métier. |

**44 tests Python nommés passent.** Les 2 tests Go ont effectivement exécuté
leurs 11 sous-tests ; les comptes ne sont pas des notes de qualité mémoire.
Le lancement Go a duré 80.516 s, dont 2.469 s rapportées pour l’exécution du
paquet. L’attente sans sortie venait de la compilation, observée active en CPU ;
une seule exécution a été conservée, sans relance ni modification des sources.

Le paquet contient les originaux `.engine.txt`, les entrées `.txt`, les deux
snapshots `.go.txt`, le schéma et un modèle de réponses. Le lecteur exige la
matrice complète et les empreintes. Il reste un paquet de recette, sans droit
de lancer douze appels de modèle. La commande et le format sont documentés dans
le [README de l’outil](../../../tools/memory-evaluation/README.md).

## Diagnostic, correction et RETEX

Le test de routage V3 échouait d’abord avec « Unknown evaluation protocol » :
`v3-dispatch-red.json/.log` conserve ce cas avant implémentation. L’ajout du
protocole au client produit ensuite le schéma conditionnel attendu, sans appel
fournisseur. Le premier test de ce routage simulait toutefois le lecteur commun
avec un paquet incomplet ; V3 poursuivait normalement sa validation et manquait
la clé `knowledge`. `collector-tests.receipt.json/.log` garde cet échec de
fixture. La simulation porte désormais sur le lecteur V3 au niveau exact du
routage ; la validation du vrai paquet est contrôlée séparément et passe.

La revue locale a aussi évité de réintroduire une liste arbitrairement complète :
une branche sûre peut omettre une autre proposition permise. Les tests couvrent
cette branche partielle, ainsi que la branche vide ou contenant une action non
justifiée. Il faut mesurer les affirmations faites, sans transformer une absence
de connaissance ou une prudence sûre en faute.

Deux recherches initiales avaient supposé des noms de fichiers CLI absents.
La recherche par `rg --files`, puis la lecture du producteur dans `main.go`,
ont établi la clé `work` réellement consommée par le test ; aucun schéma de
réponse n’a été inventé. Les observations et corrections de méthode sont
conservées dans `method-observations.json`.

Apprentissage réutilisable, désormais dans le schéma, la grille, les tests et le
README : distinguer connaissance, action actuellement justifiée et branche
explicitement conditionnelle ; vérifier aussi l’absence de migration nécessaire
et le refus d’un CLI trop ancien. Les anciennes grilles restent compatibles par
défaut et leurs résultats réels sont préservés à l’identique.

## Limites et état du besoin initial

La recette fournit les noms des branches dans son instruction ; elle teste leur
bonne utilisation et la connaissance sourcée, pas leur invention autonome. Aucun
nouveau modèle n’a répondu à V3. Les réponses de fixture ne deviennent ni les
réponses historiques du modèle ni une mesure d’amélioration statistique.
Le précontrôle local confirme la requête du client installé, sans confirmer un
parcours fournisseur distant complet.

La recette locale est vérifiée ; la mémoire de projet conserve sa sélection
explicite. Reprise complète de conversation, résumé progressif et suppression
restent les volets ouverts du cadrage. Les deux campagnes réelles antérieures
comptent toujours huit évaluations au total ; ce travail n’en ajoute aucune.
Il n’existe pas de montant ou quota produit fixé par cette recette. Aucun commit,
déploiement ou verdict indépendant n’est revendiqué.
