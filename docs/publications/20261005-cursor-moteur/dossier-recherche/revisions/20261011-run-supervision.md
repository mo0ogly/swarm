# 11 octobre 2026 — retirer les coupures de durée de la production

## Besoin repris et fait discriminant

L’utilisateur demande que le superviseur IA décide de la conduite et que le
moteur cesse d’arrêter automatiquement le travail au seul dépassement d’un délai.
Il demande la documentation d’architecture en complément de la correction.
La méthode `replan` reprend ce besoin, l’état public de T2 et les originaux ;
aucune tentative n’est remboursée et aucun budget n’est augmenté.

Sur `w-ca937edf4e0abd85f8d3d67b`, l’agent `auto-f2abbcfe6bed66fd1a5e`,
tentative `a-a349f9134d355713576b7166`, a commencé à 08:06:25.430999347 UTC,
puis a été interrompu à 08:36:27.473812792 UTC : code 143, `stop_kind: delai`.
Le compteur observé est 78 appels / 78 résultats, plafond 80. Le dernier outil
exécutait les tests de la base acceptée avec ses délais explicites. Le délai
total de 1 800 secondes a coupé le producteur sans décision sémantique du responsable.
Ce fait n’établit ni succès des tests ni blocage du code testé.

Le travail était sauvegardé dans le stash
`a73f26244e8e0e59877fb30e8c88a8628b7ad33d`, basé sur
`e32388fda21e7c225c3b2416d230d6f5450230cd`. Une copie opérateur indépendante
permet l’examen ; copie native, stash et références d’origine sont conservés.
La remise de cette tentative déclare `partial` ; T2 n’est pas accepté.

## Diagnostic, options et correction

Le défaut confirmé se trouve dans `agents_process.go` et
`agent_terminal_process.go` : un tick transforme la durée en signal d’arrêt.
`loop_guard.go` coupe aussi sur la durée d’un outil observable. Un autre défaut
empêchait le responsable d’agir pendant la production :
`managedIntegrationPending` considérait une copie en production comme une
intégration dont il fallait attendre le verdict.

| Option | Effet | Décision |
| --- | --- | --- |
| Augmenter le délai | Repousse le prochain arrêt arbitraire | Écartée |
| Mode observation global | Retire aussi des plafonds indépendants de dépense | Écartée pour cette correction |
| Observation attribuée puis décision IA explicite | Sépare conduite, mesures et contrôle des droits | Retenue |

La correction introduit les événements `run_observation` et les opérations
strictes `continue-run` / `stop-run`. Le responsable reçoit la tentative et ses
compteurs dans le contexte réellement produit par le moteur. L’arrêt décideur
est distinct d’une fin de processus, d’une remise et de l’acceptation.
Le moteur conserve la transaction versionnée, le bail, la génération, les budgets
et la vérification du processus courant. Le verdict d’intégration après fin du
producteur reste une frontière protégée.

Le [contrat d’architecture FR](../../../../architecture/RUN-SUPERVISION.md) et
[son équivalent EN](../../../../en/RUN-SUPERVISION.md) donnent les responsabilités,
les cas d’indisponibilité, le protocole et deux schémas Mermaid. Les renvois sont
ajoutés au protocole IA/moteur, à la reprise autorécursive et au chapitre moniteur.
Les libellés Admin et pré-lancement FR/EN décrivent des seuils d’observation,
sans ajouter de réglage caché ni de nouveau délai de remplacement.

