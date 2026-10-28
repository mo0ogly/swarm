# Améliorations conservées après bascule / Retained after-cutover improvements

[File des projets APEX](../FILE-APEX.md) · [APEX project queue](../../en/APEX-QUEUE.md).
Le registre rassemble les plans et leurs dépendances ; il ne lance aucun lot.
/ The register brings together plans and dependencies; it starts no work package.

Décision utilisateur du 22 octobre 2026, méthode APEX : ne pas inclure le
durcissement supplémentaire dans le chemin critique sans défaut reproduit ou
exigence de contrat. / User decision: defer additional hardening unless required
by a contract or a reproduced defect. Documentation is not verified effect.

| ID | Origine / Origin | Valeur / Value | Périmètre et propriétaire / Scope and owner | Critère observable / Observable outcome | État et reprise / State and resumption |
|---|---|---|---|---|---|
| AC-01 | Plans101/120 et lecture des sources189, lot201 | Diagnostic privé durable du bootstrap / Durable private bootstrap diagnostics | Canaux stdout/stderr superviseur, Codex racine / Supervisor output channels, root Codex | Création Root-bound, permissions, refus/effets partiels et conservation vérifiés / Verified Root-bound creation, permissions, refusals/partial effects and lifecycle | Proposition différée : Go utilise os.DevNull ; pas de politique de rétention héritée. Reprendre sur besoin produit validé ou défaut réel reproduit ; décider persistance/Admin et visibilité avant activation / Deferred proposal: no inherited Go file-retention contract; resume on validated product need or reproduced defect, deciding lifecycle/Admin and visibility before activation |
| [AC-02](../actions-confiance/securite-execution-apex.md) / [EN](../../en/plans/actions-confiance/securite-execution-apex.md) | Demande utilisateur du 23 octobre : enregistrer, sans démarrer ; Bromure/OpenShell et Agent Baseline Software Factory/YAML / User request: save without starting | Confinement, réseau/secrets, traçabilité, menus et catalogue de contrôles / Confinement, network/secrets, lineage, menus and control catalogue | Actions de confiance ; Codex racine / Trusted actions; root Codex | Contrats SE/UX/SF, quatre parcours et preuves liées au même candidat / SE/UX/SF contracts, four journeys and evidence bound to one candidate | Plan enregistré, A0–A7 non démarrés. Reprise après migration/qualification Rust et demande explicite de démarrage. Références clair/sombre et YAML conservés ; aucune activation / Recorded plan, A0–A7 not started. Resume after Rust migration/qualification and explicit start request. Light/dark references and YAML retained; no activation |

Sources : [lecture189](evidence/20261022-root-instance-201/worker-channel-parity-next189.md),
[prompt adopté](PROMPT-METHODE-CODEX.md). Cette décision retire uniquement la
création de fichiers de sortie comme prérequis du conducteur ; elle ne dispense
pas de conserver les causes de spawn, exit, attente, arrêt et nettoyage. Les
reçus natifs et logs métier gardent leurs contrats propres. / This only removes
output-file creation as a conductor prerequisite. Spawn, exit, wait, stop and
cleanup failures must still be preserved; native receipts and business logs keep
their own contracts. No code, retention policy or runtime activation is implied.

### Diagnostic des chemins d'artefacts — découverte321

Origine : status terminal, GateContractError::Evaluation→GateEvaluationError→
LocalPathError::OutsideProject. Causes typées conservées, mais la projection sûre
CLI expose seulement gate-evidence-invalid (pas le sous-motif). Valeur attendue :
diagnostic plus actionnable sans chemins privés. Propriétaire : racine/contrat
native_diagnostics, périmètre commun CLI/web. Critère : labels statiques précis
pour ces familles, matrice source/IOwrapper/cleanup et absence de fuite vérifiées.
État : amélioration documentée, non réalisée ; report car le contrat actuel est
respecté et la recette confirme déjà le refus et la cause typée à leurs couches.
Reprise : travail commun sur l'explication des erreurs après bascule, ou défaut
réel du parcours utilisateur démontré avant celle-ci. Aucun droit supplémentaire.

### Distinguer bail périmé et absence dans l'UX de supervision

