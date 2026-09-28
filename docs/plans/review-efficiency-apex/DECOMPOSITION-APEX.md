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
