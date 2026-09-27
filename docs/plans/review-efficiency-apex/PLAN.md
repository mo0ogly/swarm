# APEX — revue fiable et proportionnée

## Contrat
Objectif : supprimer les refus incompréhensibles et les réexamens sans information nouvelle, sans affaiblir l'acceptation. Mission E6 existante conservée, aucun budget augmenté ni appel réel pour développer. Base 2b14d90 ; diagnostic externe publications/debug-review-loop/DEBUG.md.

## Décision
Deux options : (A) élargir les quotas et conserver le protocole global ; (B) versionner les observations, rendre les refus exploitables et invalider selon le périmètre examiné. B retenue : A ne traite ni la répétition ni les défauts non démontrés. Aucun cache fondé sur le seul contenu d'un fichier : critères, méthode, modèle et dépendances font partie de la validité. La décision finale reste fraîche et liée au même SHA que les contrôles.

## Séquence et responsabilités
| Étape | Rôle / responsable | Périmètre | Critère / reprise |
|---|---|---|---|
| RD1 | Planification / Codex | Contrat, migration v1/v2, tests isolés | Anciennes preuves lisibles, aucun appel live |
| RD2 après RD1 | Réalisation / Codex | Refus détaillé avec citation, localisation, reproduction ; budget de réponse borné | Refus inventé rejeté ; détail durable dès premier appel ; pas d'explosion implicite du nombre de paquets |
| RD3 après RD2 | Réalisation / Codex | Registre d'observations et invalidation différentielle | Aucun unknown promu ; changement de dépendance invalide ; avis final courant requis |
| RD4 après RD3 | Réalisation / Codex | Prévol CLI/HTTP et détection des répétitions | Coût expliqué, refus identique sans nouvelle information non relancé |
| RD5 après RD4 | Vérification | Tests comportementaux isolés et mesure avant/après, suite Go/vet | Gains mesurés, vrai défaut détecté, corruption refusée ; aucune affirmation d'avis indépendant par l'auteur |
| RD6 après RD5 | Livraison / Codex | Documentation/RETEX, installation contrôlée | Même binaire testé/installé, historique intact, E6 non déclarée terminée sans revue |

Pas d'agent indépendant lancé sous couvert de cette méthode. L'avis indépendant de la mission reste celui du moteur ; les tests locaux et la relecture de l'auteur ne le remplacent pas.

## État
RD1 terminé. RD2 implémenté et testé. RD3 non terminé : réutilisation différentielle volontairement inactive tant que la couverture des dépendances n’est pas démontrée. RD4 partiel : prévol CLI/HTTP et garde contre deux interruptions identiques implémentés. RD5 : tests du premier lot réussis, recette différentielle non réalisée. RD6 : livraison du premier lot en cours, clôture globale non terminée. Limite de revue live 71, consommé 64. Aucun passage à79 autorisé dans ce plan.
