# E6 — preuve réelle contrôlée, 23 septembre 2026

## Résultat et portée

**PASS pour la campagne isolée. E6 de la mission principale reste non validée.**
Le moteur testé est `1a93811e81739a392be727780639eadeded97bf1`, binaire
`c52bcce0648d6d51516e8e29524af67c4dcf22a0fcd423ae1194efe42d3d457b`.
La copie interrompue de la mission principale n'est pas cette version : aucun
transfert implicite de validité vers cette copie, ni remise à zéro des quatre essais.

## Parcours observé

Un responsable réel délègue à un sous-planificateur, qui crée une tâche.
La première production correcte est altérée par le banc autorisé : la fonction
accepte à tort `wrong`. Le contrôle moteur la refuse. Le sous-planificateur
décide `retry`, la seconde production corrige, les contrôles et le vérificateur
indépendant passent sur le même candidat. La tâche est acceptée, puis les deux
responsables ferment leurs périmètres. Aucune décision de reprise, d'acceptation
ou de fermeture n'a été fabriquée par l'observateur.

- Travail isolé : `w-8b821fb542b4a765d0031644`.
- Consommation : 5 activations de planification, 2 productions, 1 revue.
- Plafonds : 12 / 2 / 4 ; fenêtre de 20 minutes.
- SHA contrôles / revue / publication : `3f9ccc56e97ee73851c4703c8eb328d96479b7af`.
- Résultat du lanceur : code 0, `PASS`, aucune preuve manquante.
- Agents arrêtés : confirmé ; arrêt du serveur de campagne par le lanceur.
- Coût monétaire : inconnu, ne pas confondre compte d'appels et facturation.

## Reproduction et preuves

Le lanceur est `tests/controlled_autonomy_campaign.py`, commandes `prepare` puis
`run`, avec un dossier neuf, un moteur figé et une autorisation distincte.
Ne jamais relancer un dossier contenant `run-started.json`.
Les empreintes des preuves sont dans [le manifeste](e6-real-trial-20260923.json).
Les traces brutes et configurations restent hors Git. Les essais précédents FAIL
sont conservés. Les tests déterministes sont distincts de cet essai Claude réel.

## Limites et suite

Ce petit besoin vérifie une boucle, pas une fiabilité statistique, les gros dépôts
ou les conflits simultanés. Le rattachement à la mission principale exige un
candidat cohérent avec le moteur testé, ses contrôles et sa propre revue fraîche.
E7 et E8 ne sont pas validées par cette campagne.
