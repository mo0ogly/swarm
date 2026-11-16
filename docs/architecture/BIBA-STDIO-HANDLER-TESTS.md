# Branches supplémentaires du consommateur stdio — proposition non exécutée

Ces huit tests supplémentaires forment une suite séparée. Ils ne modifient ni
les huit tests de rétention précédents, ni le candidat caller r8 en revue.
Le harnais réutilise ses helpers typés exacts et appelle l'entrée publique
`run_git_stdio` -> `stdio_probe` avec le collecteur réel en Python. Son
subprocess est intégralement doublé avant chargement du candidat ; les modules
proof/fixture sont des doubles explicites. Les programmes embarqués sont écrits
comme texte mais jamais exécutés. Aucun Biba/Git/driver/fournisseur n'est lancé.

| Cas nommé | Résultat attendu, non exécuté |
| --- | --- |
| healthy_handler_control | Un appel inerte, JSON valide, chemin public sain |
| known_completion_malformed_and_postcheck_control | Achèvement0, cause JSON réelle et postcheck réuni |
| unknown_completion_does_not_read_or_parse_retained_bytes | Octets présents mais code inconnu : aucune deuxième lecture/parsing |
| spawn_failure_does_not_read_or_parse_empty_channels | Échec avant achèvement : pas de parsing d'un vide |
| missing_stdout_hash_does_not_read_or_parse_again | Hash refusé par le collecteur : aucune relecture de remplacement |
| changed_retained_stdout_groups_hash_and_original_errors | Hash connu puis octets modifiés : cause d'intégrité + cause initiale |
| unreadable_retained_stdout_groups_read_and_original_errors | Lecture tardive refusée : cause IO + cause initiale |
| final_failure_publication_keeps_both_original_causes | Cause JSON + cause initiale + refus réel de publication finale |

Chaque injection a son témoin sain et sa récupération sur de nouveaux dossiers,
avant les assertions susceptibles d'échouer. Les compteurs distinguent la
lecture de hash initiale du collecteur de la relecture du handler. Ils contrôlent
aussi l'absence de parsing quand il est interdit. Les types d'exceptions et
champs d'issue sont inspectés structurellement, sans recherche dans un texte.
Le défaut de canal modifié est une injection sur notre fixture synthétique,
après son reçu de hash ; aucune preuve d'exécution originale n'est modifiée.

La CLI exige --candidate et son --candidate-sha256, --outcome-source et
--outcome-sha256, --helpers-source et --helpers-sha256, --evidence-parent existant
et admis par le propriétaire de stockage. La recette proposée utilise les
variables SWARM_HANDLER_CANDIDATE_SHA256, SWARM_HANDLER_OUTCOME_SHA256,
SWARM_HANDLER_HELPERS_SHA256, SWARM_HANDLER_EVIDENCE_PARENT avec les chemins
canoniques. Les pins, sources, essais, stdout candidat capturé, sortie unittest
et résumé sont retenus ; chaque pin est revérifié indépendamment après la suite.
Aucun délai/retry n'est ajouté. Le nombre huit est cet inventaire supplémentaire,
pas un quota ni une preuve héritée des tests précédents. Revue puis autorisation
d'exécution distincte restent nécessaires. Cette suite ne qualifie ni la politique
Admin, ni le writer atomique, ni le confinement, ni une installation.
