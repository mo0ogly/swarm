# PAEA81 — acquisition source dans le propriétaire et staging service

Reprise15:44UTC, dernier80/continuité80 relus, v237 réellement active,
ELF9d3346aebb56f76fdb1d58415475aef500e7f569d01258081afcb00f8e7fb272.
PA00…13 et RETEX conservés, PA02/03partiels,0/17. Aucun commit/push/délégation.

Replan : job80 recevait une Source déjà observée et des fichiers arbitrairement
privés. Extraire les témoins physiques sans changer les contrôles, puis construire
un job Pending avant toute observation Git, avec staging créé depuis la capacité
Root service détenue. Comparé à un second builder : même propriétaire/state et
mêmes fermetures, sans copie de sources ni nouveau chemin protégé accepté.

Producteurs vérifiés : current publication → native original → verify_result et
hash_git_report. Leurs Git en lecture utilisent cwd résolu à spawn et Git-dir . ;
la lane et les parents physiques doivent être déjà détenus quand une erreur garde
Child vivant. Le constructeur public observation-first reste historique ; aucune
conformité globale de ses autres consommateurs n'est déduite du nouvel accès job.
Une recherche a d'abord visé un nom de fichier inexistant : rg a retrouvé le vrai
managed_git_current → managed_publication/writer → managed_repository_result.

## Replan core01 — source déclarée concurrente incomplète

Les8unités et le SDK normal ont tous passé, mais le collecteur refuse la clôture :
source_stable=false, planning_stale_gate.rs est apparu après l'inventaire initial
alors que planning_native.rs le référençait déjà. Autre session, pas une erreur
fonctionnelle Source ; preuve originale FAIL/VERIFIED et ELFs gardés. Ne pas
corriger l'archive ancienne. Nouveau core02 inventorie les sources complètes et
garde le verrou build pendant toute la recette pour empêcher une édition coopérative
durant les compilations ; libération avant launcher, aucune suppression de verrou.

Amélioration du vérificateur découverte à cette reprise : le SDK de rebind faisait
son assertion avant restitution du nom original. Il capture désormais le résultat,
restaure les deux fautes connues, puis vérifie le refus ; un résultat inattendu ne
laisse plus volontairement l'original sous source-held. Rejouer ce SDK modifié
est nécessaire ; les anciennes preuves restent distinctes. Ces substitutions sont
réversibles sur notre fixture inerte78, avec contrôle final des octets DB et des
identités ; les jobs sont read-only côté source, le SDK ne prétend pas être sans
opération de fichiers. Aucun fichier de résultat historique n'est réécrit.

## Replan de frontière de création du dépôt

La revue de producteur a établi que les provision/open publics exécutent aussi
un Git bare avant de rendre la capacité. Une fabrique protégée ne doit donc pas
les appeler à chaque acquisition. Extraction de la garde FS seule du même code,
puis fabrique Root interne qui vérifie la déclaration existante et crée Pending ;
bare/format natifs sont contrôlés dans le job. Les API publiques historiques
gardent la vérification bare, avec observation après construction de leurs cadres
FS ; refus toujours sans adoption, pas de claim global sur leurs autres lecteurs.
Nouveau cas bare endommagé réel : Pending sans Git puis refus du job avant pack,
restauration puis nouveau job valide. SDK normal formats ajouté car contrat partagé.

Le lancement core02 a été refusé avant édition par un autre build propriétaire.
Sa session courte n'a pas été relue assez tôt : corriger aussi notre suivi en
vérifiant explicitement résultat/session, puis CPU/owner avant toute reprise.
Aucun check n'était en cours à ce refus, aucun verrou ou processus d'autrui tué.

## Replan core02 — manifeste/lock momentanément incomplets

Core02 a refusé Cargo --locked avant toute compilation/test : une autre session
avait ajouté la dépendance Linux directe swarm-biba-linux au manifeste moteur,
et sa mise à jour Cargo.lock n'était pas encore disponible. Les deux inputs
restaient stables pendant le refus ; Cargo.lock a ensuite changé, ajoutant
exactement cette arête. Ne pas supprimer --locked ni régénérer arbitrairement les
versions. Build concurrent observé en progression CPU, conservé jusqu'à sa fin.
Core03 doit vérifier metadata offline/locked sans mutation avant les11cas, sous
verrou coopératif complet. Module planning et dépendance Biba restent attribués
à leur autre propriétaire ; aucune acceptation de ces fonctionnalités par PAEA.

## Replan — nouvelle dépendance transitive, portée de preuve

La nouvelle dépendance directe moteur/Biba compile aussi src/kernel.rs, absent du
normal237 précédent et des patterns explicites anciens Cargo.toml/src/lib.rs.
Core03 ne peut donc qualifier les octets de ce module par son seul inventaire
déclaré, même si ses cas passent. Ses originaux restent dans leur portée déclarée.
Remplacer ces deux chemins par le crate complet dans les13recettes PAEA concernées,
sans changer commandes/droits/politiques ni réécrire les preuves anciennes. Les
lots antérieurs n'avaient pas ce nouveau fichier dans leur graphe normal237 ;
leur histoire reste valide dans cette portée, les nouveaux replays élargissent
l'inventaire. Core04 complet et stable inclut kernel.rs avant les11cas, sousverrou.
Le module Kernel et l'arête de dépendance appartiennent à l'autre session ; PAEA
n'en déduit pas confinement, validation fonctionnelle ou acceptation indépendante.

