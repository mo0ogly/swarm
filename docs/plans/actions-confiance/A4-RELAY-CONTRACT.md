# A4 — Candidat privé de banc de référence

Candidat privé : les résultats sont dans les reçus liés aux empreintes exactes,
pas dans une étiquette du source. Aucun fichier du dépôt consommateur n'est modifié
par cette remise. Quatre chemins nouveaux sont proposés : le driver, ce contrat,
sa version `docs/en/plans/actions-confiance/A4-RELAY-CONTRACT.md` et la recette.
L'intégration partagée et sa réservation restent à attribuer.

## Portée et données

Le driver Python autonome ne lance ni n'importe Swarm. Il ne prouve que son
modèle fictif et son oracle : deux acteurs A/B, une autorisation enregistrée par
l'opérateur du banc, deux relais en mémoire R1 puis R2, un point d'application et
des observations d'effets. Les états known/engaged/unknown sont déclarés dans le
scénario. Aucun effet distant, processus, confinement OS, autorité cryptographique,
supervision IA ou intégration native n'est éprouvé. Un résultat vert futur sur ce
banc ne clôturera pas la qualification native A4.

L'original est immuable et enregistré séparément des reçus. R1 reçoit cet ordre ;
R2 reçoit réellement le reçu R1, avec son contenu et son prédécesseur. La garde du
banc vérifie l'original, l'ordre des deux relais, leur lien et leur enregistrement.
Cette identité d'objets en mémoire n'est pas une garantie de transport distribuée.
Remise/réception n'appliquent rien. Seule une application validée change l'état.
Une reprise nécessite un autre identifiant, une nouvelle décision enregistrée et
le lien vers le dernier arrêt applicable ; le même opérateur peut prendre cette
nouvelle décision. Changer de personne n'est pas une précondition de produit.

## Scénarios proposés

| Critère | Assertion prévue | État |
| --- | --- | --- |
| Témoin A/B | Actions A/B, arrêt sain et reprise autorisée sur clones témoins avant les variantes correspondantes | Voir reçu associé |
| Deux relais | R2 reçoit R1 ; réception sans arrêt ; application seule stoppe A ; B agit ensuite | Voir reçu associé |
| Portée/sens altérés | À chaque relais : cible B/global, interdiction de discussion, émetteur/décision remplacés refusés ; original inchangé | Voir reçu associé |
| Chaîne incomplète | Reçu manquant, chaîne inversée, prédécesseur étranger et contenu de reçu altéré refusés ; chaîne valide encore applicable | Voir reçu associé |
| Rejeu | Doublon sans ajout de décision ni réécriture de l'original | Voir reçu associé |
| Effets et reprise | Anciennes observations engagées/inconnues conservées, nouvelles actions A refusées ; nouvelle décision liée permet A/B | Voir reçu associé |
| Reprise invalide | Ordre non enregistré, même décision, mauvais arrêt de référence ou absence d'arrêt refusés ; reprise autorisée ensuite | Voir reçu associé |
| Sensibilité de l'oracle | Omission volontaire de garde dans le seul banc produit arrêt B ou nouvel effet A ; aucune mutation du moteur | Voir reçu associé |

Toute opération refusée lève un code `Refused`, sans modifier l'état complet du
banc, y compris les journaux et reçus. Aucun diagnostic n'est stocké dans cet état.
Les états avant/après sont comparés, pas seulement l'exception. Les effets déjà
engagés/inconnus ne sont jamais effacés ; aucune compensation ou réconciliation
réelle n'est simulée. Les décisions d'authentification restent hors du banc.

## Provenance historique des comparaisons Go / Rust

Cette section décrit les sources de travail lues le 15 novembre 2026, pas les
capacités de la version livrée. Les extraits exacts et leurs empreintes sont dans
le dossier privé de preuve `a4-private-20261115-9801ea9866/context.json`, SHA-256
60db7e052c73c1118c236c77da267329f9830fc7675b3e4d8d00b30dbd601706.
L'index local de cette preuve conserve son emplacement et les reçus. Ce contexte
et les archives ci-dessous ne sont pas annoncés comme des fichiers livrés par main.

