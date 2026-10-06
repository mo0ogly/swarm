# RETEX de supervision — 4 octobre 2026

Mission w-01567e073c1ed2f3d4c71c9e ; préparation prep-f9037cb7d0d128597ace824a ; lot QW5/QW6/QW8/QW7.

## Préparation réelle par l’interface

Besoin enregistré, RETEX-CLOTURE.md joint intégralement en extrait (lignes 1–67), brief Claude reçu et adopté. La proposition de plan a expiré après 120 secondes, sans relance automatique. Le superviseur a rédigé le JSON à six tâches dans l’éditeur, résolu les décisions d’organisation, vérifié le plan et le prévol, créé la mission puis confirmé son lancement. Ce travail est attribué au superviseur et ne démontre pas une préparation autonome réussie.

Les sélections programmatiques de listes ne déclenchaient pas les gestionnaires filtrant isTrusted. La sélection au clavier a déclenché la résolution réelle Claude/sonnet. Cet incident concerne la méthode de pilotage du navigateur ; il ne prouve pas que la sélection native d’un utilisateur est cassée.

## T1 — production et correction documentaire

Un producteur terminé normalement, neuf outils observés (deux lectures, une écriture, six non classés), plafond 25, pas de nouvelle production. Le rapport initial est préservé dans T1-worker-original.md. Le superviseur a rectifié : coût des 45 revues présenté à tort comme revues 41–45 ; recherche négative d’un libellé interprétée à tort comme absence de fonctions budget/quotas ; budget d’exploration présenté comme épuisé malgré neuf outils sur 25. Le dossier courant est condensé à environ 6,5 Ko, avec attribution des rectifications.

Contrôle documentaire réel : quatorze fichiers présents, huit lignes du tableau conformes à CLOTURE-r332.json, quatre contrats bornés, mutations publiques budget/quotas constatées dans leurs sources ; git diff --check code0. Reçu T1-supervisor-check.json. Aucune recette des corrections produit encore revendiquée.

## Délai du vérificateur

Les deux premières revues ont expiré à 90 secondes sans événement final : aucun verdict, aucune acceptation. La deuxième utilisait le dossier condensé. Le délai a été porté explicitement à 300 secondes via planning review-timeout, puis reprise publique du même résultat. Le plafond de 40 appels reste inchangé ; aucune consommation remboursée. La réduction du dossier seule n’a donc pas suffi à produire un verdict dans 90 secondes ; la cause interne du fournisseur reste inconnue.

Le conducteur et le vérificateur sont réels ; leur activité ne vaut pas réussite. La troisième revue review-80789b7d77f2ee3358c696ff a rendu PASS sur les trois critères le 2026-10-04T07:46:24.811672585Z. Gate documentaire courante enregistrée puis acceptation normale dans le cockpit ; aucun contrôle de correction produit revendiqué. T1 acceptée et T2 démarrée automatiquement par le conducteur. L’acceptation documentaire est distincte d’un reçu de contrôles exécutés par le moteur ; les vérifications du superviseur sont attribuées dans T1-supervisor-check.json.

## T2 — exploration arrêtée, correction bornée et preuves transmises

Première tentative : 60/60 outils, interruption sans rapport final ni recette navigateur terminée. Le superviseur a repris le code : bouton budget visible dans le pilotage principal (l’inbox était fermée), clé de rafraîchissement incluant les décisions, accès aux quotas épuisés, routage de la portée tâche et distinction du plafond du plan. Deux recettes isolées budget/quotas FR/EN × deux thèmes et CLI ont passé ; npm test, go build/vet et diff check ont passé. Aucun fichier Go modifié.

Deuxième tentative existante, sans hausse : plafond réduit à 12, cinq outils observés pour vérification d’intégrité et remise, terminé normalement. Correction et contrôles attribués à Codex, pas présentés comme production autonome du second agent. Première revue de T2 : unknown sur les trois critères car le contexte n’incluait ni preuves annexes ni captures ; quatre appels reviewer consommés au total depuis le début du lot. Cet incident démontre que référencer des preuves n’équivaut pas à les transmettre.

