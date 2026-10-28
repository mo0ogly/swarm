# Diagnostic du collecteur de build —26octobre2026

Le lanceur conserve désormais collector-result.json : code de retour, signal
si négatif, SHA256 des deux canaux, avant le refus existant du profil. Aucun
retry, délai ou contournement de qualification ajouté. But : distinguer une
terminaison du collecteur d’un échec de compilation ou de stabilité des sources.
La terminaison historique sans canaux de la tentative81 reste inexpliquée.

Trois tests de tests/native_build_metadata_test.py passent : métadonnées,
syntaxe de version et conservation d’un signal9 simulé avant refus. Le build82
qualifié fournit le témoin réel positif returncode0/signalnull ; les erreurs
originales et ce résultat sont retenus avec la campagne préparation du26octobre.
Le test simulé ne prouve pas la cause de la terminaison historique. Les échecs
écriture des preuves restent propagés et ne qualifient aucun candidat.

Owner installation/Codex. Commit séparé de la fonctionnalité de préparation :
ce diagnostic peut être repris indépendamment. Aucun push demandé.
