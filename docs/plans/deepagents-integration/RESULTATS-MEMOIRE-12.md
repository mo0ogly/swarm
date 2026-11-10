# M16 — pertes de contexte signalées et sources retrouvables

16 octobre 2026, après « next » puis « replan et corrige ». Les quatre lots de
[l'incrément 12](INCREMENT-MEMOIRE-12.md) sont réalisés et vérifiés localement.
Codex seul, aucun modèle réel, aucune modification du serveur utilisateur.
Ce lot corrige la projection déterministe déjà présente ; il ne réalise pas
la synthèse progressive par LLM prévue dans MEM-C17.

## Défauts et correction commune

Les cinq tests initiaux échouent effectivement avant correction : une activité
raccourcie garde un marqueur faux ; quatre champs du cadrage ont le même défaut ;
deux activités distinctes deviennent un doublon après raccourcissement ; une
omission de critères n'atteint pas le marqueur global ; une tâche disparue donne
une projection réussie sans cadrage. Les originaux sont dans
`evidence/m16-20261016/projection-red.log/.json` et leur source est conservée.
Le cas à treize critères est une fixture historique/invalide : les modifications
publiques actuelles de tâche en acceptent au plus douze.

La correction passe par `workerRecoveryContext`, `recoveryTaskFocus` et un petit
module de projection commun. Le texte complet **après masquage** sert à dédupliquer,
avant la réduction de largeur. Les activités retenues ont une référence `seq`
et un indicateur de raccourcissement, dans le même ordre que leurs textes.
Les comptes expliquent chaque ligne conservée : examinée ou hors fenêtre, puis
retenue, annonce de résultat exclue, doublon ou omission faute de place.
Leur somme est vérifiée, y compris pour un extrait vide.

Le cadrage nomme ses champs raccourcis et compte ses critères omis ; sa perte
de représentation remonte au marqueur global. Travail, tâche, agent, tentative
et révision du cadrage restent distingués. Une tâche introuvable est refusée.
Les critères gardent leur obligation de vérification ; ni les anciens verdicts
ni un processus terminé ne créent une acceptation.

Les bornes existantes sont conservées, sans nouvelle politique ou migration.
`omitted_budget_rows` désigne uniquement la place de l'extrait. Le masquage et
les annonces de résultats filtrées sont des exclusions intentionnelles ; un
marqueur de borne faux ne signifie pas « historique complet ». Les références
visent les journaux encore disponibles, eux-mêmes soumis à leur rétention et à
leur borne d'enregistrement existantes. La projection ne les réécrit pas.

## Vérification du parcours

| Critère | Résultat et preuve sous `evidence/m16-20261016/` |
| --- | --- |
| Défauts avant correction | Cinq tests FAIL, dont quatre variantes de champ : `projection-red.json/.log`, préimage du test. |
| Correction et dépendances de réception | 18 tests Go distincts PASS : `projection-contract.json/.log`. Sept nouveaux tests, dont dix sous-tests nommés de champs et de scénarios. Les cinq premiers verts sont un sous-ensemble, pas cinq tests supplémentaires. |
| Bornes, vide, Unicode, filtrage, source, scope et historique | Même lot : 24 lignes, 4 500 octets, fenêtre 160, critères omis, ordre des sources, doublons après masquage, refus hors tâche et original des journaux inchangé. |
| Refus de capacité ou d'écriture avant fournisseur | Tests M9 réexécutés dans ce lot ; reçu final/prefixe préparé, ancienne tentative et consommations conservés après réouverture. |
| Vraies CLI, supervision et API compilées | Sept contrôles PASS : `native-cli/result.json`, commandes et reçus voisins. Deux invocations de scripts, zéro appel de modèle. |
| Analyse statique, configuration et compilation exécutable | `static.json`, `config.json`, `build-cli.json` puis `build-final.json` couvrant aussi le vrai point d'entrée et la recette. |
| Diff, liens, empreintes courantes et serveur conservé | `audit-final.json` et `candidate.diff`. Branche et HEAD initiaux conservés ; PID 2186583, même URL et état prêt. |

Dans le parcours public, le premier script échoue après trois appels et trois
résultats observés. La nouvelle tentative reçoit trois opérations : les deux
lectures longues distinctes, bien que leurs extraits se ressemblent, et une
commande masquée. Sur six lignes d'activité conservées, trois annonces de
résultat sont exclues et trois opérations attribuées sont transmises.
Le critère courant raccourci est signalé par `criteria.1.expected`.

L'entrée réellement enregistrée par le script est identique au prompt relu par
CLI puis par le serveur API ouvert séparément. Son SHA-256 et sa taille sont
exacts ; le manifeste préparé correspond au préfixe conservé. Les séquences
renvoient aux messages d'origine distincts. Le dossier contient les prompts,
les reçus CLI/API et les sources avant/après strictement identiques.
Une demande API pour un autre travail reçoit HTTP 403 sans divulgation du prompt.
Les deux tentatives restent présentes et la tâche n'est pas acceptée.

La [documentation FR](../../PROJECT-MEMORY.md) et la
[documentation EN](../../en/PROJECT-MEMORY.md) décrivent les mêmes champs,
bornes, pagination des journaux, refus et recette reproductible :

```sh
go build -o /tmp/swarm-memory-candidate ./cmd/swarm
python3 tests/worker_recovery_projection.py /tmp/swarm-memory-candidate /tmp/swarm-memory-new-proof
```

La sortie doit être nouvelle. La recette initialise sa propre racine temporaire,
utilise un fournisseur scripté et arrête uniquement son propre serveur.
Pas de modification visuelle : aucune recette navigateur n'est revendiquée.
Pas de nouvelle synchronisation ou d'état partagé modifié : les tests de
concurrence antérieurs gardent leur périmètre, aucun nouveau PASS `-race` n'est
attribué à M16. Les sources courantes du lot et de ses dépendances sont liées
aux reçus, avec contrôle final de leur absence de dérive.

## Replan, revue locale et RETEX

La première commande de compilation visait `.` : la racine est le paquet de
ressources embarquées et produit une archive Go, pas un exécutable. Son code zéro
ne prouvait donc aucun parcours natif. La recette échoue avant `init`, sans appel
de fournisseur. L'archive, le journal et la source de recette sont conservés ;
le bon point d'entrée `cmd/swarm/main.go` est inspecté puis compilé.
La recette exige désormais un exécutable et vérifie sa version avant tout état
de fixture. La nouvelle exécution passe ses sept contrôles.

Le collecteur de preuves hérité omettait ce point d'entrée et le nouveau script.
Sa version initiale est conservée ; les reçus suivants ajoutent le point d'entrée,
les ressources, modules, script et collecteur. Le contrôle final relie le même
binaire et la même recette aux résultats natifs, et conserve l'échec initial.
Les recherches initiales avec noms supposés et sorties trop larges sont aussi
consignées dans `method-observations.json` ; découverte des chemins réels et
lectures bornées appliquées ensuite. Ces incidents ne sont pas des défauts produit.

La revue locale couvre producteurs, transaction de lecture, consommateurs,
masquage, sérialisation, capacités et sources de preuve. Elle ne constitue pas
une revue indépendante. La leçon réutilisable est inscrite dans la documentation
opérationnelle : tout extrait indique ses pertes et ses sources disponibles ;
un indicateur faux, un reçu ou une compilation seuls ne prouvent pas la complétude.
Le script récepteur démontre que les nouvelles attributions atteignent le contexte.

L'inspiration amont est vérifiée sur le code Deep Agents fixé au SHA
`29d646fd2d832a44c476964333eddecb0ec1c0cd`, fichier
`libs/deepagents/deepagents/middleware/summarization.py`, copie et empreinte dans
`upstream.json`. Ce code sépare résumé, messages récents et chemin d'historique.
Il continue la synthèse après échec de l'offload en avertissant que les anciens
messages ne seront pas récupérables. Nous ne lui attribuons donc pas une garantie
de conservation. Aucun test de ce middleware amont n'est exécuté dans M16.

MEM-C16 reste **PARTIAL** : cette projection de métadonnées est traçable,
mais ne reconstruit pas une séquence de messages/outils fournisseur.
MEM-C17 reste **NOT TESTED** pour le résumé progressif : sa prochaine frontière
est un contrat de synthèse avec sources, exigences/incidents conservés, échec
sans état faussement complet et comparaison sur les mêmes cas. Une politique
de déclenchement éventuelle doit être persistée et éditable dans Admin/API/CLI/Go.
La qualité d'un modèle réel nécessite une autorisation et un budget distincts ;
les huit appels M6/M7 restent consommés. Aucune purge, migration réelle,
publication, acceptation indépendante ou mise à jour du serveur n'est déclarée.