Le superviseur a joint le contenu des reçus/empreintes au rapport et configuré, via validation preview/apply, trois vrais contrôles moteur en mode human (acceptation non automatique) avec sorties partagées et captures explicitement déclarées. Les noms de fichiers arbitraires cités dans le rapport ne sont pas suivis par le moteur. T2-engine-receipt-snapshot.json constate qw5-build, qw5-budget et qw5-quotas exécutés et réussis, état pending_human. Les captures sont des fixtures isolées et non une preuve d’autonomie fournisseur. Réexécution justifiée ici par le passage à un reçu moteur déclaré, pas simple répétition payante du même dossier. Nouvelle revue après modification de précondition : preuves effectivement présentes dans son contexte ; aucun troisième producteur.

Une tentative de remplacer le libellé de livrable hiérarchique par un chemin documentaire a été refusée par le moteur : contrat immuable, aucune dérogation. La solution retenue est une annexe dans le rapport, des inputs déclarés et une reprise publique de revue. L’ancien avis est conservé. Aucun compteur réinitialisé, aucun critère changé. À ce stade, avis final encore en attente ; pas d’acceptation revendiquée.

## T2 acceptée et portée des limites lors du relais vers T3

Revue review-48141078aa930f42c8d1eff6 PASS le 2026-10-04T08:16:40.092323897Z, avec huit images et reçus moteur réellement joints. Gate puis acceptation normale via UX ; mission 2/6, T3 déclenchée par conducteur. Serveur local remplacé par le binaire testé /tmp/swarm-qw5-candidate, même URL/session. Administration vérifiée en DOM réel : dernière tentative T2 5/12, restant7, plafond du plan60 séparé.

Incident de supervision : agent start avec une limite réduite pour la remise T2 a enregistré cette limite aussi dans le profil mission. T3 a donc été interrompue à12 au lieu de son budget initial60. Le superviseur a rétabli le profil mission d’origine (sans limite personnalisée) et celui de T3 via profile public, sans augmenter le budget autorisé60. La reprise explicite conserve néanmoins12 : agents_store.go applique cappedBy(previous.Limits). La garde ne permet pas de relever les limites figées d’une reprise par le seul changement de profil. La première tentative interrompue est conservée ; seconde tentative en cours, pas de troisième tentative ni de remboursement. Ce comportement et l’effet sur le profil mission doivent faire l’objet d’un correctif de portée explicite dans un lot ultérieur, sans suppression de la protection de reprise. Ne pas annoncer que la seconde tentative a60 appels : elle en a12. Le profil rétabli concerne les autres futurs départs.

Métadonnée distincte observée : go version -m du candidat rapporte vcs.revision64cd7a177e05 tandis que le HEAD du checkout est cc3069d. Les empreintes des fichiers et le binaire construit/testé sont explicitement liés, mais la provenance VCS automatique Go n’est pas une preuve du HEAD de ce worktree ; anomalie à diagnostiquer, aucune version de release revendiquée.

## T3 — distinguer les responsabilités et éviter un nouveau cycle

- **Erreur de supervision** : le lancement réduit à 12 appels de T2 a été
  mémorisé comme profil commun ; T3 a donc reçu 12 au lieu des 60 prévus.
  Restaurer le profil ne modifiait pas la tentative déjà créée et la reprise
  conservait correctement le plafond précédent. Deux tentatives T3 consommées,
  première interrompue, seconde terminée normalement après 11 appels avec
  rapport partiel (0,7033048 USD rapportés pour la seconde).
- **Défaut moteur confirmé** : promotion implicite des limites et du délai
  propres à une tâche vers le profil de toute la mission. Correction locale :
  préserver les plafonds communs existants, et ne pas les initialiser depuis
  le premier lancement local. Les réglages de la tâche restent locaux. Aucun
  compteur ou plafond de reprise historique modifié.
- **Défaut de raisonnement du producteur** : le rapport T3 affirmait que le
  garde de clôture couvrait déjà Planning.Failure. Lecture exacte : il ne
  couvrait que les branches suivantes. Deux tests reproduisent le défaut puis
  passent après correction du moteur et du rendu web. Une lecture déclarée
  cohérente n’était donc pas une preuve suffisante.
- **Correction par le superviseur** : après terminaison du producteur, compléter
  code, tests, captures et pièces de revue sans troisième lancement. Attribution
  explicite, rapport initial conservé. Fixtures moteur/CLI isolées et rendu
  Chrome français/anglais, deux thèmes, clavier et focus ; aucune IA appelée
  par les recettes. Une reprise du vérificateur utilise le budget existant.
