# Recette des Actions de confiance

La recette porte sur un candidat identifié, des entrées figées et des racines isolées. Chaque reçu distingue tests Go, parcours CLI/API, contrôles navigateur, revue indépendante et limites. Les unités ne sont pas additionnées en un score. Les dossiers historiques restent conservés sous `docs/plans/actions-confiance/verification-globale-20261016` ; le dossier du candidat corrigé relie le binaire, les résultats et la décision de revue.

## Reproduire

`python3 tests/actions_confiance_campaign.py --binary BINAIRE --candidate-receipt RECU_COMPILATION --build-inputs EMPREINTES_COMPILATION --evidence-dir NOUVEAU_DOSSIER --timeout-seconds 180` exécute neuf étapes : connecteurs, signatures/témoin/archives, laboratoire, courtier historique, courtier sous politique de données, substitution du helper, pilote, Processus et matrice de contrôles.

Le reçu de compilation contient `binary_sha256`. Les empreintes sont un objet chemin/digest ou une liste d’objets `path`/`sha256`. `--prepare-only` vérifie sans exécuter. `--resume` exige mêmes entrées et binaire, résultats intacts et sortie0 observée ; aucun échec n’est converti en succès. Le délai est un watchdog de recette, pas une politique produit.

`tests/actions_confiance_effect_proof.py` vérifie aussi le refus d’engager sur chaîne altérée, les contextes de politique/intention/résultat, les instantanés de réconciliation, les deux connecteurs et les preuves historiques. La recette navigateur `tests/actions_confiance_evidence_ui.cjs BINAIRE DOSSIER --effect-proof` exerce un montant dépassant 2^53, sa signature et sa vérification exacte.

## Portée et décision

L’utilisateur a autorisé une revue Codex en contexte séparé et reporté les essais eBPF natifs positifs. Le report ne qualifie pas R20 : collecte réelle, attribution, exclusion d’une seconde fixture possédée, pertes/reprise/nettoyage et surcoût restent à vérifier dans un environnement explicitement autorisé. Le lancement proposé est conservé dans `kernel-positive-launch.md` sous le dossier de qualification. Aucun privilège Linux ni nouvelle tentative n’est implicite.

Le RETEX clôt le périmètre convenu avec ce report explicite. Simulation locale, contrôle d’intégrité, revue de code et acceptation métier restent distincts. Aucun compte tiers, paiement ou fournisseur général n’est qualifié. Les limites mêmeUID, swap/traçage privilégié et temps tiers restent décrites dans T4/T6/T7.
