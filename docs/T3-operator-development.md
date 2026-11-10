# T3 — exploration et sous-planification concurrente

Priorité explicite de l’utilisateur : développer T3 avant le correctif27 différé.
Le service26 et la mission native restent conservés. T2 est développée mais non
acceptée ; aucun agent natif T3 n’est encore lancé. L’API installée a refusé la
révision de dépendances d’un plan géré ; les pauses de révision ont été restaurées.

Le candidat isolé compose le parent26 qualifié, les six fichiers Go du juge T2
natif original et les fonctions T3. Deux conflits de lignes ont été résolus en
conservant les commandes CLI et les champs de contexte des deux versions.
La copie native T2 et ses originaux restent inchangés. Le correctif27 est exclu.

| Exigence | Effet et vérification locale |
| --- | --- |
| req-9 | POST decide/explore, lecture du Git accepté, originaux reçus après réouverture ; copie source modifiée ignorée |
| req-10 | Découverte suivie de création de tâche et révision des dépendances futures ; ancien/nouveau graphe attribué |
| req-11 | Deux sous-processus de rôles distincts simultanés, réservations et compteurs propres/ancêtres/partagés |
| req-12 | Pause annule les décisions actives ; refus de budget, préparation déjà figée, graphe invalide, source périmée et rejeu sans effet double |

Les tests T3 et Judge composés passent en8.406s. Les sous-processus sont des
fournisseurs déterministes de test, pas une recette de fournisseurs IA réels.
Race, suite complète, vet, méthodes et build restent à qualifier sur le candidat
final figé avant installation. Aucun résultat local ne vaut acceptation native.

Architecture FR avec Mermaid : CURSOR-ENGINE-EVOLUTION-20261010.md.
Architecture EN avec Mermaid : docs/en/CURSOR-ENGINE-CONTRACT.md.

Reprise nécessaire : qualifier et installer T3, transmettre le nouveau contrat
au superviseur par opération publique, observer sa décision sur les dépendances
T4 puis T3 et vérifier le lancement réel de T3. Garder T2 non acceptée ; T4 doit
conserver T2 comme prérequis pour la suite. La correction27 reste différée.

## Incident de réservation sur une route différente

Après l'installation de T3, le superviseur a révisé les dépendances et lancé
une première tentative native, qui a demandé un choix de convenance au lieu
de continuer, sans importer le paquet ni écrire de rapport. La deuxième est
interrompue par un quota Claude429 ; aucune modification de code n'est observée.
La route Codex est ensuite choisie publiquement, sans effacer le délai Claude.
La réservation continue pourtant de vérifier l'ancien fournisseur par défaut.
Cette frontière est corrigée dans planningChange, en résolvant la sélection
du périmètre avant le contrôle du quota. Tests de racine/enfant, sélection,
héritage, refus du bon fournisseur, compteurs préservés et replay conservés.
Les sorties rouges originales et les erreurs de construction du premier test
sont conservées séparément. Les régressions ciblées passent ; qualification
complète, installation et effet natif doivent être établis par leurs nouveaux
reçus. Aucun succès de mission ni acceptation n'est déduit de cette correction.
