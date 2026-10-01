# Recette des contrats du moteur

Exécuter depuis le dépôt Swarm. Les six commandes utilisent des Stores et dépôts
jetables ; elles ne modifient pas la mission ouverte dans le cockpit.

| Cas | Commande | Ce qui est contrôlé |
| --- | --- | --- |
| E1 | `node tests/engine_acceptance.cjs --case launch` | Lancement et acceptation sans contournement du vérificateur, fraîcheur des preuves |
| E2 | `node tests/engine_acceptance.cjs --case ownership` | Propriété des exigences, délégation, isolation et transmission au responsable |
| E3 | `node tests/engine_acceptance.cjs --case revision` | Même révision pour contrôles et revue, refus des preuves modifiées ou périmées |
| E4 | `node tests/engine_acceptance.cjs --case recovery` | Reprise après interruption, compteurs conservés, verdict réutilisé, exclusion concurrente |
| E5 | `node tests/engine_acceptance.cjs --case truth` | États et preuves CLI/web ; navigateur réel en français/anglais, clair/sombre, bureau/mobile |
| E6 | `node tests/engine_acceptance.cjs --case real` | Parcours déterministe défaut/refus/correction/acceptation et fermeture des périmètres |

Le nom historique `real` ne signifie **pas** que le test appelle un modèle réel.
Les décisions et les processus agents sont simulés. Ces commandes ne consomment
aucun appel de modèle et ne démontrent pas l’autonomie de bout en bout.

Prérequis : Go, Node.js, les dépendances du dépôt et Chrome/Puppeteer pour E5.
Le lanceur refuse une sélection sans test exécuté, un test ignoré, un échec ou un
résultat incomplet. `go test ./...` seul ignore la recette navigateur E5 ; il ne
remplace donc pas ces six commandes.

Enregistrer la révision du dépôt, le diff éventuel, chaque commande et son résultat.
Pour un essai avec de vrais agents, figer aussi l’empreinte du moteur et le schéma
Stockage. Garder le même moteur pendant l’essai, enregistrer les appels par rôle
et toute intervention externe. Une réparation manuelle reste une intervention ;
aucun succès déterministe ne permet d’accepter automatiquement une mission réelle.

Préparation distincte : [essai avec défaut métier contrôlé](CONTROLLED-AUTONOMY-TRIAL.md).


## Preuves locales et validation indépendante

Le producteur renseigne `docs/<task>.delivery.json` avec ses propres observations : commande exécutée, résultat et fichiers de preuve suivis. `pass` signifie que ses vérifications locales couvrent le critère ; `complete` signifie que la livraison locale est complète. Ces valeurs ne déclarent jamais la tâche acceptée. Les commandes configurées sont fournies dans le prompt avec leurs options, sans permission supplémentaire de modifier leurs définitions. Une vérification impossible ou non exécutée reste `not_tested`.

Le moteur vérifie l’attribution et la complétude du bilan, exécute ensuite ses contrôles sur la révision candidate, puis demande une revue indépendante. Le producteur et le planificateur ne doivent pas attendre ces étapes futures pour déclarer les observations locales réellement obtenues. Un bilan `pass` ne contourne ni un contrôle moteur échoué ni un avis de revue défavorable.

L’essai réel du 23 septembre 2026 a révélé cette confusion : deux livraisons `not_tested` ont bloqué le parcours avant les contrôles. Les instructions et diagnostics ont été clarifiés ; cela ne prouve pas encore la réussite d’un nouvel essai avec un modèle réel.

## Contrôles comme preuves d’une revue humaine

Une politique `human` peut désormais contenir des `controls` préautorisés.
Le moteur les exécute sous l’autorisation de mission active et transmet son reçu
au vérificateur indépendant. Un succès conserve la tâche « À vérifier » : il ne
vaut ni décision humaine ni libération des tâches suivantes. Les critères
qualitatifs peuvent rester sans contrôle automatisé.

Dans « Configurer les validations », conserver « Revue humaine », ajouter les
contrôles, sélectionner leurs critères et renseigner les fichiers examinés.
Les chemins `inputs` sont relatifs à la racine du projet, même si `dir` diffère.
Le moteur empreinte ces fichiers et le rapport avant les contrôles, puis refuse
les preuves si leur contenu change. Sans `inputs`, seul le rapport est lié au reçu :
cela ne prouve pas que tous les fichiers source sont restés identiques.

Le CLI utilise le même contrat : `swarm --json validation preview WORK --task TASK
--input politique.json`, puis `validation apply` avec le `preview_token` retourné,
la même `expected_revision` et un nouvel `event_id`. Exemple de champ `policy` :

```json
{"mode":"human","controls":[{"id":"tests","command":["go","test","./..."],"dir":".","timeout_seconds":300,"criteria":[1],"justification":"La suite vérifie le comportement du premier critère.","inputs":["go.mod","engine.go"]}]}
```

Adapter les chemins et critères réels avant confirmation. Une modification de
politique archive l’ancien avis et invalide les reçus ; elle est refusée pendant
une revue active. Un reçu courant est réutilisé sans relancer les mêmes tests à
chaque passage du conducteur. Les fichiers non déclarés ne sont pas surveillés.

Recette ciblée de ce contrat :

```sh
go test -run 'TestHumanControl|TestHumanPolicy' -count=1
go test -race -run 'TestHumanControl|TestHumanPolicy' -count=1
go build -o /tmp/swarm-human-evidence .
node tests/human_evidence_ui.cjs /tmp/swarm-human-evidence /tmp/swarm-human-evidence-ui
```

## Captures dans la revue indépendante

Les images PNG/JPEG déclarées par l’opérateur dans les entrées des contrôles
sont jointes au vérificateur Claude lorsque leurs empreintes correspondent au
reçu courant. Elles doivent appartenir à `docs/screenshots/` ; les liens du
rapport ne déclenchent aucune lecture supplémentaire. Limites : 20 images,
5 Mio par image, 10 Mio cumulés, 20 millions de pixels par image. Les fichiers
inaccessibles, invalides, trop volumineux ou périmés bloquent la revue avant
l’appel. Un adaptateur sans prise en charge visuelle refuse explicitement la
revue, sans omettre silencieusement les images.

Le vérificateur conserve sa session distincte, aucun outil de modification et
ses budgets/délais habituels. La revue des pixels reste un jugement IA ; elle
n’est ni un contrôle déterministe, ni une acceptation. Une image modifiée
invalide les preuves liées par empreinte. Le transport utilise les blocs
image du [format streaming Claude](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode).
