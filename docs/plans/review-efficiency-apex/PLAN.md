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
RD1 et RD2 terminés. RD3 implémenté : observations historiques conservées sous leur
identité originale, invalidation du groupe si une entrée change, nouvel examen
obligatoire des impacts. La décision d’architecture révisée et ses limites sont
expliquées dans DIFFERENTIAL.md. RD4 : prévol CLI/HTTP partagé, coûts et capacité de
transport distincts, garde contre deux interruptions identiques. RD5 : recette
isolée différentielle réussie, suite Go complète 296.292s, race ciblée 182.519s,
vet et contrôle du contrat réussis. RD6 : correctif 9da8a65 installé, CLI/HTTP identiques et sans mutation,
assets et compteurs conservés ; preuves dans les artefacts de livraison. E6 reste bloquée : 10 appels requis pour 7 disponibles
et décision finale trop grande. Aucun passage à 79 autorisé ; aucun appel live.

La clôture de ce correctif du moteur et celle de la mission E6–E8 sont distinctes.
La seconde exige toujours des avis indépendants et les preuves actuelles.

## Étape suivante
Le protocole 4 est intégré et recetté : admission par tokens, capacité du client
natif ancrée, réserves et preuves conservées. Prévol E6 : six appels pour sept
disponibles, transport prêt. Binaire a0780d6 installé et revue E6 relancée par l’action publique dans le
budget autorisé. Premier appel réservé : 65/71 ; aucune acceptation déduite du
seul prévol. Suivre la revue en cours puis E7/E8.
Voir TRANSPORT-NEXT.md et EVIDENCE.md.
