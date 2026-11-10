# Aperçu du parcours produit et clarification des modèles

Demande « next », suivie de la précision utilisateur sur les modèles de travail
et de la demande d’une skill réutilisable. Branche codex/clear-product-method-names,
HEAD d4539e0041000a38acb162a17f053e318fb3db5d, checkout sale préexistant conservé.
Sauvegarde des fichiers avant ce lot : /tmp/swarm-method-preview-before-20261010.zip.

## Résultat

Depuis l’accueil Projets, « Voir le parcours produit : 5 + 6 » ouvre un dialogue
fermé par défaut : cinq fondations communes, puis six étapes par fonctionnalité.
Chaque étape sélectionnée affiche son résultat et ses preuves attendues. L’aperçu
explique PRODUCT-WORKFLOW.md ; il n’affiche aucun avancement de mission inventé.

Le lien de départ parle de modèle de cadrage. Le texte explique que le plan à
adopter définira les tâches, dépendances et validations. Les six sujets n’ont pas
été transformés en graphes opérationnels prédéfinis. PRODUCT-ARCHITECTURE.md
conserve cette distinction et la cible exprimée par l’utilisateur : groupes
d’actions, orientations de décision, validations et reprises réutilisables.

## Vérification

Binaire canonique /tmp/swarm-method-preview-20261010. Deux recettes ont utilisé
leurs propres racines temporaires ; aucune base de mission vivante utilisée.

| Critère | Contrôle | Résultat |
| --- | --- | --- |
| 5 fondations et 6 étapes, résultats distincts | projects_method_ui.cjs, vrai serveur, sélection clavier de chaque étape | PASS |
| FR/EN, sombre/etat, largeur 1440/1024/760/390/320 | recette, DOM et 16 captures inspectées par IA | PASS |
| Fermeture/Échap, retour focus, Tab/Maj+Tab, réouverture | recette navigateur | PASS |
| Catalogue en panne, aperçu encore accessible | HTTP 503 simulé sur lecture catalogue uniquement | PASS |
| Aucun départ ni mission créée | non-GET absents et work list public vide dans recette de méthode | PASS |
| Accueil, navigation, préparation, erreurs et reprise | projects_home_ui.cjs sur même binaire | PASS |
| Frontend, traduction, configuration | npm test, test:i18n après texte final, agent-workflows/check.py | PASS |
| Jetons et couleurs | 19 jetons définis, aucun littéral ajouté ni style inline coloré | PASS |
| Diff | git diff --check, delta relu contre sauvegarde | PASS |

Sources moteur inchangées dans ce lot ; suite Go globale antérieure non rejouée.
L’instantané d’intégrité courant est vérifié séparément, sans renouveler aucune
preuve historique. Les modifications préexistantes de recherche et formation
restent hors attribution. Aucune revue indépendante ni acceptation gérée annoncée.

## Incidents conservés

- Premier contrôle de focus FAIL : le comportement natif laissait Tab sortir du
  DOM du dialogue. Bouclage explicite Tab/Maj+Tab ajouté ; nouvelle recette PASS.
- Assertion anglaise attendait Verify the result alors que le catalogue partagé
  contient Check result. L’assertion a été corrigée, sans réécrire la traduction
  partagée ni déclarer un défaut de traduction produit.
- Recette historique d’accueil FAIL intermittent : la sortie du cockpit de
  bootstrap enregistre réellement POST /api/v1/visit via pagehide/keepalive. Le
  diagnostic a identifié son document source `/` et cockpit.js. Attendre une
  visite avant sortie a ensuite expiré : hypothèse de recette erronée conservée.
  L’initialisation comprend maintenant la sortie vers l’accueil et sa stabilisation
  réseau ; seuls les POST de visite de la mission fixture sont admissibles dans
  cette phase. Le parcours accueil/préparation conserve l’exigence zéro mutation.
  Recette finale PASS ; aucun code produit du cockpit changé.

## Skill demandée

Skill personnelle Codex swarm-model-design, découverte normale :
/home/fpizzi/.codex/skills/swarm-model-design/SKILL.md. Créée avec skill-creator,
quick_validate.py PASS. Elle distingue cadrage, modèle de travail et instance,
réutilise les formats publics existants, exige de vérifier chaque branche/garde
réelle, et couvre parcours FR/EN, reprise, preuves et protection des missions.
Cette validation structurelle ne prouve pas l’exécution autonome de la skill.
Aucun script ni référence supplémentaire inutile ajouté.

## Présentation

Aperçu isolé 18848 avec la racine /tmp/swarm-projects-home-hsch8k. La lecture
publique avant actualisation constate une mission fixture sans tâche ni agent.
Le cockpit 18792 et le candidat figé des campagnes ne sont pas redémarrés/modifiés.
Aucun commit, push, appel fournisseur ni mutation de workflow géré.