Origine : relecture GLM341 du projecteur339. La présentation Go-compatible
mappe stale→absent sans clock_issue ; cela ne prouve pas l'absence de ligne ni
la vie du processus. Valeur hypothétique : expliquer la dernière présence au
lecteur sans changer le sens de sécurité. Propriétaire UX/supervision ; état
opportunité documentée, non réalisée. Reprise après bascule avec contrat JSON
compatible et parcours UX montrant bail absent/périmé/arrêté, sans nouvelle
autorisation ni prétention d'authentifier le processus.

### Explication du planning avant le premier résultat — découverte425
Origine : revue GLM425 du raccord status initial, preuve publique424 planning-status-configured/authorized. Le message « Les résultats attendent une décision du responsable du périmètre » est observable avec zéro tâche/résultat ; Rust reproduit exactement le Go mission_status.go585–586 (pas un écart introduit). Valeur : expliquer la préparation initiale sans laisser croire à des résultats ou à un planificateur actif. Propriétaire produit/UX, intégration racine. État : amélioration qualifiée, non réalisée ; report selon la décision de parité stricte des sorties JSON, pour garder ce raccord borné. Reprise : évolution explicite du texte de présentation après bascule ou décision de changement du contrat avant celle-ci. Critère : zéro tâche affiche une préparation initiale distincte des résultats existants, sans confondre configuration/autorisation/conducteur actif ; vérifier versions FR/EN et parcours CLI/cockpit, garder le statut de sécurité inchangé. Le diagnostic de capacité proposé par GLM ne s’applique pas à une lecture pure ; les limites positives viennent des métadonnées validées, pas de defaults zéro inférés.

### Préciser la dimension du budget refusé — découverte436
Origine : quatrième claim de la recette statut, planning_activation préserve le typed PlanningBudgetError mais publie sa famille budget_exhausted ; le JSON CLI ne distingue pas activations/décisions/tâches. Valeur attendue : aider l’opérateur à retrouver le paramètre concerné sans diagnostic privé. État : amélioration proposée, non réalisée ; propriétaire Root/contrat erreurs CLI. Reprise : comparer le contrat Go strict et les consommateurs, puis définir un champ sûr ou une explication compatible ; vérifier trois dimensions et absence de mutation. Le refus et la lecture du plafond réel sont vérifiés437 ; aucun assouplissement de budget autorisé par cette découverte.

### Motif sûr des refus de décision scoped — découverte457
Origine : refus testé de titre+réouverture retourne seulement format, alors que TaskLifecycleError conserve scoped-initial-decision-fields-invalid. Valeur : indiquer quel type de contrat est refusé sans révéler donnée privée. État proposé, non réalisé ; propriétaire Root/contrat diagnostic CLI, relié à la découverte436 de famille budget générique. Reprise : vérifier parité Go/consommateurs et convenir d’un motif statique sûr compatible, puis tester champs/état/historique sans mutation. Le refus lui-même est vérifié458 ; ce suivi ne change ni droits ni garde.

### Collision entre arrêt local et échéance d’un rôle — découverte993
Origine : revue de l’interruption foreground. Hypothèse à vérifier : lorsque
un signal et une échéance deviennent observables ensemble, le motif public
peut être l’interruption locale même si l’échéance est déjà due. Le rôle est
arrêté dans les deux cas, sans succès métier ; aucune extension de délai n’est
observée. Valeur : attribution exacte du motif et stabilité du code de sortie.
Propriétaire Root/contrat exécution. État : hypothèse, non vérifiée ; comparer
le contrat Go et construire une collision déterministe avant de changer une
priorité ou un délai. Reprise si le parcours démontre un refus masqué ou lors
de la qualification temporelle ; aucune nouvelle borne de test autorisée.

### Unifier le cache des révisions connues — découverte1051
Origine : revue GLM1050 et régression1051 ; interrupted_phase remplaçait rev5
par rev3. Le défaut commun est corrigé1052. Amélioration de méthode/code :
centraliser la mise à jour highest-same-Work employée par publish/interrupted
et les autres retours de phases, pour prévenir une dérive future. Valeur :
cohérence des erreurs tardives, sans prétendre que chaque writer actuel régresse.
Propriétaire Root/contrat diagnostics. État proposé, reporté hors de ce parcours ;
reprendre au prochain changement de ces writers, avec témoin/rejeu de sortie
tardive et sans comparaison de révisions entre Work différents.

