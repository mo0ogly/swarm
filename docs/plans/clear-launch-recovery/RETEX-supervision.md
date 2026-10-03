# RETEX de supervision — 3 octobre 2026

Mission : w-115c11e8f802a4f98c3def32. Candidat initial : 1d9570bd4ef617130c6be96b7ec88844fdbcd00e.

## Faits observés avant T1

- Préparation Codex hors contrat : aucune adoption ; préparation Claude ensuite adoptée.
- Conversion et lancement cockpit distincts : intervention du superviseur nécessaire.
- Premier T0 : 20 outils, interruption après écriture du rapport ; zéro erreur structurée.
- Soumission manuelle du rapport crée un enregistrement de tentative terminé supplémentaire sans nouvelle exécution. Le vérificateur exige néanmoins un agent producteur terminé normalement ; le rapport soumis reste sans revue.
- Décision du responsable concurrente à la soumission : retour périmé refusé ; pause puis reprise explicites nécessaires.
- Reprise ciblée T0 : deux fichiers existants relus, six outils, processus terminé normalement et avis indépendant obtenu. Pas de réinventaire.
- L’identité moteur annonce départ 3/2 : deux vrais agents, trois enregistrements de tentative dont une soumission manuelle. Ne pas confondre cet enregistrement avec une troisième exécution. Ce mélange dans le contexte du worker nécessite une correction ciblée.
- L’avis indépendant est documentaire et indique ne pas pouvoir vérifier les citations contre le dépôt ; corroboration des structures sources réalisée séparément par le superviseur.

## Conséquences pour QW2 et QW4

Présenter la soumission interrompue comme une remise à vérifier avec action de reprise réellement autorisée ; distinguer tentatives de processus et enregistrements de remise dans le compteur/context. Conserver les preuves et historiques, sans auto-acceptation ni augmentation silencieuse de limites. Ces défauts sont constatés, pas encore corrigés.

## Validation et départ suivant

T0 accepté après gate documentaire 6/6 et avis indépendant favorable ; aucune dérogation. Le responsable a aussi rencontré un timeout à 90 secondes (67 événements, aucun résultat final), diagnostic conservé. Reprise de planification après stabilisation des preuves. T1 a démarré automatiquement à 08:27 UTC le 3 octobre : résumé avant lancement web/CLI. État constaté : 1/6 validé, T1 actif, quatre tâches en attente. La mission complète n’est pas encore recettée.

## Première tentative T1 : diagnostic à la place du correctif

T1 s’est terminée en 18 appels, sans modification applicative ; coût rapporté 0,76 USD. L’agent a interprété « deux corrections » comme une limitation empêchant un correctif cohérent de quatre fichiers et déclaré les critères PARTIAL/FAIL/NOT TESTED. Avis indépendant changes_requested, sans acceptation. Les recettes Puppeteer installées n’ont pas été utilisées : absence de connecteur navigateur confondue avec impossibilité de recette par shell.

Reprise autorisée dans le second départ existant : consigne explicite d’implémentation, quatre fichiers liés dans le périmètre, deux corrections = cycles, tests browser existants à réutiliser sur racine isolée. Distinction clarifiée : palier demandé / modèle résolu par configuration / modèle réellement observé. Aucune hausse de plafond.

Rectification du bilan T0 : compteur final agent show = 7 outils pour la reprise, pas 6 annoncé juste avant la fin. Le dernier Edit est inclus. Les instantanés d’activité ne doivent pas être pris pour un total final.

## T1 : changement d’approche après le second échec

La seconde tentative a atteint la limite de 25 outils, avec une erreur structurée et aucune modification applicative. Nouvelle reprise identique exclue. Le superviseur Codex a implémenté le lot directement dans le dépôt ; cette intervention n’est pas une preuve d’autonomie du Swarm.

Ajouts : identité de lancement commune et par départ, respect du profil propre à la tâche, séparation niveau demandé / modèle configuré / modèle observé inconnu, profil projet et skills, rendus CLI et web. La recette réelle a détecté des textes moteur non traduits dans l’aperçu anglais ; ils ont été corrigés avant livraison.

Contrôles : suite Go complète PASS en 476,065 s sur le premier candidat ; tests ciblés après les corrections de traduction PASS ; npm test et go vet PASS. Recette sur racine temporaire via cockpit, aucun lancement de fournisseur : français/anglais, clair/sombre, sélection native, Échap et retour de focus, modèle configuré sonnet et programme sans modèle résolu connu. Les captures sont sous test-results/clear-launch-recovery/. Les journaux console capturés du navigateur de recette ne contiennent pas d’erreur/avertissement. Cela ne prouve pas l’absence de toute erreur réseau : pas de collecteur réseau exhaustif dans cette recette manuelle.

Le rapport de la première tentative est conservé. Le correctif du superviseur a son propre rapport T1-supervisor.md. T1 n’est pas encore accepté par le moteur et les autres lots restent à effectuer.

## Remise ciblée T1 et incident de délai

Une tentative supplémentaire T1 (2 → 3, outils 25 conservés) a été autorisée dans le cockpit avec motif : précondition changée par le correctif réel et ses preuves. Agent f7bf328d-d2ba-4d92-84e5-6bee6bbd9447, tentative a-62b5c700c364e7236430a9ca : fin normale, 8 outils, empreintes 6/6 identiques, contrôles ciblés et npm test PASS. Le premier rapport complet est aussi archivé dans T1-first-attempt-report.md. La recette visuelle est attribuée au superviseur, pas à ce worker.

La revue indépendante de cette remise a atteint le délai de 90 secondes : 71 événements thinking_tokens, aucun résultat final. Ces événements ne sont pas 71 appels d’outils ni 71 appels IA ; la tentative de revue consomme un appel dans le compteur moteur. Configuration publique du délai à 300 secondes avec motif, puis reprise explicite du même candidat. Plafond de 40 appels et historique conservés. Verdict de cette reprise encore attendu au moment de cette note.

