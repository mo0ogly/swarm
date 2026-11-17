# Témoin SIGTERM du processus primaire

`trace_signal_witness.run_signal_witness(out, admission, strace, python)` observe
un worker témoin, un worker qui se signale lui-même SIGTERM, puis une reprise
dans un autre dossier. Il réutilise le publisher, le décodeur hex et l'exécuteur
natifs du noyau de transport. L'ancien témoin0/7/0 reste distinct et inchangé.

Le caller doit conserver stockage qualifié/FD, pins des sources/helpers/outils,
umask077, leaf neuf vide0700 et garde20Gio hôte/sorties. Les fichiers doivent être
0600. L'environnement factice est exact et sans héritage ; l'allocation de chaîne
vient des argv/env connus, jamais d'une politique arbitraire de build.

PID primaire, exec/argv/env, terminal de signal et résultat numérique du traceur
sont vérifiés séparément. Le manuel strace installé décrit une terminaison du
traceur par le même signal ; le témoin doit observer la correspondance réelle
SIGTERM/returncode négatif, pas la supposer. Seule la ValueError numérique
attendue sans autre cause est acceptée ; toute issue supplémentaire demeure
bloquante et liée au reçu natif. Les preuves du signal sont comparées avant/après
la reprise fraîche. Un refus conserve les fichiers partiels et causes d'origine.

Portée uniquement Linux et ces trois workers connus sans forks. Aucun autre
processus n'est signalé. Aucun Cargo, provider, Swarm installé ou test Rust n'est
appelé. Le résultat conserve complete_build_qualification=false et
behavioral_admission=false. Autres signaux, défaut propre du traceur, capture
corrompue, fichiers consommés et arbres arbitraires restent non qualifiés.

La conservation des résultats connus pendant une erreur/interruption tardive
utilise désormais [le contrat commun](TRACE-OUTCOME-CUSTODY.md). Le code natif
est copié avant tout diagnostic risqué ; l'issue primaire n'est déclarée observée
qu'après comparaison de la trace. Les nouveaux témoins sont synthétiques et ne
rejouent pas les observations SIGTERM historiques.
