# M16 — projection de reprise et omissions attribuées

16 octobre 2026, reprise « next ». Codex seul ; APEX et `replan`.
MEM-U08/MEM-U13 ; prérequis déterministe de MEM-C16 et MEM-C17.

## Observation et choix

Le lecteur actuel limite les activités et le cadrage d'une reprise. La lecture
montre que le raccourcissement d'une ligne, de la prochaine action, d'un blocage
ou d'un critère ne renseigne pas systématiquement `bounded_excerpt`. La
déduplication après raccourcissement peut fusionner des activités distinctes.
Ces hypothèses doivent être reproduites avant correction.

Deux solutions bornées : corriger le seul booléen, ou rendre aussi attribuables
les lignes retenues et les raisons des omissions. La seconde est retenue : les
séquences de journaux et les compteurs expliquent le contenu reçu sans joindre
les résultats bruts. Les anciens champs et les bornes de représentation restent
compatibles. Aucune nouvelle politique, migration ou consommation modèle.

Le code Deep Agents à SHA `29d646fd2d832a44c476964333eddecb0ec1c0cd`
sépare messages conservés, résumé et chemin d'historique. Son échec d'offload
n'empêche pas le résumé ; cela n'établit pas une conservation garantie.
Swarm garde ici une projection en lecture seule des enregistrements disponibles.
Il ne produit pas de synthèse LLM et ne promet pas un historique sans rétention.

| Lot | Propriétaire, dépendance | Critère observable |
| --- | --- | --- |
| M16.1 | Codex ; M15 conservé | Contre-exemples avant correction : texte raccourci non signalé, fusion de lignes distinctes, omission de critères et cadrage introuvable. |
| M16.2 | Codex ; M16.1 | Source travail/tâche/agent/tentative, séquences des activités retenues, comptes des exclusions ; chaque borne est signalée, masquage des secrets conservé, refus d'une tâche introuvable. |
| M16.3 | Codex ; M16.2 | Fournisseur scripté recevant le texte exact ; persistance et lecture par CLI/API compilées, références vers les journaux inchangés, refus de capacité/écriture et absence d'acceptation implicite. |
| M16.4 | Codex ; preuves précédentes | Vérifications ciblées, revue locale, docs FR/EN, évaluation ajoutée sans renotation, RETEX avec limites. |

Les fixtures sont isolées. Le serveur utilisateur et les huit évaluations
réelles précédentes restent intacts. Les séquences d'outils fournisseur,
la qualité d'un résumé automatique, la purge et une revue indépendante ne sont
pas qualifiées par ce lot. Pas de modification visuelle : la recette porte sur
le contenu commun déjà exposé dans les reçus CLI/API.

## Résultat local

M16.1 à M16.4 sont réalisés et vérifiés : cinq contre-exemples conservés,
18 tests Go distincts et sept contrôles de la vraie CLI/supervision/API avec
deux invocations locales scriptées. Les sources et les consommations sont
conservées ; les erreurs de compilation et de collecte de preuve ont été
diagnostiquées et corrigées avant la recette réussie. Documentation FR/EN,
revue locale et RETEX dans [RESULTATS-MEMOIRE-12.md](RESULTATS-MEMOIRE-12.md).
Résumé automatique, séquences de messages fournisseur et qualité réelle restent
ouverts ; le serveur utilisateur reste inchangé.