- **Coût de la fraîcheur** : la correction T3 touche planning.js et les
  traductions, qui étaient liés aux preuves T2. T2 devient périmée même si
  les deux recettes QW5 repassent sur le candidat T3. Revalidation explicite
  du même producteur par l’opération publique retry-review, sans nouvelle
  production. À mesurer au bilan : revues initiales, reprises, revalidations
  induites par fichiers partagés ; ne pas les confondre avec appels d’outils.
- **Temps de vérification** : la suite Go complète prend plusieurs minutes et
  n’émet pas de progression par défaut. Ce temps n’est pas un agent en boucle.
  Deux exécutions ont été lancées ici parce que la dernière correction et un
  test ont été ajoutés après compilation de la première : surcharge attribuable
  au superviseur, à éviter en figeant le candidat avant le contrôle complet.

### Revue des preuves : deux sources de reprises supplémentaires

- T3 : une citation du vérificateur avait remplacé l'apostrophe typographique
  par une simple. Le moteur a refusé le verdict, sans accepter la tâche. Le
  rapport a été normalisé en apostrophes simples avant une reprise explicite.
  Le fond et le code n'ont pas changé. Pas de suppression du contrôle de citation.
- T2 revalidation : les contrôles étaient PASS mais les sorties se limitaient à
  « PASS budget UI » et « PASS quotas UI ». Le nouvel avis a demandé les
  observations détaillées sur historique et clavier. Les recettes exposent
  désormais leurs valeurs observées et vérifient explicitement la conservation
  du préfixe des événements via `work show`, le focus après Échap et les
  compteurs avant/après. Le test moteur existant non nul (4 activations,
  3 décisions, 2 revues) est rejoué et joint. Ces corrections concernent la
  visibilité des preuves, pas une troisième tentative de production.
- Suite Go finale : PASS, 441,792 secondes. La première suite a également passé
  en 434,953 secondes. Les temps et le doublon restent attribués au superviseur.

### Issue vérifiée de T3 et reprise du plan

- Revue T3 favorable 3/3 : `review-f836457fe989de5936dee14e`, terminée
  le 4 octobre 2026 à 08:43:56 UTC. Le reviewer distingue les fixtures du
  fonctionnement réel et relève la limite de détail des sorties CLI.
- Nouvelle revue T2 favorable 3/3 après sorties détaillées et contrôle non nul,
  le 4 octobre à 08:46:03 UTC ; compteurs de revue conservés, 10/40 consommés.
- Deux demandes de gate T3 ont été rejetées sans mutation : le superviseur
  avait utilisé `inputs` au lieu de `artifacts`, puis une chaîne au lieu de la
  liste de preuves. Corrigées suivant le contrat de gate ; ne pas attribuer
  ces erreurs à l'agent ni au fournisseur.
- Gates T2 et T3 fraîches, 6/6 contrôles ; acceptations normales effectuées
  dans l'UX. Statut public vérifié : 3/6 résultats validés, T4 démarrée.
- Serveur remplacé par `/tmp/swarm-qw6-candidate`, même port et même session,
  après fin des revues : aucun agent ni avis interrompu. La version embarquée
  reste un développement avec provenance VCS à clarifier au bilan T6.

### T4 : reprise et consommation observées

- Première tentative : interrompue à 600 secondes, 41 appels, sans rapport.
- Le conducteur a démarré de lui-même la seconde tentative autorisée à 08:59 UTC ; la supervision ne l'avait pas encore constatée lorsqu'elle a annoncé prendre seule le relais. Corriger cette observation, ne pas inventer une nouvelle tentative.
- Seconde tentative auto-d9d0f38349900e043509 : fin normale 09:06:53 UTC, 21 appels, coût brut fournisseur 0,5456906 USD. Entrée 32, sortie 10187, caches lecture 1044843 et création 58697 séparés. Mesures issues du dernier événement fournisseur, aucune somme globale déduite. Coût première tentative non rapporté.
- Deux producteurs ont lancé la suite complète via un pipe tail et le second a terminé sans preuve de fin de cette suite. La supervision a rejoué indépendamment ; doublon et attente à attribuer aussi à son observation tardive de la reprise. Ne pas compter une commande en cours comme PASS.
- Lacune confirmée : indépendantValidationEvidence refuse sans appel mais son erreur était silencieusement absorbée. Projection corrigée par le superviseur pour montrer raison et action, sans écriture ni consommation. Test étendu, métadonnées et fournisseur de fixture explicitement simulés.
- Requête de configuration des contrôles tentée pendant running : refus normal du moteur sans mutation ; configurée après submitted. Courte pause des départs pendant cette configuration, reprise normale, compteurs conservés.

