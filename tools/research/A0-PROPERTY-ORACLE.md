# Oracle A0 — propriétés sur preuves conservées

L’outil `a0_property_oracle.py` évalue chaque rangée historique du corpus A0, sans rejouer le récepteur, ouvrir SQLite ou appeler un fournisseur. Les statuts historiques ne sont jamais utilisés comme verdicts.

```sh
python3 tools/research/a0_property_oracle.py --index docs/recherche/reunion-limos-evalia-20261117/A0-PREUVES.json --output /chemin/nouveau-dossier
python3 tests/test_a0_property_oracle.py -v
```

## Entrées, intégrité et sorties

L’index fournit `location`, `inventory_sha256` et `journey`. Le format conservé utilise le dictionnaire chemin relatif → SHA-256 de `A0-INVENTORY.json`. Les variantes explicites `inventory_path`, `inventory`, `journey_path` et `result_path` sont également validées. Aucun chemin traversant ou composant symbolique n’est accepté. Tous les fichiers sont hachés avant et après évaluation ; les empreintes effectivement consommées sont comparées au manifeste. Cela détecte les divergences observées, sans garantir un snapshot atomique face à un écrivain hostile concurrent.

Les requêtes numérotées, résultats, métadonnées de processus et observations avant/après produisent les événements. Le pilote archivé indique que `initial.observation.json` suit l’initialisation ; une opération `present` réussie est en lecture seule. Ces contrats sont des hypothèses explicites de l’adaptation. Le pilote archivé a été rapproché octet pour octet du source lu. Aucun statut `committed` seul ne crée un événement : un effet doit apparaître dans les captures persistées.

Le dossier de sortie doit être inexistant. Il est réservé exclusivement ; les fichiers complets sont synchronisés et publiés par lien sans remplacement. `COMPLETE.json` lie uniquement les empreintes des artefacts publiés ; sa présence seule ne prouve jamais que le processus s’est terminé avec succès. Vérifier une exécution exige aussi une preuve conservée d’une sortie de processus réussie (reçu terminal), en plus des empreintes. Un échec d’écriture conserve la cause originale (OSError) et les fichiers partiels ; si l’échec survient après la publication du marqueur (fsync final du dossier), le marqueur peut exister et la publication est rapportée incertaine, sans suppression ni retour arrière simulé. Une sortie existante est préservée. Code 2 : refus global ; code 0 : rapport produit, y compris lorsqu’il contient des verdicts `fail`, `error` ou `inconc`. Ces trois états restent visibles.

## Portée des verdicts

- PT compare les termes complets de l’action et de l’approbation persistée.
- PA vérifie la projection d’autorisation disponible, l’usage antérieur et l’historique de retrait ; PR vérifie l’absence de retrait effectif antérieur. Les canaux logiques observés sont supposés activés. Aucune identité OS/réseau n’est établie.
- PV1 vérifie l’existence d’un effet antérieur correspondant à l’identifiant, sans ajouter une égalité de digest absente de sa formule. PT porte cette égalité.
- PV1/PV2 sont présentées selon deux projections : `intention` (`intent_id` dans le cas) et `attempt` (commande numérotée). Les atomes non projetés restent `inconc`, car le sens de i dans l’article demeure à préciser.
- Une réponse `unknown` ne déclenche ni PV1 ni PV2 : la vacuité est annotée et ne prouve aucune qualité de récupération.
- PL reste `inconc` lorsqu’un retrait apparaît : Delta et les temps requête/acquittement ne sont pas établis. Sans retrait, le passage est seulement vacuitaire. L’expiration réelle n’est pas modélisée.
- Les trous de trace limitent les verdicts ; aucun ordre n’est inventé entre plusieurs effets d’une même capture. Deux effets d’un même droit prouvent toutefois au moins un double usage.
- Toutes les captures d’un cas doivent se rattacher à une même exécution (`case_run`) ; une divergence entre captures est une erreur de preuve et aucun diff d’effets n’est dérivé entre exécutions.
- Une ligne d’effet qui disparaît ou qui est réécrite à identifiant égal entre deux captures est une incohérence de persistance : les propriétés concernées deviennent `error`/`inconc`, jamais un simple pass annoté ; un échec antérieur à témoin solide est conservé.
- PT ne conclut à l’échec (absence d’approbation) que si le préfixe antérieur à l’engagement est complètement observé ; sinon `inconc`. Un témoin d’approbation exact trouvé reste enregistré même si un trou sans rapport apparaît plus tard.