La CLI publique isolée en anglais confirme aussi auto → sonnet, observation inconnue avant exécution, aucune sélection de profil/skill inventée, et traduit périmètre/limites/budget/reprises. Aucune invocation de fournisseur pendant ce contrôle.

## Revue refusée : livraison erronée et reprise du même résultat

La revue à 300 secondes s’est terminée avec corrections demandées : le livrable déclaré restait le diagnostic de la première tentative. Le nouveau rapport renvoyait vers des fichiers que la revue sans outils ne pouvait pas ouvrir. Cause : remise documentaire incorrecte, pas absence du correctif T1.

Ancien diagnostic conservé dans T1-first-diagnostic.md ; T1-handoff.md contient maintenant le correctif, les extraits, les tests et les empreintes, avec attribution des vérifications au superviseur. La CLI publique retry-review a repris le même résultat après modification réelle des preuves, sans nouveau worker et sans remboursement d’appels. Correctif moteur : seuls des contenus lisibles précédemment liés à une revue refusée, réellement modifiés, permettent cette reprise explicite ; suppression, absence, refus inchangé et budget épuisé ne l’autorisent pas. La tâche revient à vérifier, jamais à acceptée.

Les contrôles Go et JavaScript sont maintenant autorisés par la politique humaine T1 via preview/apply publics. Le moteur enregistre leurs reçus et les empreintes des quatre captures sous docs/screenshots/clear-launch-recovery pour transmission au réviseur. Cette politique ne valide pas automatiquement le critère qualitatif.

Autre défaut confirmé : un lancement manuel T1 copiait sa consigne spécifique dans le profil commun des tâches suivantes. Le profil commun a été corrigé via la CLI publique avec une consigne neutre. Le correctif moteur de cette propagation reste à réaliser dans le lot reprise/coordination ; aucune autonomie réussie n’est revendiquée.

## La revue indépendante détecte un champ obligatoire absent

Le cinquième appel de revue a passé les critères inconnus et recette documentée, et refusé le résumé complet : l’objectif n’était pas transporté ni rendu dans le récapitulatif. Correctif borné : objective provient de Work.Objective, s’affiche dans le CLI et le cockpit, est signé avec l’aperçu et couvert par un test moteur et un test de rendu. La nouvelle recette FR/EN × etat/sombre et les quatre captures v2 montrent le champ ; Échap restitue le focus à mission-primary. Le contenu saisi par l’utilisateur n’est pas traduit automatiquement. Ancien candidat et ancien handoff archivés. Reprise publique du même résultat et reçus réexécutés : trois tentatives producteur conservées, appels de revue dépensés conservés.

Suite Go sur le correctif de reprise PASS 420,734 s ; nouveau test ciblé objectif/reprise PASS, npm test et go vet PASS. La suite Go complète sur le candidat incluant objective est lancée séparément, résultat encore attendu.

Le correctif moteur de propagation des consignes est maintenant implémenté : un lancement opérateur conserve l’instruction commune existante (ou vide), tout en enregistrant son instruction locale dans le profil de la tâche. Deux tests comportementaux ciblés PASS : départ suivant sans fuite de consigne, et conservation d’une consigne commune explicite. Intervention du superviseur, pas résultat autonome de T4.

## Héritage effectif et passage à T2 — 3 octobre

Le moteur assemble désormais la consigne commune et la consigne locale dans le prompt réellement enregistré pour l’agent ; le dispatcher ne duplique plus la consigne commune. Les tests vérifient présence des deux, présence unique de la consigne commune, et absence de fuite de T1 vers T2. Tests ciblés PASS 0,354 s ; suite complète de cette dernière version encore en cours.

Septième revue T1 favorable, gate delivery 6/6, puis acceptation après revue enregistrée par le superviseur dans l’UX. Lecture actualisée : révision 61, 2/6 tâches validées, T2 démarrée automatiquement, un agent actif. Le bandeau de planification indique encore à tort que tous les prochains départs attendent : défaut UX observé, à traiter dans T2. Aucun nouveau producteur T1 ni remise à zéro des appels.

## Correction du bandeau dans le moteur partagé

Défaut reproduit : le bandeau demandait une décision de planification malgré T2 active. Cause : la condition Planning.Failure écrasait missionUnderstanding et missionGuidance sans tenir compte des tâches déjà réservées, prêtes, à configurer, à examiner ou à reprendre.

Correction : l'échec de planification domine la prochaine action uniquement quand aucune action existante ne peut continuer. L'état des tâches et les autorisations restent inchangés ; le diagnostic Planning.Failure reste enregistré et visible dans la section des décisions. Aucune protection de lancement, gate ou budget modifiée. La phrase affirmant que tous les prochains départs attendent est supprimée. Moteur commun au CLI et au cockpit.

Vérification ciblée : tests de priorité running/review/intervention/configure, réservation réelle du moteur sur racine temporaire sans fournisseur lancé, conservation du diagnostic et de l'état de tentative PASS (0,141 s). npm test et go vet PASS. Catalogue anglais complété pour le message dynamique de blocage ; tests de traduction Go réels PASS (0,051 s), CLI anglais et cockpit FR/EN en etat/sombre observés. Focus clavier visible. Captures guidance-* dans docs/screenshots/clear-launch-recovery/. Suite Go complète du correctif en cours.

État après déploiement : T2 a terminé sans démontrer tous ses critères (rapport partiel, pas de nouvelle recette navigateur par le worker). La modification de fichiers liés à T1 rend ses preuves anciennes périmées ; 1/6 validée et deux tâches signalées. Ce recul du compteur est la fraîcheur des preuves, pas une suppression de résultats. La revalidation reste nécessaire avant de relancer les dépendances. Intervention du superviseur clairement distinguée de l'autonomie de Swarm.

