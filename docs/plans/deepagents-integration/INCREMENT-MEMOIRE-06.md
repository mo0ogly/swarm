# Dialogue : confirmer un tour avant de le conserver comme répondu

Reprise du 16 octobre 2026 après « next », Codex seul, méthode APEX et `replan`.
Le contexte de départ est désormais lié à son empreinte. La suite porte sur la
frontière de fin d’un tour, préalable à une éventuelle reprise de conversation.

## Observation et décision

Au début du lot, `runAgentDialogue` marque un tour `answered` après un code de
processus nul et une identité de session. Il écrit ce statut avant d’envoyer la
fin au moniteur. Celui-ci peut pourtant avoir reçu un refus fournisseur ou
détecter des résultats d’outils manquants. Un processus terminé ne suffit donc
pas à établir une réponse complète.

Deux options : contrôler seulement le flux dans le wrapper, ou faire confirmer
la frontière par le moniteur qui possède les compteurs cumulés et les résultats
d’outils. La seconde est retenue. Le canal d’autorisation existant confirme aussi
la fin : événement terminal positif unique, identité cohérente, absence d’échec,
d’activité non prise en charge, de résultat manquant ou d’erreur de persistance.
Le statut `answered` n’est écrit qu’après cette confirmation. Un refus empêche
l’échange suivant et conserve une interruption, sans rejouer le fournisseur.

La classification sûre des refus fournisseur existante est réutilisée ; le texte
privé d’une erreur n’est pas repris dans le diagnostic public. Les usages déjà
observés restent comptabilisés, même si la fin n’est pas confirmée. La voie
Codex attend `turn.completed` ; la voie Claude attend un `result` de succès sans
`is_error: true`. Les sorties ambiguës, contradictoires ou dépourvues de marqueur
positif sont refusées. La sortie zéro ne remplace pas cette preuve.

## Travaux et critères

| Lot | Périmètre | Vérification |
| --- | --- | --- |
| M10.1 | Wrapper, moniteur, table `agent_dialogue_turns` et boucle terminal | Reproduire un faux `answered` avec processus local : refus fournisseur en sortie 0, fin positive absente, outil sans résultat. |
| M10.2 | Confirmation de fin et état de protocole par tour | Refus des flux incomplets/contradictoires ; autorisation après contrôle et persistance ; écriture conditionnelle unique du statut. |
| M10.3 | Sous-processus réels de fixture, canaux fd3/fd4 et reprise entre tours | Deux tours confirmés dans la même session, compteurs cumulés, prompt suivant distinct, refus avant second appel après erreur, panne de stockage et redémarrage implicite refusé. |
| M10.4 | Documentation, consommateurs et RETEX | Tests ciblés et `-race`, analyse statique, compilation, docs FR/EN et preuves conservées. |

Les fixtures ont leur propre base et un exécutable local ; aucun appel de modèle
ni session utilisateur n’est effectué. Les préimages conservent les modifications
antérieures. Aucun nouveau seuil d’exécution, champ de configuration ou droit de
reprise n’est introduit.

`answered` qualifiera la fin technique d’un tour, pas une acceptation métier ni
une preuve de fidélité de la réponse. Le wrapper déjà commencé restera interdit
au redémarrage implicite : une vraie restauration doit aussi reconstruire les
gardes cumulées et qualifier les effets incertains. Ce lot ne le prétend pas.

## État de réalisation

Les quatre tâches sont réalisées et vérifiées dans le périmètre local : trois
faux statuts reproduits avant correction ; dix tests distincts verts, dont six
avec `-race` ; vraie compilation et parcours superviseur/terminal/CLI ; analyse
statique, documentation FR/EN et RETEX. Les preuves et limites sont dans
[RESULTATS-MEMOIRE-06.md](RESULTATS-MEMOIRE-06.md). Aucun appel de modèle,
déploiement ou succès de restauration après arrêt n’est revendiqué.