L’agrégation conserve le témoin le plus fort : `fail`, puis `error`, `inconc`, `pass`. Une vacuité n’est pas une propriété exercée. Le rapport conserve les traces, témoins, notes et empreintes ; le tableau garde une ligne par rangée historique, y compris une assertion sans trace attribuable.

## Première application réelle et limites

La première exécution privée est conservée dans `/run/media/fpizzi/SWARM/swarm-evidence/article-v6-oracle-kjevtgz4/evaluation-r1`. Elle porte sur 83 rangées et 2324 fichiers inventoriés ; ce n’est pas un rejeu de 83 expériences. Elle retrouve PT en échec pour `E1-W_sensitivity`, et PV2_intention en échec pour `E1-V06` (effet puis refus pour la même intention). PV2_attempt n’échoue pas dans V06. Au stade historique `evaluation-r1`, avant les reprises correctives décrites ci-dessous, l’effet des nouvelles règles sur le corpus réel n’était pas encore établi et la table restait à reconstituer. Cette limite appartient à r1 ; les réévaluations r3 puis r4 conservées plus bas établissent depuis une nouvelle table de 83 lignes, sans changement de verdict observé, dans leur portée documentée.

Les captures clairsemées de `E1-V05_revoke`, `E1-V05_policy` et les noms de captures non numérotés de `E1-V07` ne sont pas tous résolus par l’adaptateur. Les verdicts affectés restent inconclusifs. `E1-Q_evidence_refusals` n’a pas de trace propre attribuée. Aucune de ces limites n’est remplacée par le PASS historique. Le rapport réel, les contrôles synthétiques, la revue indépendante et l’acceptation scientifique restent des preuves distinctes. Aucune conclusion universelle sur Swarm, H2, l’expiration ou PV2 n’en découle.

## Reprises correctives et portée des preuves

Un premier candidat de correction a passé 32 tests et produit les 83 mêmes
verdicts dans `article-v6-oracle-repair-bpscx4x9/evaluation-r3`. La revue suivante
a clos PT, publication incertaine et index ; deux variantes restaient ouvertes :
la capture terminale ignorée en présence d’un after explicite, et un ensemble
cohérent de captures d’un autre run que celui demandé à l’initialisation.
Ces variantes ont été reproduites après témoins valides puis corrigées.

Désormais, chaque capture est liée au case_run de la requête d’initialisation
réussie et corroborée. Les captures initiales doivent respecter la configuration
initialisée et l’absence d’effets du magasin fraîchement créé par le récepteur.
La capture terminale est contrôlée même si l’après du dernier pas est explicite ;
un effet supplémentaire non attribuable produit un trou, jamais un engagement
fabriqué. Un changement/disparition d’effet produit une erreur de preuve.

Treize tests ciblés passent sur cette correction, dont les deux nouveaux gardes
et leurs consommateurs de préfixes, snapshots, PT/PV2, unknown et erreurs de forme.
Les 32 PASS précédents restent liés à leurs anciens octets. Le manifeste permanent
prévoit maintenant les 34 tests maintenus ; cette version complète du manifeste
n’est pas encore réexécutée. Revue corrective et acceptation restent ouvertes.
Le code de sortie terminal et les hashes du marqueur restent tous deux nécessaires.

L’application corrective r4 est conservée dans
`article-v6-oracle-repair-bpscx4x9/round2/evaluation-r4`. Elle évalue les 83 rangées,
avec zéro verdict error et aucun changement de verdict par rapport à r3.
Code de sortie 0 et hashes du marqueur ont été relus ensemble. Cela établit
l’exécution limitée du candidat, pas sa revue indépendante ni une conformité globale.