Vérification finale du correctif : `go test ./... -timeout 40m` terminé avec code 0. Sortie : ok  	swarm.local/companion	442.451s. Le test supplémentaire de réservation réelle (ajouté pendant cette suite, sans modification du code de production) a été exécuté séparément avec succès. Build canonique `sh build.sh` installé et serveur confirmé dans le navigateur ; commit de base 1d9570bd4ef6, modifications locales non commitées.

## Revalidation sans nouvelle production

Trou confirmé : accepted avec preuves périmées ne pouvait pas reprendre une revue ; l'UX proposait de rouvrir la production, incompatible avec trois tentatives T1 déjà dépensées. Correction du moteur retry-review et bouton principal : reprise explicite du résultat existant uniquement après modification réelle de preuves liées, lisibles, même tentative terminée et contrat inchangé. Ancien avis archivé, reçus conservés sur disque, anciennes mutations gardées ; nouvelle gate/acceptation requises. Le budget du vérificateur reste appliqué. Tests ciblés passent : accepté inchangé refusé, fichier absent refusé, budget épuisé refusé, même tentative et dépense conservées. Aucun système de production gérée n'est modifié.

Revue 8 refuse EN clair en affirmant voir un fond sombre. Inspection humaine assistée de l'image conservée constate des fonds blancs/gris ; le désaccord n'est pas converti en acceptation. Recette EN clair rejouée sur le binaire courant : data-theme=etat, lang=en, fond de dialogue rgb(255,255,255), page rgb(246,246,246), inconnus visibles, Échap et focus contrôlés. Capture refaite et ancienne conservée. Revue 9 explicitement reprise après ces nouvelles preuves ; aucun départ T1 supplémentaire.

Suite finale du correctif de revalidation : go test ./... -timeout 40m PASS 436,151 s, code 0 ; npm test, go vet et git diff --check PASS. Recette réelle via le bouton du cockpit, puis trois reçus de contrôle nouveaux PASS ; avis indépendant encore attendu, pas d’acceptation déclarée.

### Refus 9 et nouvelle recette
La revue accepte les critères 1 et 2 mais conserve unknown pour la recette : elle cite la limite historique du worker et celle du contrôle Python comme si elles annulaient la recette ultérieure du superviseur. Nouvelle recette CUA réellement rejouée dans quatre combinaisons ; addendum daté, sans supprimer la limite historique. Attention : auto vs sonnet est demande vs résolution, pas une divergence de modèle exécuté. Ne pas annoncer ce dernier cas comme testé.

### Refus 10 : vraie lacune, correction au moteur
La revue confirme champs, inconnus et quatre parcours navigateur ; elle refuse le cas modèle demandé différent du modèle rapporté, réellement non couvert. Le rapport précédent assimilait auto vs sonnet à ce cas : confusion entre niveau/résolution et observation, à ne plus reproduire. Correction attribuée au superviseur : Agent.reported_model structuré, source/date, aucune inférence de la route, rendu distinct et inconnus conservés. Test de processus enfant réel sous fournisseur de protocole et recette CUA dans quatre modes ; ce protocole ne prouve rien sur un modèle physique Claude. Les anciennes déclarations de réussite ne remplacent pas cette preuve. Aucun nouveau départ T1, aucune hausse de budget ; dix appels de revue ont été consommés à ce stade. Le produit n’a donc pas réalisé ce lot sans intervention du superviseur.

### Reprise après remplacement des contrôles
La politique de validation archive une revue refusée et met la revue courante à nil. Le chemin de reprise précédent ne consultait que la revue courante : bouton perdu, bien que le résultat terminé et les preuves corrigées existent. Correction : seule la dernière revue archivée refusée, liée au même contrat et à la même tentative terminée, peut être reprise explicitement si une preuve déjà liée a réellement changé et reste lisible. Vue publique dédiée archived_review ; jamais déclarée courante/valide. Historique non dupliqué, compteurs conservés. Tests Go ciblés PASS 0,506 s, npm PASS ; suite Go complète en cours.

### Reprise T2 observée
T1 accepté après revue 11 : 2/6 résultats valides. T2 deuxième tentative effectivement démarrée depuis le cockpit : agent 537b1d6b-7c30-4e7f-878a-3c832554c719, tentative a-4fea4cb374a0cf60b5cc2680. Le nouvel enregistrement reported_model observe la déclaration réelle system/init `claude-sonnet-5` à 2026-10-03T14:37:17.782284905Z ; la demande reste la route `sonnet`. Ce sont deux identifiants distincts, pas la preuve d’un changement de famille physique ni une garantie de version 5.5. La première tentative interrompue à 25 appels reste conservée et sans modèle rétro-inféré.

Suite complète du correctif de reprise archivée : Go PASS 535,190 s, code 0 ; vet PASS sans diagnostic ; race ciblée PASS 3,114 s ; npm PASS ; check.py PASS (9 skills) ; diff --check PASS. Les deux suites complètes du présent travail ont donc passé (modèle rapporté puis récupération archivée). Le résultat de T2 reste à examiner, aucune clôture de mission annoncée.

## 3 octobre — correction de projection des refus et fiabilité des preuves