### Issue T4

- Revue indépendante favorable 3/3, terminée 09:11:15 UTC, 11/40 revues consommées.
- Suite Go du superviseur : exit 0 en 464,548 secondes. Elle a démarré avant l'ajustement final du texte de prochaine action ; contrôle moteur ciblé et test final exécutés ensuite sur la source exacte. Limite de provenance déclarée dans T4-candidate.json.
- Gate fraîche 6/6, acceptance normale par l'UX. Statut public : 4/6, T5 démarrée automatiquement, T6 en attente.
- Le sélecteur d'action de la modale n'est pas associé au label Action : getByLabel ne le trouve pas. La valeur DOM du combobox confirme accepted avant validation ; ne pas attribuer l'échec du sélecteur à la gate. À noter à la recette accessibilité.
- Provenance des binaires de supervision : go build brut a repris une autre métadonnée VCS. build.sh est déjà la recette canonique qui injecte le HEAD correct et dirty=true. La supervision utilisera ce script au candidat final ; ne pas transformer cette erreur de recette en défaut moteur sans preuve.

## T5 — comptage, inconnues et recette (4 octobre)
Premier worker interrompu au plafond initial de 60 appels, avant mise à jour finale du rapport. Analyse initiale conservée. Le superviseur a complété durée des processus et mesures inconnues : ne pas attribuer le livrable au seul worker ni affirmer autonomie. Tests ont révélé une ancienne attente erronée (mesures inconnues : 2 et non 1), puis la recette anglaise a révélé des chaînes dynamiques coût/revues/reprises non traduites : corrigées et recontrôlées.
Le superviseur a interrompu sa première suite Go lancée avant gel ; log conservé, pas de PASS. La suite suivante a PASS 408.945s. Une dernière traduction seule est postérieure : tests ciblés et frontend rejoués, limite datée dans manifeste. La recette de capture sélectionnait un ledger en fond de page plutôt que dans la modale ; corrigée par portée #modal, matrice rejouée PASS.
Changer le catalogue partagé périme les preuves T2/T3 : recontrôles et revues via opérations publiques, aucun producteur rerun. C’est un coût de granularité des preuves/source partagé, distinct d’une boucle de production agent. Invocation ./build.sh exit126 était une erreur du superviseur (script lancé normalement via sh). Une requête de politique initiale utilisait directory au lieu de dir ; refus normal, payload corrigé sans mutation partielle.

### Renouvellement T3 : échec fournisseur réel
Revue review-d36b9e098d4a4f1aacba8e92 : exit 1 après 206.7s, résultat final présent, 22290 jetons sortie bruts, coût rapporté 0.4620996USD ; diagnostic interne vide. Les contrôles ont PASS, pas le fournisseur. Cause exacte non prouvée : ne pas inventer panne de quota ou certitude sur longueur. Dossier condensé après conservation de l’ancien rapport ; retry-review public dans plafond original 40, producteur non relancé. Cette tentative payée et les recontrôles restent dans les compteurs.

### T3 : recette réussie mais preuve non observable
Revue condensée : critères 1/2 favorables, critère 3 unknown. Cause confirmée : contrôles capturaient seulement « PASS QW6 history UI » et Go sans -v ; ni opérations clavier/focus ni sorties CLI n’étaient accessibles au reviewer sans outils. Correction des traces, pas du produit : Go -v, sorties réelles mission status FR/EN, quatre observations clavier Enter/repli/focus SUMMARY, erreur courante et historique retained. Recette ciblée rejouée exit 0 puis retry-review public pour nouveau reçu. Cet aller-retour est une lacune de recette du superviseur, pas une preuve d’échec du code ni du worker.

