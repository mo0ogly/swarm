# T5 — Voir l’effort par tâche : remise vérifiée du superviseur

## Résultat et critères
1. Chaque tâche affiche durée des processus enregistrés, appels d’outils observés, revues enregistrées pour cette tâche, reprises et coût rapporté. La somme des processus peut se chevaucher : ce n’est pas le temps écoulé de la mission. Les appels internes au modèle restent non mesurés.
2. CLI et cockpit reposent sur la même agrégation et la même provenance datée : horaires des processus, activité reçue, usages fournisseurs et historique des revues. Une tentative sans horaires conserve une durée null ; un zéro connu reste zéro. Outils et coût inconnus sont « non rapportés ». Aucun montant estimé n’est présenté comme rapporté.
3. Recette sur données synthétiques explicitement isolées : deux processus de 120000 et 30000ms se chevauchent (temps écoulé 120000ms, somme 150000ms), 11 outils, 2 revues, 1 reprise, 0.25USD rapporté et un coût manquant. Une autre tentative sans horaires/outils/coût conserve ces inconnues. Les tests exécutent vraiment le CLI, HTTP et le cockpit embarqué, pas un fournisseur IA. FR/EN, thèmes État/sombre, ouverture clavier, Échap, retour du focus, revision et événements inchangés sont démontrés. Aucune preuve d’autonomie globale n’est revendiquée.

## Attribution et limites
Le premier agent T5 a ajouté comptage par tâche et horaires, puis a été interrompu au plafond initial de 60 appels sans remettre son rapport final. Son analyse initiale est conservée dans T5-worker-initial.md. Le superviseur a terminé durée, inconnues, traductions, fixtures et recettes ; ce n’est pas une réalisation autonome du seul agent.
La suite Go native a PASS en 408.945s (T5-go-final.log), avant la dernière addition de traduction du coût composé. Après cette addition, contrôles ciblés et matrice cockpit rejoués PASS, puis frontend et vet. Une suite précédente interrompue volontairement par le superviseur reste conservée (T5-go-before-freeze-interrupted.log), jamais comptée PASS.
La recette de captures a d’abord sélectionné un élément du fond de page au lieu de la modale ; sélection resserrée à #modal puis recette rejouée. Ce défaut concernait la recette, pas une preuve de défaut du défilement du produit.

## Preuves
- T5-targeted-final.log : tests Go détaillés, y compris CLI bilingue sans libellé français du coût.
- T5-browser-result.json et T5-browser.log : quatre variantes langue/thème PASS, sans erreur console.
- T5-effort-fixture.json : valeurs synthétiques et provenance.
- T5-cli-fr.txt et T5-cli-en.txt : sorties réelles.
- docs/screenshots/qw7/ : huit captures summary/attempts, FR/EN, État/sombre ; lisibilité des valeurs inconnues vérifiée.
- T5-candidate.json / T5-candidate.diff : HEAD, candidat sale et SHA binaire ; aucune release ni commit nouveau.
- tests/qw7_acceptance.py : commande reproductible, racine SQLite temporaire ; aucun accès au stockage vivant.

## Prochaine étape
Un deuxième agent dans la tentative restante relit cette remise et rédige son handoff normal sans recommencer l’implémentation. Les contrôles préautorisés, la revue indépendante et la décision normale restent nécessaires. Aucun PASS futur ou acceptation anticipée.