- Défaut confirmé et corrigé dans `result_presentation.go` : les refus indépendants ordinaires étaient masqués par un rapport « à soumettre », bien que soumis. La projection actuelle conserve processus, rapport, avis et décision distincts, liés au bon producteur et à la bonne tentative. Tests ciblés PASS ; npm et parité i18n PASS.
- Revues 12/13 : captures mal attribuées puis captures contenant le texte de la revue précédente, considéré à raison comme contenu non fiable. Une boucle de preuve peut naître du dossier de revue lui-même. Correction : dossier courant concis, historiques conservés à part, nouvelles captures du cas réel organisation absente (FR/EN, clair/sombre, bouton réel, Entrée/Échap/focus). Plafond 2/2 réel conservé et observé. Aucun nouveau producteur.
- SQLite_BUSY sur une application publique de politique : réessai de la même opération après refus, succès. Aucune correction générale de concurrence SQLite n'est démontrée.
- Serveur lancé par nohup interrompu au retour de la commande : remise en service dans une session persistante, disponibilité vérifiée dans le navigateur.
- Suite complète : délai par défaut de 600 s atteint dans `TestManagedFragmentIntegrationPublishesOnlyFinalVerdict` préexistant. Relance `go test ./... -timeout 20m` ; en attente du résultat. Ce délai de test n'est pas une limite d'agent.
- Revue 14 demandée après changement réel des preuves. Deux tentatives producteur, 37 appels d'outils observés, pas de remboursement ni de plafond relevé. Le coût des revues reste distinct des outils du producteur.

### Clôture T2 et départ T3 — 3 octobre, 17:17

T2 finalement acceptée normalement à la révision 116, gate delivery révision 115, avis indépendant 17 favorable sur les trois critères. Quatre contrôles code 0, dont relectures directes CLI des deux environnements par le contrôleur ; absence de doublure et origine ne reposent plus exclusivement sur un relevé JSON écrit par le superviseur. Aucune dérogation, troisième tentative ou hausse de plafond. T3 départ automatique révision 117, consigne bornée aux points d'entrée connus et mesure des inconnues conservée. Le cockpit indique 3/6 et un vrai agent actif.

