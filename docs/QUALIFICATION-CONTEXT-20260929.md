# Qualification réelle — reprise après refus

## Verdict : PASS sur un scénario borné

29 septembre 2026. Binaire corrigé testé dans un dépôt et une base jetables,
indépendants de la mission Administration, qui reste en pause à 0/8.

Scénario : implémenter `is_expected(value)`, vrai seulement pour la chaîne
`expected`. Après la première production correcte, le banc injecte une faute
métier une seule fois. Les contrôles externes doivent refuser cette production ;
le sous-planificateur doit décider de la correction, puis obtenir contrôle et
revue indépendante sur le résultat corrigé et clôturer son périmètre. Le
responsable principal doit ensuite clôturer le sien.

## Résultat mesuré

- Durée totale, arrêt du banc compris : 306,1 secondes.
- 2 productions Claude réelles, 8 appels d’outils chacune (plafond 20 chacune).
- 5 activations de planification sur 12 autorisées ; 1 revue sur 4 autorisées.
- Défaut injecté détecté, première production refusée.
- Deuxième production acceptée, revue indépendante `passed`.
- Périmètres racine et sous-planificateur `closed`.
- Aucun appel manuel de correction, acceptation ou augmentation des limites.
- Banc terminé ; arrêt des processus confirmé par son nettoyage final.
- Coût rapporté par les deux producteurs : 0,655538 USD. Ce montant exclut
  la planification et la revue : ce n’est pas le coût total du scénario.

## Reproduction

`python3 tests/controlled_autonomy_campaign.py prepare <dossier-neuf> --engine <binaire> --providers <configuration-locale> --provider claude`

Avant départ, limiter `profile.limits.max_tool_calls` du manifeste de recette à
20 (limite inférieure au défaut du banc), puis :

`python3 tests/controlled_autonomy_campaign.py run <dossier-neuf>`

Le banc utilise le CLI public. Il ne modifie pas directement la base et n’accepte
pas de résultat lui-même. Les sorties brutes restent hors dépôt dans
`/tmp/swarm-qualification-context-20260929`. Le résumé JSON voisin conserve les
empreintes, identités de tentatives et mesures sans exposer les prompts complets.

## Ce que cette preuve ne démontre pas

Une fonction et une correction contrôlée ne représentent pas une mission de
refonte en huit tâches. Ce scénario ne passe pas par la préparation web et ne
prouve pas directement la transmission d’un brief révisé depuis cette page.
Il démontre une chaîne réelle de refus/correction/revue/clôture sur le binaire
corrigé ; les tests ciblés couvrent séparément la transmission du contrat courant.
Il ne démontre pas à lui seul que le correctif a réduit le nombre d’appels : il
n’y a pas de comparaison contrôlée avant/après avec le même modèle.

Prochaine qualification nécessaire : préparation et révision d’une petite mission
par le web, vérification du contrat transmis, puis livraison sans dépannage.
La qualification communautaire reste partielle jusque-là.
