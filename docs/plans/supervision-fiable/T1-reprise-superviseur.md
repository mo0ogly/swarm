# T1 — reprise supervisée, 4 octobre 2026

La première tentative a été interrompue après 300 secondes d'outil sans résultat pendant `go test ./... -count=1`. Le plan réservait cette suite complète à la recette finale. Les 29 appels et la tentative restent consommés ; une tentative autorisée subsiste.

Le superviseur a reconstruit le candidat avec `sh build.sh /tmp/swarm-r1-recovery-candidate`. Les tests ciblés `go test ./... -run 'TestVersion|TestServerVersion|TestRuntimeHealthEndpoint' -count=1` ont terminé code0 (0,383s). Leur exécution verbose est conservée dans T1-go-targeted.log.

Le test navigateur réel `node tests/version_ui.cjs /tmp/swarm-r1-recovery-candidate docs/plans/supervision-fiable/T1-version-ui-real-final` termine code0,10 contrôles : cockpit et préparation FR/EN, deux thèmes, ouverture clavier/Échap/focus, chargement et indisponibilité. Serveur TCP isolé effectivement démarré depuis le superviseur, sans modification du sandbox du fournisseur.

Les deux premiers essais de recette sont conservés en échec : assertion divergent attendue à tort pour le serveur réel, puis libellé anglais attendu State au lieu du Status réellement traduit. Corrections limitées à tests/version_ui.cjs : état réel inconnu observé et contrat traduit exact. Ces erreurs de recette ne sont pas une preuve de défaut produit.

Le contrôle `--fixture` couvre séparément les identités distinctes et manquantes ; résultats dans T1-version-ui-fixture-final/result.json. Données simulées déclarées, aucune autonomie fournisseur revendiquée. Le CLI public sans serveur est sauvegardé T1-cli-version-real.json.

Précondition de reprise : ne plus lancer la suite complète ni tenter d'écouter TCP dans le sandbox. Examiner les preuves sauvegardées et terminer le rapport T1 avec attribution correcte ; toute preuve manquante reste explicitement NOT TESTED. Ne pas augmenter les limites. Avis indépendant et acceptation publique encore requis.

Le suivi Codex récurrent suivre-swarm-supervision-fiable est ACTIVE, toutes les10 minutes, attaché à ce chat. La supervision n'avait pas été programmée au lancement initial : contrairement à la demande sans intervention humaine, la fin du tour avait laissé le blocage sans reprise. Ce suivi n'est pas présenté comme une autonomie déjà démontrée du produit.
