# APEX — diagnostic du découpage des revues

## Périmètre livré
Détecter quand organiser une revue, identifier les cinq types de découpage,
proposer un inventaire de lots et expliquer les contraintes. Cette étape ne
transforme pas une proposition heuristique en plan exécutable et ne crée pas un
faux sous-planificateur. L’exécution coordonnée de nouveaux lots sémantiques reste
un chantier distinct : le moteur actuel sait exécuter des lots par tâche et des
fragments techniques, pas accepter arbitrairement ce nouvel inventaire.

## Analyse et décision
Un simple plafond de100 fichiers confond périmètre et capacité de revue.
Conserver les garde-fous existants, mais enrichir le précontrôle avec un diagnostic
lié au couple base acceptée/candidat. Les alternatives étaient une réduction
automatique des fichiers (risque de perte de dépendances), un découpage aveugle
par taille (aucune cohérence métier), ou une proposition explicite à examiner.
La troisième approche est retenue pour le diagnostic.

## Détection
- Plus de100 fichiers : signal de remise étendue.
- Diff seul supérieur à192 Kio : dépassement de la borne actuelle du contexte de
  revue unique. Cette mesure n’est ni le nombre de tokens ni le coût total.
- Au moins4 groupes techniques et20 fichiers : mélange de responsabilités.
- Un petit correctif code/test/documentation ne déclenche pas une délégation.

Ces seuils sont des conventions locales explicites, pas des règles Cursor.

## Types proposés
1. Exigence : regrouper par résultat et critères.
2. Composant : proposer des responsabilités techniques à partir des chemins.
3. Dépendance : examiner contrats, cycles et ordre entre lots.
4. Spécialité : autoriser des examens complémentaires avec recouvrement déclaré.
5. Volume : fragmenter seulement après structuration, sans tronquer les preuves.

Chaque fichier apparaît exactement une fois dans l’inventaire initial par
composant. Cela prouve une couverture de chemins, pas une indépendance sémantique.
Les dépendances portent explicitement la mention « non examinées ». Les critères,
les références croisées et la revue finale doivent être établis avant exécution.

## Interfaces et preuves
`planning scope-preview` et l’API GET correspondante exposent `decomposition` :
`planning_required`, `reasons`, `diff_bytes`, `cut_options`, `suggested_lots`,
`candidate_commit`, `state=proposal_only`, `final_review`.
L’export de patch reste disponible comme option distincte, jamais automatique.
Aucune augmentation de budget, aucun appel fournisseur ni mutation par le diagnostic.

Tests : TestReviewDecompositionSignalsAndExactCoverage couvre petit correctif,
gros fichier unique, frontière192Kio, nombre de fichiers, mélange de composants,
couverture exacte et déterminisme. TestManagedScope couvre le parcours CLI/API,
la conservation et la revue après une réduction explicitement soumise.

## Suite architecturale non livrée par ce diagnostic
Un véritable sous-planificateur de revue doit proposer un plan ancré au SHA,
avec critères, lots, dépendances et budget ; le moteur doit le valider et le
persister, exécuter les avis locaux, vérifier les interactions et invalider les
avis affectés par une correction. L’inventaire heuristique ne peut pas autoriser
cette exécution ni lever seul le garde-fou actuel.

## Étape suivante — proposition versionnée

Ajout de `review-plan` aux opérations du responsable. Les entrées exactes sont
fournies dans `review_planning_inputs` et la proposition est ancrée au candidat
et à l’empreinte du contexte. La validation vérifie couverture, références, DAG
et revue finale ; le résultat persiste dans `task.review_coordination` sous
`validated_not_executed`, sans revue payante. Le test public couvre la décision
et son rejeu. Les tests négatifs couvrent omissions, références étrangères,
cycles, identités dupliquées, preuve périmée et absence de revue finale.

Surprise vérifiée : `planningDeliveryContext` est également appelée depuis la
transaction de claim, avec une seule connexion SQLite. Lire le Store depuis ce
contexte provoquerait un blocage. Les nouveaux précontrôles sont donc préparés
avant la transaction puis liés à sa révision ; le rendu du contexte reste pur.
L’exécution des lots demeure distincte et non livrée par cette étape.

## Raccordement des lots — 28 septembre 2026

Choix : conserver le journal d’inspections existant et lui ajouter les lots et
contrats approuvés. Alternative écartée : créer une seconde machine de revue,
qui dupliquerait réservations, erreurs, reprise et règles de publication.

Le chemin `review-plan → retry-review → réconciliation → revue finale → publication`
est couvert par une recette isolée avec fournisseur déterministe. Le candidat
publié doit être celui du plan ; un rejeu ne dépense aucun appel supplémentaire.
Tests supplémentaires : ordre des dépendances, omission/altération des preuves,
critères inconnus, budget final insuffisant, échec du fournisseur et verdict négatif.
La couverture des lots et les preuves communes sont intégralement conservées.

RETEX : un inventaire ou une proposition validée ne suffit pas à rendre le
moteur autonome. Il faut tester le raccordement par l’entrée publique, jusqu’à la
publication, et distinguer cette preuve mécanique d’une revue par un vrai modèle.
Le nouveau parcours reste séquentiel ; il ne prétend pas créer un agent par lot.
E6 n’est pas acceptée par ces tests et les 73 appels déjà consommés restent comptés.

### Précontrôle E6 sur preuves exportées — 28 septembre

L’application au dossier réel a révélé deux écarts absents des petits scénarios :
les consignes de lot étaient ajoutées après le calcul de taille, et la revue
finale coordonnée dépassait la limite historique de message. Corrections :
compter les contrats avant de remplir un paquet ; conserver les frontières de
lots lors de l’adaptation à la capacité observée du client/modèle. Le coût affiché
et l’exécution choisissent désormais le même protocole sous le même budget.

Le dossier immuable exporté par `planning review-dossier` a été précontrôlé hors
ligne, sans utiliser la base active comme fixture et sans appel au modèle.
Candidat `5bf7197cd65dee3eb3868b7712b532f0ce0e09a1` : dix lots,
16 inspections (preuves communes incluses), deux appels finaux, transport complet
compatible avec la capacité locale observée de `gpt-5.6-sol`. Ce résultat est un
précontrôle de capacité, pas un avis favorable sur le candidat.

Autre cause de lenteur confirmée dans le code : plusieurs anciens événements
pour une même tâche provoquaient des reconstructions identiques de l’inventaire.
Le contexte réutilise maintenant cet inventaire pendant une même préparation.
Les événements historiques restent conservés ; leurs acquittements passent par
les décisions publiques du responsable et consomment le budget prévu.
