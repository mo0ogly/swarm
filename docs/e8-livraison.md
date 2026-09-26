# E8 — dossier de livraison et reprise directe

## Verdict au 26 septembre 2026

**Livraison technique préparée ; mission principale non close (5/8).**
Le 26 septembre, l'utilisateur a demandé que Codex reprenne directement le dossier.
La surveillance périodique a été supprimée : elle constatait le blocage sans le
résoudre. Cette intervention externe est explicite, pas une autonomie démontrée.

Le lanceur comportait un scénario `delivery`, mais aucun test correspondant.
`engine_contract_delivery_test.go` raccorde maintenant ce scénario à six contrôles
comportementaux existants, comprenant vingt cas et sous-cas. Il ne modifie aucune
règle d'acceptation. Les tests emploient des Stores isolés et des fournisseurs
simulés ; aucun nouveau fournisseur payant n'est lancé par cette recette.

## Matrice exigences et preuves

| Exigence | Contrôle reproductible | Limite |
|---|---|---|
| C1 — responsabilité et délégation | scénario `ownership` | Fixtures, pas une nouvelle campagne réelle |
| C2 — copies et canaux isolés | scénario `ownership` | Conflits couverts par les cas testés |
| C3 — retours et reprise | scénarios `recovery`, `real` | Retour simulé dans la recette déterministe |
| V1 — vérificateur obligatoire | scénario `launch` | Disponibilité réelle du fournisseur distincte |
| V2 — indépendance | scénarios `launch`, `delivery` | Cette reprise Codex n'est pas un avis indépendant |
| V3 — même candidat | scénarios `revision`, `delivery` | Revue réelle finale encore absente |
| V4 — invalidation des preuves | scénarios `revision`, `history` | Aucune validité transférée entre SHA |
| V5 — acceptation atomique | scénarios `launch`, `revision`, `delivery` | Pas de validation manuelle de la mission |
| R1 — reprise durable et bornée | scénario `recovery` | Le budget n'est jamais remboursé |
| U1 — vérité web/CLI | scénario `truth`, tests frontend | Recette navigateur isolée ; pas un test utilisateur novice |
| P1 — autonomie réelle | manifeste `e6-real-trial-20260923.json` | PASS historique sur moteur `1a93811`, pas sur le moteur actuel |

Commandes : `node tests/engine_acceptance.cjs --case CASE`, avec CASE parmi
`launch`, `ownership`, `revision`, `recovery`, `truth`, `real`, `history`, `delivery`.
Le lanceur refuse une sélection sans test ou contenant un test ignoré.
Résultats et durées de cette passe : `e8-verification-20260926.json`.

## Huit preuves de livraison

1. **Révision** : base source et empreinte du changement testé dans le manifeste.
2. **Contrôles** : commandes, codes de sortie et durées ; sorties brutes conservées hors Git.
3. **Rapport et limites** : ce document et les rapports E6/E7.
4. **Vérificateur** : revue `review-d836c9f94b6ab3c30178d7ac` interrompue après
   600 secondes ; aucune décision favorable sur le candidat courant.
5. **Décision** : poursuivre la reprise directe autorisée, conserver la non-validation.
6. **Fraîcheur** : l'essai historique ne couvre pas implicitement une nouvelle révision.
7. **Livrable** : branche source et bundle local de reprise, distinct du candidat accepté.
8. **Interventions** : corrections et contrôles directs Codex, installation du moteur,
   changements de budget autorisés, relances publiques et arrêt du suivi périodique.

## RETEX : pourquoi la mission s'est immobilisée

- Des reprises de revue et des diagnostics ont été livrés sans terminer le parcours
  d'acceptation de la mission. Une amélioration du moteur n'est pas une tâche acceptée.
- Un libellé de recette annonçait un contrôle absent. Il a été corrigé pour décrire
  l'assertion réelle ; une liste de preuves doit être vérifiable, jamais décorative.
- La revue cumulative examine un gros dossier ; modifier une ligne du candidat
  invalide actuellement les inspections liées au SHA précédent. Cette contrainte
  coûte du temps et des appels, même quand beaucoup de pièces sont inchangées.
- Les événements système étaient trop peu détaillés pour expliquer certains délais.
  La télémétrie corrigée ne répare pas rétroactivement les traces perdues.
- Un suivi qui ne fait que relire un état immobile ne réalise pas une supervision
  corrective. Il faut une action bornée, un résultat vérifiable et un responsable.

Priorités moteur : réutilisation des inspections de pièces identiques avec preuves
explicites de provenance et d'interactions ; estimation du coût complet avant départ ;
arrêt d'une boucle de relances sans diagnostic nouveau ; distinction visible entre
travail exécuté, tests réussis et décision d'acceptation. Ce sont des suites proposées,
pas des fonctions déclarées implémentées par ce rapport.

## Coût et clôture

Mission : 38 appels de revue consommés sur 49 autorisés lors de cette reprise ;
coût monétaire inconnu. Campagne réelle historique : cinq activations de planification,
deux productions et une revue. Les deux comptages sont distincts.
Les interventions externes documentées dans le journal ne sont pas assimilées à
zéro intervention ; leur total historique exhaustif n'a pas été reconstitué.

La clôture reste interdite tant que l'avis indépendant final et les contrôles ne
couvrent pas le même candidat et que les trois périmètres ne sont pas clos.
Un bundle exporté et des tests verts ne suffisent pas à déclarer 8/8.
