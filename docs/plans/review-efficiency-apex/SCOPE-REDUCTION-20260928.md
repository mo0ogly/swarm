# APEX — réduire la remise avant de financer sa revue

## Problème et décision
Le candidat E6 `5bf7197cd65dee3eb3868b7712b532f0ce0e09a1` rassemble
352 fichiers depuis `c4143b6a8ad9c373f5ef3a23734205e601198539`.
Le calcul de transport proposait11 appels, alors que le vrai problème était le
périmètre excessif. Augmenter le budget aurait financé cette accumulation.

Choix : borner le delta de chaque nouvelle remise à100 fichiers avant tout appel
de revue, puis permettre une sélection explicite et exportable. Alternative
rejetée : retirer automatiquement des fichiers supposés inutiles, au risque de
supprimer des dépendances. Le seuil est une politique Swarm, pas une propriété
scientifique ou une directive Cursor.

## Exigences et preuves

| Exigence | Vérification |
| --- | --- |
| Zéro revue dépensée pour une remise trop large | TestManagedScopeBlocksBeforePaidReviewAndExportsExactSelection |
| Pas de contournement par reprise de précontrôle | Même test : prepareManagedPreflightRetry refusé |
| Delta depuis la base acceptée | scope-preview E6 réel :352 fichiers, mêmes SHA |
| Sélection littérale et liste des fichiers différés | Patch appliqué sur clone jetable ; fichier différé absent |
| Pas de mutation par export | Work et ManagedAttempt identiques avant/après ; budget inchangé |
| CLI/API identiques | Comparaison du JSON scope-patch |
| Nouvelle revue après réduction | Remise complète réduite acceptée par le moteur de test avec1 revue ; ancien résultat conservé |
| Pas de demande périmée ou chemins injectés | Révision périmée, chemin inconnu et doublon refusés |

Ces tests utilisent un vérificateur déterministe ; ils ne constituent pas une
nouvelle revue IA du candidat E6. L’export réel E6 est en lecture seule et ne
réduit pas encore sa copie. Aucune augmentation du plafond75 n’est effectuée.

## RETEX
Ne pas confondre capacité de transport et cohérence de livraison. Une grande
fenêtre de contexte n’autorise pas à fusionner toutes les corrections accumulées.
La réduction crée un nouveau candidat à tester ; elle ne recycle pas les verdicts
du candidat plus large. Les fichiers différés restent à intégrer séparément.

Documentation : `docs/DELIVERY-SCOPE.md` et `docs/en/DELIVERY-SCOPE.md`.