Core03 a encore refusé la qualification, malgré8unités PASS : deux fichiers
Cockpit référencés par root_store.rs sont apparus après le gel déclaré. Verrou
coopératif insuffisant contre des créations de fichiers d'une autre session.
Conserver le refus ; adapter la recette via les vrais depfiles Cargo : candidats
gelés AVANT --no-run, chaque input compilé doit être prélié/identique, puis recette
aux chemins explicites gelés. Aucune nouvelle source effective n'est admise depuis
un cache, aucun échec ancien réécrit, aucune copie de checkout ni exception parnom.
Le collecteur/profiler existant reste l'autorité de comparaison des depfiles.

Revue de droits du nouveau constructeur : vérifier explicitement le vrai mode
read-only du Root service, avant mkdir et avant exécution d'un job déjà créé.
Hypothèse à reproduire avant correction : les descripteurs FS peuvent encore
permettre les écritures malgré un Root ouvert en lecture seule. Cas témoin réel
puis Root readonly réel, fermeture des propriétaires avant verdict ; pas de
confusion avec séparation UID/humaine. Correction conditionnée au résultat rouge.

## Refus de droits reproduit avant correction

readonly-red04 : contrôle writable valide, puis vraie Root readonly sur la même
base. Résultat avant correction : constructor_rejected=false, staging_created=true,
run_rejected=false ; le test rouge attendu101 est retenu, tous propriétaires fermés
avant assertion. Ce Root readonly permettait bien mkdir et admission de l'import
FS, malgré sa connexion logique lecture seule. Ajouter le contrôle self.writable
avant ouverture/création de la fabrique, dans le constructeur depuis capacité et
avant tout Git de run_once. Les causes/artefacts restent conservés, pas de retry
implicite ; le cas vert doit vérifier les trois résultats inverses. Ceci ne prouve
ni séparation UID, ni autorité métier/humaine/indépendante.

Le vérificateur maintenu réutilise le profiler natif et le runner de recettes,
prélie avant --no-run tous les candidats et les méthodes, exige chaque vrai input
Cargo dans ce gel avec octets identiques, puis exécute les assertions nommées sur
chemins gelés explicites. Les nouvelles entrées non compilées sont conservées à
part ; une nouvelle source effective ou un changement reste un refus. Aucun
snapshot de checkout, aucune désactivation des contrôles ou admission d'un cache.

## Reprise GLM-driver du 28oct 17h16–18h20UTC — intégration des revues payées

La nouvelle instruction humaine « GLM uniquement » fait de ce contexte le principal
outillé ; les deux revues SKYNET déjà payées sont réceptionnées comme propositions
et leurs défauts sont vérifiés contre les sources réelles avant intégration. Aucun
nouvel appel GLM, aucun sous-agent, aucun message aux autres chats.

- C1 confirmé et corrigé : `poll_retained_processes`
  (rust/crates/swarm-engine/src/validation_run_recovery.rs) ne traversait que
  `RootStoreError::Launch/Cleanup` ; les enveloppes `WorkRecorded`,
  `TrustPublished`, `PreparationRecorded`, `LifecycleRecorded`,
  `AuxiliaryRecorded`, `Published`, `AgentStorageBusy` et `SqlTransaction`
  retombaient dans `_ => vec![]` alors que `planning_native.rs` enveloppe des
  causes arbitraires dans `WorkRecorded.primary` : un enfant retenu rendait le
  job inclosable. Traversée purement additive ; `Busy` reste non traversé car sa
  cause `WorkEngineError` ne retient pas de Child. Nouveau test
  `retained_process_recorded_wrappers_recovered` (échouerait avant le patch) ;
  4 tests recovery PASS. Le témoin lecture seule est rejoué sur le binaire
  reconstruit : `constructor_rejected=true staging_created=false run_rejected=true`
  (preuves glm-driver/recovery-c1-rerun, hachage ELF conservé). Le refus rouge
  readonly-red04 reste l'original conservé, non réécrit.
- C2 confirmé, statu quo documenté (option b de la revue, arbitrage propriétaire
  conservé) : `transfer.rs::prepare` et `inspect_object_format` lisent les
  settings sans réservation partagée. Effet borné : staleness de deadline/env
  git d'un seul job, aucun effet ref/grant. Doc-comments mis à jour dans les
  deux fonctions au lieu d'une contention nouvelle non décidée.
- Les propositions de revue non intégrées restent conservées : patch collecteur
  (depfile continuations, `LIBSQLITE3_FLAGS`, `failure.json`, `executables.after`,
  `--unit-only` refusal, étiquette rustdoc) — refus additifs utiles mais non
  vérifiés par exécution dans le temps restant ; ils restent des hypothèses de
  la revue qualitée, pas des défauts confirmés par reproduction ici.
- Qualification complète toujours refusée : `planning_native.rs` et
  `planning_run_material.rs` re-modifiés par l'autre session à 17h56/18h08
  locales, après le gel final05. Aucune recompilation de qualification relancée.
  Condition de reprise inchangée : fenêtre de sources stables + recette complète
  + raccordement service protégé + prérequis administrateur/UID encore ouverts.
