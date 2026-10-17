# Mémoire de reprise : empreinte du contexte effectivement transmis

Reprise du 16 octobre 2026 après « next », Codex seul, méthode APEX et `replan`.
Le besoin mémoire dépasse les comparaisons de réponses : la prochaine étape
vérifie la traçabilité du contexte d’une nouvelle tentative après interruption.

## Écart observé et périmètre

`agents_store.go` calcule le manifeste avant d’ajouter les consignes de livraison
gérée. `agents_process.go` ajoute ensuite la projection de reprise au texte envoyé
au fournisseur, sans recalculer ce manifeste. `saveAgent` sérialise ces deux
valeurs telles quelles. L’hypothèse à reproduire est donc une empreinte et une
taille ne correspondant pas au prompt complet. La projection ajoutée au départ
doit aussi respecter la capacité déjà vérifiée pour le contexte préparé.

Le refus de redémarrage implicite de `runAgentDialogue` reste nécessaire : il ne
constitue pas une restauration de conversation fournisseur. Ce lot qualifie la
mémoire de reprise d’une nouvelle tentative, sans réinitialiser consommations,
droits, résultats antérieurs ou identité de session. Aucun appel de modèle.

## Décision et contrat

Deux options : présenter le manifeste existant comme celui du seul préfixe, ou
lier le manifeste courant au texte complet tout en conservant celui de préparation.
La seconde est retenue : le consommateur lit un manifeste décrivant le prompt
effectif ; `prepared_context` conserve le manifeste avant ajout de la projection
de reprise. Ce champ est une évolution additive du reçu `Agent`, pas une nouvelle
commande de reprise ou une preuve d’acceptation.

Le calcul final de préparation intervient après les consignes de livraison. Au
départ, le contexte de reprise est ajouté, la capacité existante est contrôlée,
le manifeste effectif est calculé et enregistré **avant** de démarrer le
fournisseur. Un échec de calcul, de capacité ou d’écriture empêche ce départ.
Le texte est conservé sans troncature silencieuse ; les références mémoire et
leurs décisions sont maintenues. Le manifeste préparé n’est pas modifié par le
recalcul. Les anciens enregistrements ne sont pas réécrits rétroactivement.

## Travaux et preuves attendues

| Lot | Périmètre | Critère observable |
| --- | --- | --- |
| M9.1 | Producteurs, stockage et consommateurs de contexte | Contre-exemples avant correction : livraison gérée et projection de reprise donnent un manifeste ne correspondant pas au texte. |
| M9.2 | Assemblage commun et reçu Agent | SHA-256 et taille exacts, manifeste préparé distinct, copie sans mutation des références ; capacité existante contrôlée après ajout. |
| M9.3 | Parcours réel `prepare` → `supervise` avec fournisseur local de test | Octets reçus identiques au texte conservé et à son manifeste après réouverture ; échec de capacité avant invocation ; historique et consommation antérieurs inchangés. |
| M9.4 | Consommateurs et documentation | Tests ciblés de mémoire, contexte, reprise et source d’observation ; docs FR/EN, diff et RETEX. |

Les modifications préexistantes des fichiers du moteur sont conservées dans les
préimages du lot avant édition. Les tests emploient uniquement des racines
temporaires et un adaptateur local, pas une mission en cours ou un modèle réel.
La capacité de contexte du moteur reste celle existante, sans nouvelle politique
de rétention, de budget ou d’exécution. L’interface web n’est pas redessinée.

Les garde-fous du dialogue, la reprise complète d’une conversation, le résumé
progressif et la purge ne sont pas déclarés réalisés par ce correctif.