### Durée de vie des peers avant concurrence complète — découverte1059
Origine : job1042 ouvre un Root à chaque activation, même si planning_once
retourne sans appel quand aucun planner n’est configuré. RootStore::open_mode
applique le bootstrap identité avant la politique SQLite ordinaire. Valeur
attendue : éviter les ouvertures inutiles et qualifier le comportement avec
plusieurs writers avant d’étendre la concurrence aux autres rôles. Hypothèse
non reproduite : une ouverture peer pourrait rencontrer un refus SQLite BUSY
pendant un commit d’un autre rôle ; aucune panne actuelle n’est déduite de ce
risque, les quatre recettes1054/1055 passent.
Propriétaire Root/contrat IO et ordonnanceur. État proposé, non réalisé. Reprise
à la conception des rôles asynchrones supplémentaires : comparer peer par appel
et peers conservés, mesurer créations sans planner, reproduire témoin valide →
writer retenu sur fixture possédée → libération/reprise. Conserver identité,
réglages effectifs et causes ; ne pas augmenter le timeout technique de bootstrap
ni contourner la liaison au Root pour masquer un diagnostic manquant.

###1072 — apprentissages et limites du prochain scheduler
- Origine1064 : diagnostic de contexte actif dépendant de la phase. Corrigé1065,
  registered1067 PASS ; propriétaire conducteur. Prévention : une observation
  d’état ne doit pas attendre un job qui ne lui confère aucun droit.
- Origine1067 : marker dispatch utilisé comme preuve de fin métier. Corrigé1069,
  garde + manual1070 PASS ; propriétaire recettes. Les quatre appelants contrôles/
  reviewer attendent leur reçu réel ; refus restent des sorties non nulles.
- Amélioration proposée, non mesurée : chaque phase vide attend la cadence saved
  et ouvre un peer. Propriétaire scheduler/Root, lié1059. Mesurer latence de phases
  vides et coût des opens avant de choisir peer retenu ou notification ; aucune
  nouvelle borne, retry ou duplication des gardes d’éligibilité n’est autorisée
  par cette proposition. Reprise au prochain changement du scheduler.
- Limite observée1067 : SIGINT avant permission peut conserver ConnectionReset
  gate et interruption, sortie non nulle avec Work connu. Les six signaux
  qualifiés sont après permission réelle. Propriétaire IO : comparer arrêt avant
  claim/permission/exec et après grant avec témoin/reprise, puis décider sur causes
  et reçus réels ; ne jamais effacer une erreur indépendante pour obtenir code0.

###1170 — transmission des huit vagues
- Statut dégradé et causes secondaires : erreur d’interprétation1132, contrat CLI
  relu et corrigé1141 ; source du DTO et responsabilité d’acquittement séparées.
  Propriétaire API/CLI, preuve1165 et tests cause-transfert1142 ; leçon réutilisable
  dans les lecteurs futurs, pas preuve de conservation illimitée après acquittement.
- Restauration de fixture : RED1131 puis tentatives indépendantes ; une panne
  de restauration reste un refus avec original conservé, reprise après correction
  de sa cause, sans retry caché. Propriétaire recettes, quatre gardes1165.
- Comparaison : horodatages runtime/dépenses et compteurs libres sont des mesures
  indépendantes ; montants, appels, droits et états restent exacts. Propriétaire
  banc différentiel, source comptable relue, garde refuse changement des calls.
- Hypothèse GLM d’expiration d’échanges confondue avec queued worker non adoptée :
  objets distincts, aucun reproducteur fourni. Propriétaire G3 ; reprendre avec
  vrai lecteur/writer d’échange et contre-exemple, pas à partir d’un nom similaire.
- Parcours deux tâches et récupération managed proposés, non implémentés dans
  ce lot ; owner G3, dépendance avant futur conducteur serveur global. Conserver
  noms/interfaces réels et droits, pas de nouvelle orchestration sur avis seul.
- Méthode :9 réponses tronquées sur64 à2000tokens malgré effortlow. Certaines
  hypothèses de modèle contredisent les codecs réels ; conserver arbitrage et
  source avant intégration. Réduire les tâches/paquets futurs selon contrat,
  pas augmenter seulement le budget. Économie Codex non mesurée.