La suite globale `go test ./... -timeout 20m` a également atteint son délai, cette fois dans `TestManagedFragmentTokenStoreRuntimeAndCapabilityDrift/fail` (le test d'intégration précédent a donc été franchi). Il n'est pas établi qu'une assertion échoue ; aucune réussite globale ne peut être déclarée. `go vet ./...` termine code 0. Les tests ciblés de projection/reprise, npm et la parité i18n restent réussis. Une exécution globale avec durée adaptée et candidat final figé reste nécessaire ; ne pas relancer immédiatement toute la suite alors que T3 modifie encore ses sources.

Leçon de preuve : joindre les pixels de la modale ne montre pas nécessairement l'acteur affiché derrière ; joindre un JSON de recette ne montre pas nécessairement sa provenance. Préparer dès le premier dossier les bandeaux lisibles, l'action réelle essayée, les deux thèmes/langues, puis les commandes produit rejouées par le contrôleur avec sortie conservée. Les inconnues d'un avis indépendant ne doivent pas être maquillées en PASS.

## T3 — interruption et complément ciblé, 3 octobre

La première tentative `a-cacc533574c699dddab19104` s’est interrompue après
25 appels d’outils : 14 lectures puis 11 éditions. Elle a commencé les compteurs
moteur mais n’a livré ni interface ni tests ni rapport. La limite est conservée.
Le superviseur complète le même périmètre ; la seconde tentative autorisée doit
vérifier et remettre ces changements, sans refaire un inventaire global.

Les catégories sont conservatrices : Read/Glob/Grep sont des lectures,
Write/Edit/MultiEdit/file_change des écritures ; les commandes mixtes restent
non classées. Une répétition est une signature nom+entrée déjà vue sous un
nouvel identifiant ; les retransmissions d’un même identifiant ne comptent pas.
Les erreurs cumulées restent visibles après un succès. Une visibilité perdue
conserve les observations mais marque la mesure partielle. Les anciennes
tentatives et les terminaux sans événements structurés restent inconnus.

La validation affichée est celle enregistrée pour la tâche, toutes tentatives
confondues : elle ne transforme pas une tentative interrompue en réussite.
Un départ enregistré n’est pas un appel LLM ; les appels internes du fournisseur
ne sont pas mesurés. Les jetons et coûts absents ne sont jamais présentés comme zéro.

Tests frontend/parité FR-EN et configuration : PASS. Tests moteur ciblés
`TestAttemptMetrics|TestAttemptLedger|TestLoopGuard` : PASS (0,440 s).
Race, build, vet, recette navigateur et suite complète : en cours à ce point.
Les preuves T1/T2 sont honnêtement périmées par les nouveaux fichiers communs ;
leur revalidation doit conserver leurs tentatives et budgets.

### Recette T3 et répartition observée

FR/EN × clair/sombre : rendus réels, captures conservées, Entrée/Échap et
retour du focus vérifiés. La recette a détecté un focus tombant sur le sélecteur
du graphe après actualisation et deux libellés anglais manquants : corrigés
et revérifiés, pas ignorés derrière les tests statiques. Le résumé global ne
présente plus 0/0 jetons quand aucun fournisseur ne fournit son usage.

Au relevé avant revalidation : exécutants 140 outils observés sur 8 tentatives
(27 T0 + 51 T1 + 37 T2 + 25 T3) ; coûts rapportés 2,03 USD, incomplets
pour 4 tentatives. Vérificateur : 17 appels enregistrés, 212029 jetons de
sortie rapportés et 4,16 USD sur 16 appels, 1 usage absent. Responsable :
5 activations, 0 outil observable, 18077 jetons sortie et 0,45 USD sur
3 usages rapportés. Ces sommes ne sont ni le coût total ni les appels internes
réels des modèles. Les catégories historiques ne sont pas réinventées.

Le poids des revues répétées est donc démontré dans ce relevé ; l'attribuer
uniquement aux lectures d'un exécutant serait incorrect. Les lacunes de preuves
et les changements de candidat expliquent certaines reprises ; les appels
non mesurés du fournisseur restent inconnus. Piste : regrouper les changements
des fichiers communs avant revalidation, et fournir un dossier cohérent avec
les contrôles réels dès la première remise. Ne pas résoudre cela en contournant
les gates ou en effaçant les refus.

Revalidation T1/T2 enregistrée par leur modale : révisions 120 et 121, mêmes
producteurs et tentatives, anciennes gates/avis conservés dans l'historique.
Un conflit de révision pendant les contrôles T1 a conservé le résultat sans
acceptation ; le conducteur doit reprendre sur la révision courante.

### Interruption externe du serveur pendant la revalidation

La session serveur `13613` s’est terminée avec code 143 (SIGTERM) ; le port
18792 ne répondait plus et le processus de revue n’était plus présent. La cause
du signal n’est pas établie : ne pas l'attribuer au modèle ou aux tests sans preuve.
Le moteur a marqué l'avis `error` au redémarrage, révision 124, sans accepter
ni relancer automatiquement. Le serveur est relancé avec terminal alloué,
et la reprise explicite T1 utilise le budget existant, appel interrompu conservé.
C'est un incident d'environnement distinct du correctif de mesure T3.

### Focus du graphe et contention des politiques

La revalidation T1 a jugé insuffisante la preuve clavier du cas de divergence
(des captures de modale et un relevé JSON). La recette actualisée a retrouvé
le focus DOM, mais montré un vrai masquage visuel par la règle de bordure d'alerte.
Le correctif web/graph.css donne au focus clavier une priorité explicite ; quatre
captures et le relevé des styles rendus (4 px, pointillés 4/2) sont liés aux contrôles.
Le refus reste conservé. Ce nouveau défaut n'est pas maquillé en simple demande
de document : il est corrigé et rendu dans les deux thèmes.

Deux applications CLI de politique ont échoué SQLITE_BUSY pendant l'activité
du serveur. Sans revue ni agent actif, serveur brièvement arrêté : les deux
politiques T1/T3 ont été appliquées par le CLI public, puis serveur relancé avec
terminal alloué. Cela démontre une contention, pas sa correction dans le produit.
À traiter dans la reprise ciblée T4 : erreur récupérable et sérialisation des
opérations ; conserver révisions et idempotence, ne pas refaire de validations
ni dépenses pour un conflit local. Aucun plafond relevé.

## 3 octobre — T3 acceptée et relais automatique vers T4

- La seconde tentative T3 `58bcf730-5ee7-429f-af70-c4d2496993ca` / `a-9c26df876eb8691a342f2e00` a terminé sans changer les sources applicatives. Les trois contrôles enregistrés par le moteur sont exécutés et passent (codes 0) ; revue indépendante `review-58bbb5db1d39f3f24170ec38`, favorable aux trois critères, terminée à 16:06:09 UTC.
- Gate delivery enregistrée à r147, puis acceptation normale via le cockpit à r148, sans dérogation. État public lu ensuite : 4/6 validées, T4 réellement démarrée par le conducteur, T5 attend. Le relais n'a pas nécessité un nouveau départ manuel.
- Le rapport de l'exécutant T3 reprend un constat ancien de T2 périmée. Ce constat a été résolu avant son départ : T1 et T2 ont été revalidées respectivement à r139 et r140, et sont fraîches à r148. Les rapports historiques restent conservés ; ce présent checkpoint précise l'état courant.
- Attribution : la première tentative T3 a commencé les modifications (11 éditions observées) ; le superviseur les a complétées, ajouté le rendu et exécuté les recettes. La seconde tentative a vérifié et rédigé les deux rapports. Ses compteurs finaux doivent être lus dans le bilan moteur, pas dans son chiffre de 11 annoncé avant la fin de rédaction.
- La sortie du test Go global lancé avec `-timeout40m` contient `ok swarm.local/companion 1483.907s` : le paquet termine avec succès. Le processus enveloppe de terminal a ensuite été signalé 143 ; on distingue cette terminaison de l'enveloppe du résultat Go effectivement enregistré. Les derniers changements de libellés CLI et focus ont des contrôles ciblés séparés ; ne pas attribuer à ce test une source modifiée après son démarrage.

## T4 — corriger avant de relancer

Premier agent interrompu à 25 outils (7 lectures, 3 écritures, 15 non classés). La commande de compilation avec pipeline masquait le code d’erreur ; compilation directe et correction Task/*Task par le superviseur. Le blocage SQLite de changement de politique a été reproduit dans un test concurrent isolé : opération absente de la réservation du rédacteur ; ajout ciblé, puis PASS avec détecteur de courses. L’aperçu T4 compare seulement les preuves liées et expose les inconnus, sans faire un inventaire global ou relever les limites. Suite globale T4 relancée après correction d’une faute de syntaxe du drapeau de test ; ne pas confondre cette erreur de commande avec un défaut produit.

## Checkpoint après revalidation T1–T3 et vérification T4

T1 revue 23 favorable et acceptation normale r165 ; T2 revue 24 favorable et acceptation r170 ; T3 revue 25 favorable et acceptation r175. Les mêmes producteurs terminés ont été conservés : aucune nouvelle production pour revalider les traductions partagées. Le CLI public confirme 4/6 acceptations fraîches à r175.

T4 seconde tentative `3500cb7c-3823-48cf-a454-295a032b309e` / `a-9b4502ef490c0a9b2227676b` : terminée, 12 outils (3 lectures, 2 écritures, 7 non classés), contre 25 lors de la première interruption. Les commandes ont passé ; la recette web/CLI a été effectuée par le superviseur IA Codex. Le handoff dit « superviseur humain » et environ 11 outils : ces deux approximations sont corrigées ici, sans réécriture de son rapport original. Le contrôle fournisseur indique claude-sonnet-5, pas un modèle 5.5 présumé.

Le superviseur a ajouté la preuve de reprise réelle T1/T2/T3 au rapport T4 au moment de son entrée en revue : cette mise à jour tardive de preuve peut périmer l’avis et impose une revalidation, même si le code n’a pas changé. C’est une intervention de supervision à compter, pas un défaut de l’exécutant ni une raison de masquer la fraîcheur. Les preuves doivent être figées avant remise ; le moteur doit conserver ce contrôle.

Le candidat final est identifié par `T5-candidate.json` : base 1d9570bd4ef617130c6be96b7ec88844fdbcd00e + empreintes de 41 fichiers de code/configuration/tests modifiés, candidate_id c333207a971f5929c95f78a86b6c169509928824da90d2797cf59a43472167b4. La recette finale n’annonce aucune livraison avant revue et acceptation.

## Suite globale du candidat T4 final

Commande corrigée `go test ./... -timeout 40m` : PASS, 491,140 s, code 0. Journaux et attribution dans T5-global-checks.md et T5-go-full.log. Les contrôles ne démontrent pas à eux seuls l’autonomie de la mission ; une grande partie du complément et de la recette vient du superviseur IA.

## Preuve de séquence T4

La revue T4 (appel 27) a jugé insuffisant le troisième critère : les quatre captures de modale ne prouvaient pas à elles seules le parcours complet. Le superviseur a donc relié les captures déjà effectuées interruption/consigne/reprise et ajouté les captures du refus actuel et de sa correction. La correction publique r188, après l’événement de refus r185 observé à r187, affichait exactement UN critère restant et les preuves inchangées repliées. Les JSON CLI avant/après sont conservés. Un troisième contrôle moteur lit en direct l’état par le CLI public, sans remplacer l’exécution par un JSON archivé ; pas de nouvel exécutant ni de hausse de limites. Le défaut de preuve relève du cadrage de recette, pas d’un échec démontré du code ni de l’exécutant de vérification. Éviter la prochaine fois une revue qui ne dispose que de captures statiques.

## Cause du dossier de revue incomplet et recette fraîche

Les control.inputs sont liés par empreinte mais ne sont pas automatiquement transmis comme texte au vérificateur ordinaire : seuls rapport + Markdown livrable déclaré + reçus moteur + images déclarées lui parviennent. Les compléments dans un autre fichier n’étaient donc pas accessibles, ce qui explique une partie des nouvelles demandes de preuve ; l’annexe a été attribuée et ajoutée au livrable transmis, original conservé. Les avis 28/29 ont encore demandé une exécution web corroborante, sans défaut de code. Un adaptateur de recette fraîche a ensuite été ajouté : le moteur émet un défi de 45 secondes, le pilote externe CUA réalise les actions réelles, le contrôle vérifie nonce/délai/DOM/fermeture/focus et échoue sans réponse fraîche. Les quatre contrôles T4 sont maintenant PASS code 0. Pas de nouvel exécutant, pas de hausse de limites ; c’est une recette supervisée, pas une preuve d’autonomie sans pilote externe.

Une application de politique a également rencontré SQLITE_BUSY pendant une écriture concurrente ; elle n’a pas été présentée comme appliquée. Nouvelle lecture/preview avec révision courante ensuite : application réussie. La réservation ajoutée corrige le cas BUSY_SNAPSHOT reproduit ; elle ne prouve pas l’élimination de toutes les contentions SQLite. Point de durcissement distinct à conserver au RETEX.


### Revue 30 : sortie exécutée absente du contexte du vérificateur

Les quatre contrôles ont réussi, y compris la réponse CUA fraîche demandée pendant le contrôle moteur. La revue a néanmoins refusé le critère 3 : elle recevait la commande, le code de sortie et l’empreinte, mais pas les observations produites. L’annexe du superviseur pouvait être lue comme un récit non corroboré. Le moteur est corrigé pour transmettre une sortie explicitement autorisée, bornée à 8 Kio, avec taille et troncature. Le partage reste désactivé par défaut et ne change ni les règles d’acceptation ni le budget. Le complément au livrable devient factuel et bref ; l’ancien texte reste conservé. Le prochain contrôle doit encore démontrer cette transmission réellement, avant une nouvelle revue.


### Même conflit confirmé sur une mutation ordinaire

Une correction via `task update` a échoué avec SQLITE_BUSY pendant que le conducteur web restait actif. Le test concurrent ajouté pour task.update échoue avant correction : un second écrivain entre pendant les gardes. La réservation anticipée dépendait d’une liste de types de mutations. Elle devient commune à toutes les mutations, y compris les futures, avant la première lecture ; la transaction annule cette réservation si une garde échoue. Le même test passe ensuite, et la correction CLI réelle est enregistrée en révision 209 avec le serveur actif. Cela traite l’élévation d’une transaction de lecture vers écriture, sans promettre de supprimer toute contention SQLite possible.


### Revue 33 : critère de parcours continu insuffisamment présenté

La sortie CUA fraîche a été transmise (3647 octets, non tronqués), mais la revue a encore considéré le critère 3 inconnu : l’aperçu n’exposait qu’un état et le rapport initial conservait une conclusion PARTIAL sans contexte temporel suffisamment clair. Le superviseur a donc mené et enregistré une nouvelle séquence complète dans la même mission : lecture du refus (événement r227, écran r228), correction du dossier et Next r230, reprise par le bouton web r231 ; la carte passe réellement de Bloquée à À vérifier. Les observations, trois captures et événements sont confrontés par un contrôle CLI live. L’ancien rapport du producteur est conservé séparément ; un complément attribué à Codex explique que PARTIAL décrivait l’observation initiale du producteur.

Les départs et contrôles ont été suspendus uniquement pendant l’écriture des preuves afin d’éviter de les modifier après leur gel, puis repris depuis le bouton de l’interface. Aucun nouveau producteur, plafond ou tentative ; pas de requalification d’un refus en défaut démontré. Les revues 26 à 33 et leurs coûts restent conservés, y compris celles occasionnées par les défauts de présentation et de transmission des preuves du superviseur.


Suite Go complète finale : PASS, 992,378 s et code 0, sur le candidat de 47 fichiers ee5a9259ba8369b60b948571c2e6f3d6409c25ebb1baf31cb9a550e1187ff3cd. Les quatre contrôles T4 du dossier de séquence ont code 0 ; le CLI transmet 1997 octets de chronologie réelle et le navigateur 3323 octets d’observation fraîche. Le vérificateur examine encore le résultat : les contrôles ne sont pas présentés comme une acceptation.


### Revue 34 : confusion entre indépendance du jugement et réexécution personnelle

Même la chronologie CLI exécutée (1997 octets), les trois étapes DOM et les captures ont été écartées au motif qu’une revue sans outils ne pouvait les vérifier personnellement. Ce motif bloque la revue sans désigner d’étape absente précise. Le moteur conserve les contrôles, les empreintes, les citations obligatoires et le verdict unknown pour une preuve réellement absente, mais explicite le mode evidence_assessment_without_tools : examiner les preuves et leur couverture n’est pas les rejouer. L’indépendance désigne la session et le jugement ; une vidéo ou une observation par le producteur ne devient pas une exigence implicite. Les limites temporelles du rapport initial restent conservées et les observations postérieures ont une attribution distincte. Un test de passage au fournisseur vérifie le mode effectivement transmis ; aucune réponse du fournisseur n’est forcée à PASS.

### Derniers défauts de recette, distincts des défauts moteur

Le défi navigateur a expiré pendant une reprise de contexte du superviseur ; un second contrôle a ensuite révélé un nom de fichier imposé à tort dans l’adaptateur. La comparaison réelle affichait une autre preuve modifiée depuis le dernier refus. Le contrôle a été corrigé pour exiger le verdict de péremption sans figer ce nom. Les deux échecs restent enregistrés ; aucun appel IA supplémentaire consommé. Le défi frais 6eed333a5ece7df55ad8d9d74371a500 a ensuite réussi via CUA, avec fermeture et focus, au même producteur et aux mêmes limites.

Suite Go complète courante : PASS 455,480 s, sortie 0. Candidat 6bc4552e332931e72db77ca17ca8fe54bbbaee61aa4512affef9f29ba47dc6be de 48 fichiers. Tous les fichiers Go étaient figés pendant cette suite ; l’unique changement post-départ était l’adaptateur Python, ensuite exécuté réellement et compilé. Le résultat précédent de 992,378 s reste historique. Les quatre contrôles T4 ont réussi ; la revue et l’acceptation restent distinctes.

### Recette web complète après la revue 35

Le verdict inconnu portait sur la couverture : seul l’aperçu était piloté pendant le défi moteur. Un nouveau processus de recette à défi 120 s a donc capturé les trois étapes réelles : refus r249, correction r250, reprise web r251, captures et DOM horodatés durant ce même défi. Le processus vérifie aussi les événements durables CLI et termine code 0. Le reçu complet est ensuite contrôlé par le moteur en lecture seule : muter la tâche pendant ses contrôles invaliderait légitimement la révision figée de validation. Les preuves sont externes et attribuées, sans prétendre fournir une certification cryptographique du pilote. La mission est suspendue le temps de geler ces preuves puis reprise ; aucun nouveau producteur ni plafond.

### Revue 36 : format de citation, garde conservée

Le fournisseur a ajouté son commentaire après un extrait JSON dans evidence. La garde de citation exacte a refusé ce format ; aucun PASS n’a été enregistré. Le contrat de sortie est précisé dans le vrai prompt : un court extrait contigu seulement dans evidence, commentaires dans reason. Le test ajoute le refus d’une citation suivie d’un commentaire et vérifie la consigne réellement livrée au fournisseur. Aucune normalisation qui invente une preuve ou supprime une objection. Une nouvelle suite complète couvre cette modification de prompt avant clôture.

### Erreur de revue archivée : bouton de reprise absent

Après une modification de politique, le moteur archivait aussi les erreurs de format mais son chemin de reprise ne consultait que les refus. Le résultat complété devenait donc inaccessible sans nouvelle production. La garde inclut maintenant une erreur archivée terminée, même contrat et même tentative complétée, uniquement si une preuve précédemment liée reste lisible et a réellement changé. Le test refuse le dossier inchangé et la preuve supprimée, puis vérifie la reprise publique, les compteurs non remboursés et l’historique conservé. Ciblés PASS 0,456 s, vet PASS. Une suite démarrée avant cette découverte a été arrêtée explicitement, sans la compter comme PASS ; la nouvelle suite couvre aussi cette correction.

### Dossier actuel, sans mélanger bilan initial et recette externe

La revue 38 interprétait les captures de l’ancien avis comme une injection et assimilait encore la limite initiale du producteur aux recettes ultérieures. Le dossier courant est donc une synthèse attribuée au superviseur, avec les originaux conservés séparément. Un essai de capture du graphe a expiré après 120 s et ne compte pas comme réussite. Après remise publique en Bloquée r281 (avis r277 toujours non favorable), la recette sur fiches termine code0 : correction r282, reprise web r283, trois observations réelles en 105 s dans un défi300s. Captures de fiches lisibles sans recopier un ancien verdict. Aucun nouveau producteur ni plafond d’agent ; la remise en état est attribuée au superviseur, pas inventée comme nouvel avis IA.

Suite Go finale : PASS 473,034 s, sortie0, sur candidat a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77. Aucune modification Go après cette suite.

## 3 octobre — erreur de citation JSON durant la revue T4

Après les quatre contrôles réellement réussis à r286, le 39e appel de revue a terminé en erreur à r288 : le vérificateur a cité plusieurs propriétés JSON en supprimant les retours à la ligne. Le moteur a conservé son contrôle de citation exacte et refusé l'extrait. Cette erreur de format ne démontre ni une défaillance fonctionnelle ni un critère validé.

Correction de présentation : le contrôle CLI produit désormais le même objet JSON sur une ligne ; ses assertions, accès en lecture seule et plafonds restent identiques. Contrôles rejoués, 40e revue en cours à r291. Aucun appel remboursé, aucun verdict réécrit. Ce coût relève de la combinaison entre format de preuve et respect du contrat de sortie du vérificateur. Piste d'amélioration future à tester : séparer les références structurées aux résultats moteur des citations de prose, en conservant fraîcheur et liaison aux critères. Cette piste n’est pas présentée comme implémentée.

Le plafond autorisé est atteint à 40 ; une demande explicite propose 42 uniquement pour permettre la clôture T4/T5. Aucune hausse n’est appliquée sans réponse.

### Clôture T4

Le 40e appel a rendu un avis favorable sur les trois critères à r292. Gate courante enregistrée r293, puis acceptation normale dans l’UX r294 (6/6 contrôles, 100/100, preuves fraîches). Mission : 5/6 acceptées ; T5 reste à produire et vérifier. Aucun contournement ni nouvel agent T4.

## À traiter — rendre les plafonds compréhensibles et accessibles

### Configuration existante, vérifiée le 3 octobre 2026

- **Revues IA** : cockpit → **Budgets et coûts IA** → **Plafonds de planification et de vérification** → **Appels du vérificateur**. Saisir le plafond et un motif, puis **Prévisualiser les plafonds** → **Autoriser ces plafonds**. Le réglage est propre à la mission ; les appels consommés restent conservés.
- **CLI des revues** : `swarm quotas show <mission>`, puis `swarm quotas preview <mission> --input quotas.json` et `swarm quotas apply <mission> --input quotas.json`. Le champ est `limits.review_calls`. La demande inclut aussi les plafonds de planification et de décision à conserver, `schema_version`, `event_id`, `expected_revision` courant et `reason` ; ne pas présenter ces commandes comme une modification partielle implicite.
- **Appels d’outils par tentative** : cockpit → **Administration** → choisir la portée → **Configurer cette portée** ou **Modifier cette portée**. Équivalent CLI : `swarm run-limits show`, `apply`, `history` et `effective` avec les arguments de portée décrits par l’aide. Champ `max_tool_calls`. Ces limites et celles du plan ne sont pas le budget de revues. Les tentatives déjà lancées gardent leurs limites.
- **Coût et jetons** : réglages distincts dans **Budgets et coûts IA** ; un appel de revue, un appel d’outil et une requête interne du fournisseur ne sont pas des unités interchangeables. Un coût non rapporté reste inconnu.

### Incident observé et améliorations à faire

La mission a atteint 40/40 revues, avec 5/6 tâches acceptées. Le CLI `quotas show` indique zéro revue restante. Le départ T5 a été refusé par le moteur : « Vérificateur IA indisponible ou budget épuisé : aucun nouveau départ autorisé ». Pourtant le suivi annonçait « Prête pour un départ automatique » : cette incohérence de présentation est constatée, pas corrigée dans ce lot.

- [ ] Depuis un blocage de budget, fournir un bouton direct vers le réglage exact de la mission, avec consommation, plafond et restant.
- [ ] Faire correspondre l’état « prête » aux conditions réelles de départ, y compris la disponibilité et le budget du vérificateur.
- [ ] Distinguer dans les aides les revues IA, outils par tentative, activations de planification, jetons et coûts.
- [ ] Expliquer avant confirmation l’effet du changement : historique et consommation conservés, pas de validation ; une mission active peut reprendre au prochain passage du conducteur.
- [ ] Recetter ces parcours en FR/EN, dans les deux thèmes et au clavier, ainsi que les commandes CLI sur une racine isolée.

Ces points sont un suivi d’amélioration, sans hausse de plafond ni modification du moteur effectuée par cette note.

### Autorisation de clôture

Après la proposition explicite de plafond 42 et l’explication du blocage 40/40, l’utilisateur demande « on finit ce projet ». Cette instruction est retenue comme autorisation de la proposition bornée : deux revues supplémentaires uniquement, sans remise à zéro des 40 consommées ni modification des tentatives et plafonds d’outils. La configuration passe par quotas preview/apply publics. Les améliorations UX listées ci-dessus restent un suivi à faire, distinct de la recette globale courante.

### Revue T5 r301 — erreur de sélection des pièces

T5 a terminé en une tentative avec 12 outils (7 lectures, 2 écritures, 3 non classés), puis quatre contrôles réussis. L’appel 41 a refusé des captures sélectionnées à tort : QW1 anglais clair legacy au lieu du scénario normal, QW2 ancien incident de revue au lieu des cas organisation/tentatives. Le correctif concerne la composition du dossier, pas les sources du candidat. Recapture CUA anglaise claire normale et sélection des huit bonnes images QW2 ; revue 41 et fichiers originaux conservés. Le rapport précise qu’une revue non favorable a bien eu lieu avant la proposition conditionnelle, sans prétendre à une livraison avant avis favorable et acceptation.

### Refus T5 r306 — mesures non datées et boucle de clôture

L’appel 42 valide la recette visuelle des quatre parcours, mais refuse le RETEX : son tableau ancien à 40 revues n’était pas clairement daté alors que le contrôle ultérieur mesurait 41. La correction ajoute l’instantané public complet r306, après revue 42, sans effacer le tableau original du producteur. Chaque chiffre doit être lié à un instant et une révision ; un compteur peut évoluer durant la supervision.

Le critère final mélangeait preuve de dossier et futur avis favorable de sa propre revue. Clarification proposée : évaluer candidat et dossier, puis faire respecter l’avis courant, la fraîcheur et l’acceptation dans la phase de clôture moteur. Cette correction de définition ne doit jamais supprimer le garde indépendant. Un appel 43 est demandé explicitement ; plafond 42 inchangé en attendant la réponse.