Ce changement complète les exigences de rôles et de retours du
[texte Cursor de janvier](https://cursor.com/blog/scaling-agents) et du
[texte Cursor de février](https://cursor.com/blog/self-driving-codebases).
Il conserve leurs différences : le juge de cycle de janvier ne décrit pas
l’architecture finale de février. Il ne constitue pas une conformité globale
observée à ces publications.

## Vérification et limites

| Contrôle | Résultat au stade de la correction | Portée |
| --- | --- | --- |
| Reproduction par overlay des trois fichiers originaux | FAIL attendu : trois tests détectent le comportement ancien | Aucune modification de la base réelle |
| Tests ciblés processus, terminal, propriétaire et protocole | PASS, 14,080 s | Fournisseurs doubles et stockage temporaire |
| Race sur supervision et décisions | PASS, 19,848 s | Concurrence locale des états partagés |
| Contrat de méthodes et contextes réellement construits | PASS | 12 skills ; aucun plafond de contexte relevé |
| Catalogue i18n | PASS | FR/EN, parité et repli |
| Suite Go complète du dépôt | PASS : 1 118 cibles, 19 groupes disjoints, tous exit 0 | Contrôles optionnels sautés explicitement ; `go test ./...` PASS également (504,591 s) |
| Race élargie, vet, recette Admin | PASS : race 78,101 s ; `go vet ./...` ; Admin FR/EN, deux thèmes | Copies temporaires et doubles fournisseur |
| Installation et observation du serveur courant | VÉRIFIÉ : build 09:14:45 UTC, empreinte identique du processus servi | Aucun producteur, revue ni bail actif dans les trois travaux avant redémarrage |
| Décision autonome IA réelle sur ces observations | NON TESTÉE | Les doubles ne prouvent pas ce comportement |
| Acceptation native T2 | NON ACQUISE | Original partiel et reprise conservés |

Les tests de décision utilisent le parcours CLI public, un vrai processus possédé,
les périmètres racine/enfant, la réouverture et le rejeu. Ils vérifient les refus
hors périmètre, sans entrée fournie et après fin du processus, puis la conservation
de la tentative et des compteurs. Le seuil d’un outil ne masque pas son appel en
cours. Ces preuves ne démontrent pas deux superviseurs IA réels simultanés : la
mission observée n’a toujours qu’un périmètre racine ; T3 conserve ce travail.

Les sources, commandes, sorties originales, codes de sortie, empreintes et état
courant sont enregistrés dans
[l’index de vérification](evidence/20261011-run-supervision/verification-index.json).
Les résultats ultérieurs complètent cette note, sans réécrire les anciens reçus.

## Replan des contrôles et de la remise

La première suite canonique complète a échoué uniquement sur
`TestReplayGLMRetryEventShapes`, qui exigeait encore une coupure d’outil. La
régression vérifie désormais un outil réellement en attente, son seuil visible
et ses compteurs inchangés, sans arrêt automatique. Le parcours Admin échouait
parce que la recette ouvrait le premier historique, ajouté avant le panneau
voulu ; elle cible maintenant l’accordéon de l’effectif. Les sorties originales
restent dans l’index, séparées des résultats corrigés.

La remise de T2 rencontrait un autre défaut confirmé : une réparation conservée
de l’ancienne tentative empêchait la soumission de la tentative courante arrêtée.
La garde est maintenant liée à l’agent et à la tentative ; l’événement durable
conserve l’ancienne réparation, la revue et la coordination. Le test public
vérifie deux tentatives, les références Git intactes, les reçus intacts, une
nouvelle revue, le rejeu sans appel supplémentaire et le refus d’une nouvelle
soumission implicite du même résultat. Les plafonds restent inchangés.

Les tâches T4 et T5 ont reçu le complément `run-supervision-20261011.patch`
par les opérations publiques, révisions 110 → 111 → 112, avec leurs consignes
antérieures conservées. La reconstruction intégrale a révélé trois en-têtes de
nouveaux fichiers absents du packaging précédent. L’original est conservé ;
`agent-engine-protocol-resumption-20261011-git.patch` corrige uniquement ce
packaging. Le nouveau complément a été appliqué et inversé dans un dépôt isolé,
puis ses fichiers comparés octet par octet avec les sources locales.

La copie opérateur de T2 corrige également le jugement de fin : l’identité Git
seule ne suffit pas ; il réutilise les contrôles de clôture, accepte seulement
les preuves fraîches des exigences et refuse un processus actif. Le cycle
courant est reconstruit après réouverture. Cette réparation reste distincte de
la production autonome et doit obtenir son propre verdict indépendant.

## Précontrôle impayé : découper les preuves puis reprendre le résultat

La première réparation de T2 a produit le résultat
`60bdc8ad5b395b5abc972eb701ff244faed45e69`, mais le manifeste source unique
formait une pièce de diff indivisible trop grande. Le refus est survenu avant
tout appel de revue : révision 114, aucun avis T2, 11 appels consommés sur 30.
Son candidat contrôlé `1168413b5be72556464fde60efd5fd3d73b79a98`, ses reçus
et ses références restent conservés. La cause n’était pas un défaut démontré
du juge, ni une raison d’augmenter le contexte ou de consommer une production.

Deux frontières sont corrigées : le harness écrit quatre pièces JSON contenant
toutes les entrées, liées par SHA-256 au manifeste, et le parcours public
`revise-recovered-result` reconnaît le précontrôle impayé vérifié d’un résultat
réparé interrompu. Il exige le checkpoint, les reçus, l’attribution et la fin
réelle du processus. Un échec arbitraire du contrôleur n’autorise pas ce parcours.
La correction remplace explicitement le résultat conservé, garde l’original,
puis exige de nouveaux contrôles et une revue dans le budget existant.

La suite complète du moteur corrigé couvre 1 120 cibles (19 groupes, tous exit 0) ;
les tests ciblés passent en 36,669 s, race en 93,513 s, vet exit 0. Les refus
de reçus/checkpoints altérés et le rejeu sans deuxième appel sont exercés.
Le candidat T2 conserve 1 049 entrées dans quatre pièces ; sa suite complète
couvre 1 096 cibles, tous les groupes exit 0. Les skips restent explicites.
Ces nouveaux reçus complètent les précédents, sans les écraser.

L’installation est vérifiée par empreinte du binaire et du processus servi
(build 09:35:34 UTC, SHA-256
`3b523e12b9d71b59f8fb424ea13f50b4c6611fc27eedad6c9dd21960736de2d3`).
La révision publique de T2 a passé les contrôles et démarré la revue indépendante
`review-4b2233e89effd7e81afc1b5e` sur le candidat
`8213f83900ef26d53a993e9c5d9e89eb06d55d36`. Le statut observé au démarrage
est `running`, pas une acceptation. Les preuves de packaging et d’intervention
opérateur ne démontrent pas l’autonomie IA ; T3/T7 conservent cette vérification.

La vérification ordinaire finale `go test ./... -count=1 -timeout 12m`
réussit aussi : exit 0, paquet moteur 476,732 s. Le contrat de méthodes et les
quatre contextes réels passent après cette correction. La documentation
FR/EN de reprise précise le précontrôle sans appel et les seuils de production.

Les consignes complémentaires T3/T4/T5/T7 sont enregistrées publiquement aux
révisions 119 → 120 → 121 → 122 → 123 ; critères, dépendances et instructions
antérieures sont conservés. Ce reçu établit le routage dans le champ `next`,
pas encore la lecture ni l’action d’un worker. T3 doit raccorder judge/renew
au schéma IA ; T5 doit initialiser les anciens plans sans réexécuter enable,
exposer les politiques et vérifier l’UX ; T7 doit observer les rôles réels.
Les patches de correction et de documentation ont des empreintes propres ;
les compléments déjà transmis ne sont pas réécrits.