Les deux identifiants Go suivants désignent des sources conservées sur disque dans
le checkout canonique, avec les mêmes octets que ceux fournis à GLM. Ils ne sont
présents ni dans l'arbre CI `019dd19c1f92ec7fe0bd2df14639a25e96f5cf9e`,
ni dans la référence locale main relevée `a7bb657fb8b56c410913c2f6850e74201b4dec52`.
Ces observations portent sur ces révisions exactes, pas sur un main futur.

| Identifiant de source historique, pas lien livré | SHA-256 de l'original conservé |
| --- | --- |
| internal/engine/run_supervision.go | c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f |
| internal/engine/run_supervision_test.go | 93d492c29d71953acbf611a2c3582789170dfe3793e93ab51587fc5220832450 |

Les originaux complets, relus et comparés aux empreintes, sont conservés dans
`/run/media/fpizzi/SWARM/a4-evidence/a4-private-20261115-9801ea9866/archives-doc04/`,
aux mêmes sous-chemins ; `provenance.json` conserve les observations des arbres Git.
Il s'agit d'une archive privée de preuve, pas d'une dépendance ni d'un lien public.
Dans ces seuls originaux, les lignes 125–195 du premier décrivent une liaison
stop-run à tâche/scope, tentative et observation ; les lignes 66–150 du second
contiennent un scénario d'arrêt confirmé. Le driver Python ne les appelle pas.
Le champ « historique Go » ne prouve ni conformité Go/Rust actuelle ni présence
ou absence d'une capacité dans main. Aucun fichier Go n'est ajouté par ce candidat.

Les comparaisons Rust ci-dessous sont, elles aussi, limitées aux extraits de
travail conservés dans le même context.json ; elles ne qualifient pas les octets
du binaire installé, ni la parité actuelle avec les originaux Go :

- `rust/crates/swarm-cli/src/agent.rs:55–100` et
  `rust/crates/swarm-engine/src/root_store.rs:1790–1812` montrent l'enregistrement
  d'une intention et un reçu CLI termination_confirmed=false dans ces extraits.
- `rust/crates/swarm-engine/src/agent_supervision.rs:414–451` y observe desired=stop
  et peut demander TERM puis KILL selon la politique de ces sources. Aucun signal
  ni fin effective n'est éprouvé par le banc synthétique.
- `rust/crates/swarm-engine/src/planning_decision.rs:325–340,698–715` y refuse des
  opérations non prises en charge et des entrées spécialisées run_observation.
  Ce constat de source ne suffit pas à décider un portage ou une correction moteur.

## Commande à attribuer et limites de livraison

Pour une fixture privée attribuée, après stabilisation des entrées retenues : `python3 tests/a4_relay_contract.py -v`.
Sept méthodes de test nommées sont proposées, avec variantes en subTest ; le nombre
réel exécuté et les résultats se lisent dans la sortie nommée du reçu associé. Aucun watchdog global, timeout,
retry, réseau, shell, processus enfant, fournisseur ou mission réelle utilisé.
Les deux relais et les valeurs littérales A/B sont des données/invariants du
scénario, pas une politique opérateur. Bibliothèque standard uniquement.
La recette permanente proposée reprend cette commande et attend les sept noms,
le compte exécuté et OK ; son exécution est distincte de la commande directe. Ses entrées ne sont pas un
inventaire du moteur/build natif, qui n'est pas consommé par ce banc autonome.

Conserver sortie complète, code de sortie, source/profile pertinents, empreintes
du driver/contrat et état de chaque assertion. Toute erreur reste ouverte ; un
problème de l'oracle exige correction du banc et non prétendue réparation Swarm.
Le prochain lot devra sélectionner un parcours public natif et prouver la liaison
aux vrais producteurs/consommateurs avant de qualifier la fidélité du moteur.
Pas de compilation, installation, Git ou capacité produit livré dans cette remise.