## T5 accepté et T6 : recette, quota fournisseur et relais
T5 seconde tentative bdebfa65-3638-4789-9297-4f3bfab8d8d0 terminée normalement à 7/12 appels ; contrôles préautorisés PASS, avis favorable 3/3 et acceptation publique. T3 revue finale favorable conservant limite de stdout tronqué : correction suivante réservée au wrapper T6, sans modifier les preuves T3 acceptées.
La recette groupée suivante a rencontré une course dans la recette QW7 : snapshot prêt mais bouton pas encore rendu ; attendre réellement le bouton avant focus a corrigé ce défaut de test, puis regroupement PASS. La sortie T6 est filtrée à partir d’observations réelles, 6735 octets, sans suppression du verdict/code d’échec ; le log défaillant est conservé.
T6 auto-17b39e37f002c9254bf4 : 17 appels, interrompue après quota Claude429, reset 12:20UTC le4octobre. Ne pas confondre grep sans résultat et cause d’arrêt : quota refusé est le motif enregistré par le moteur. Pas de handoff, pas de résultat validé. Relais sélectionné par les opérations publiques : Codex déjà configuré, route standard gpt-5.6-sol medium ; reviewer futur Codex, mêmes40 appels et même300s, compteurs conservés, preuves acceptées5/6 toujours fraîches. Tentative restante T6 limitée20 outils,600s ; cooldown Claude non effacé. Ce relais supervisé n’est pas une preuve de bascule automatique multi-fournisseur.

CLI installé : le serveur utilisait déjà /tmp/swarm-qw7-candidate, mais ./bin/swarm (utilisé jusque-là pour mutations publiques compatibles) rendait encore l’ancien JSON sans observed_at. Remplacement atomique du binaire CLI ignoré Git par le même candidat canonique ; sortie réelle observée10:13:21UTC avec observed_at et measurement_source. Nouvelle recette CLI FR/EN de la mission réelle sauvegardée T6-installed-cli-*.txt, aucune mutation de l’historique. Distinguer build, serveur démarré et CLI installé est une responsabilité de livraison, pas seulement de traduction.

### T6 : arrêt pour silence et consolidation manuelle
Codex 147899a3-11a7-4a56-a0b5-f0c0b9b8df85 a été interrompu le 4 octobre à 10:13:47UTC : 13 outils, trois minutes sans sortie visible après le dernier résultat, fenêtre totale600s non atteinte. Rapport initial seulement, aucun final. Activité interne du modèle inconnue : processus vivant ne prouve pas progression. Deux tentatives consommées, pas de troisième implicite. Consolidation du superviseur enregistrée séparément dans T6-consolidation-superviseur.md, aucune acceptation obtenue. Les opérations publiques de récupération consultées sont limitées au Git géré et ne conviennent pas à cette mission en dossier partagé.
Le prochain projet, explicitement autorisé par l'utilisateur, traite versions/provenance, preuves observables avant revue, silence et relais fournisseur. Il ne remplace pas T6 ni ne rembourse ses tentatives. Préparation créée par l'UX, premier échange Codex lancé. Le sélecteur automatisé selectOption ne déclenche pas le traitement réservé aux événements de confiance : sélection au clavier réussie, pas de modification du produit pour contourner ce comportement.

### Nouveau projet réellement lancé
`w-a03028dc0d3f69d1f52f0ee6`, préparation `prep-8c84a9cf6e6351c100106270`, cinq tâches séquentielles. Responsable, exécutants et reviewer Codex standard gpt-5.6-sol, un seul créneau, deux tentatives et60 outils par tâche,40 appels par rôle. Création via UX puis confirmation publique de mission continue ; cockpit confirme un agent actif et0/5 validées le4octobre vers10:26UTC. Screenshot `/tmp/swarm-supervision-new-project.png`.
La proposition IA réintroduisait un worker de vérification et ignorait certaines corrections du brief : plan corrigé manuellement avant validation, reviewer moteur sans outils conservé, borne totale de silence explicite, cinq tâches au lieu de sept. L'édition automatisée fill montrait le texte sans déclencher le suivi des modifications réservé aux événements trusted : saisie clavier supplémentaire puis enregistrement/adoption réels, ancienne proposition marquée périmée. Cet écart de recette ne prouve pas que la saisie humaine est défaillante. Première confirmation de lancement après changement de créneaux refusée avec nouvel aperçu, seconde confirmation réussie ; historique conservé.
