# PAEA71 — dernier candidat Git publié et preuves natives courantes

Reprise28oct09h29UTC : dernierlot70, version223/PID3128318/ELF7143f8c23550d1e00d4ec89f7fb7bec40ed7a38ca3af209ad2dee704d388592f observés, HEADcbd6fecd7e40016d6c259296ac3a21373a4bbd60, aucun test/build actif. Disque1,3Go.
Codex direct sans délégation/GLM/commit/push, échéance28oct19h20Paris inchangée.

Replan PA03 : writer::current vérifie les originaux mais exige une tâche, ce qui
permet à un futur raccord de choisir un ancien candidat accepté. Deux routes :
dupliquer l'admission/revue dans le nouveau service, ou fournir une observation
native fermée du dernier maillon vérifié de la chaîne. Choix : réutiliser chaîne,
contrôles, revue et cohorte réels dans un snapshot SQL/material/settings ; aucune
acceptation ou autorité implicite. L'appelant d'effet devra refaire ce contrôle
sous sa barrière courante et garder les capacités projet/dépôt/service.

Découverte : ManagedRepository n'a pas de repository_id de service. Conserver
repository.request_digest comme observation réelle, sans fabriquer une association
pour Action::Git.repository_id. Propriétaire Codex/PA03 ; suite : provisioning
explicite liant dépôt détenu, projet et service, puis vérification au point d'effet.
Critère : mauvais dépôt/identifiant refuse et dépôt correct conservé pendant CAS.

Critères locaux : deux publications cumulatives réelles issues d'un provider
inerte détenu dans une fixture neuve, jamais une mission utilisateur ou un succès
SQL fabriqué ; observation = dernier événement/candidat et cohorte complets ;
ancien candidat, originaux/SQL/Git corrompus refusent sans vide, restauration et
absence de nouvelle invocation/consommation/écriture. Tests nommés, Rustdoc strict,
revue politiques/fonctions/erreurs aux octets ; build/install local normal ensuite.
PA02/03partiels,0/17/humain/UID/confinement/E2/E3/PA13/RETEX ouverts.

Méthode : une recherche de chemin supposé a échoué avant toute mutation ; chemins
retrouvés par rg -l/import réel. Les extraits ont été réduits pour ne pas traiter
une sortie tronquée comme contrat complet. Pas de relance d'échec inchangé.

Fixture01 PASS via CLI223 réellement retenue ; deux publications cumulatives
réelles, contrôles/revues inertes, aucune acceptation mission/modèle payant. Root
propre et naissance retenue ; archive complète avant02. Ce résultat223 ne prouve
pas encore le nouveau lecteur71. Le build concurrent a été refusé par son garde
profile-effective-input-changed (lib.rs/root_store.rs) ; preuves laissées à son
propriétaire, aucun signal ni nettoyage, pas de récupération de son candidat.
Notre compilation commence après terminalité et sur inventaire frais. Les autres
changements CLI/Trust/web d'autrui restent conservés, non certifiés par ce lot.

core02 : compilation et test ordinary-work PASS, mais recette FAIL car le README
partagé des vérifications a changé pendant l'exécution (seule entrée existante
différente). Aucun octet Rust testé n'a changé dans ce delta. ELF02 conservé comme
non qualifié, intégrité du FAIL VERIFIED. Reprise sur inventaire incluant le
README actuel, sans supprimer ce contrôle ni relever une limite ; cache Cargo
réutilisable, pas de nouvelle compilation justifiée par ce seul changement doc.

core03 FAIL comportemental, sources stables, intégrité à vérifier : le nouveau
lecteur accepte un certificat public altéré d'une ancienne publication. Octets
restaurés avant assertion ; ELF rouge conservé. Contre-épreuve sur vraie CLI223
figée : replay de publication latest accepte ce même ancien certificat, alors
que work show refuse ; témoin valide/reprise et Work inchangé conservés dans
existing-reader-counterexample04. Cause : writer::current ne vérifie l'ancien
original que si la tâche demandée est ancienne. Réparation du contrat commun :
vérifier chaque publication antérieure, pas une exception au lecteur PAEA.
Le coût suit la chaîne conservée ; aucun plafond arbitraire/cap/timeout ajouté.
Méthode : édition désormais sous verrou public deploy/.local-build.lock pris
non bloquant, après terminalité de notre test ; ne jamais appeler swarm.sh tant
que ce verrou d'édition est détenu (ordre lifecycle puis build du lanceur). Ce
garde empêche notre édition pendant un build coopératif, pas celle d'un autre
éditeur non raccordé ; source-stability reste requise.

core04 PASS (2tests dont vrai cumulatif,6assertions et Rustdoc strict), intégrité
VERIFIED ; ELF qualifié04 figé avant nouvelle compilation. Amélioration71 : les
fautes certificat du test utilisaient write direct ; remplacées par staging
adjacent/fsync/rename et restauration avant assertions, avec témoin valide puis
ENOSPC injecté après écriture partielle temporaire et reprise. Aucune saturation
réelle provoquée. Production Rust inchangée depuis core04 ; requalification05
couvre cette amélioration de méthode et la documentation des consommateurs.

core05 PASS/VERIFIED : trois tests Rust nommés, six assertions cumulatives et
Rustdoc strict. ELF05 conservé ; production inchangée depuis le correctif04.
555Mo libres avant livraison :573393216octets de seuls rlib régénérables dans
notre cible PAEA retirés sous verrou Cargo, inventaire/taille/SHA conservés avant
suppression ; aucun ELF/preuve/source/depfile/cache partagé retiré.1,1Go libres
après. Profil normal et CLI d'effet historique seront vérifiés avant activation.

ENOSPC réel après préparation du build07, pendant mise à jour atomique du pilote
Python : original strictement intact (SHA identique à source-review-candidate).
Aucune étape dépendante lancée. Sous verrou Cargo de notre seule cible,515141490
octets supplémentaires de rlib de dépendances régénérables retirés après inventaire
SHA/taille conservé d'abord dans /dev/shm puis copié/fsync/relus dans la preuve.
Aucun ELF/source/depfile/preuve/cache normal partagé retiré.1,6Go libres après.
Reprise du pilote sur cette capacité rétablie ; --binary/--profile/--fixture/--output
exigés via CLI ou variables de recette explicites ; absence/vides refusent.
Nettoyage DB conserve désormais les deux causes via restoration. Champ non observé
git_ref_changed=False retiré : git_ref_effect_verified=False distingue la limite.

## Résultat local vérifié

Version **v0.1.1-dev.225** installée par le chemin source géré et le candidat
préparé exact. PID observé : 3323052 ; ELF réellement exécuté :
`bba13470359ddfb0fecc17b958935ccab77dbc74ec40253c853288acab1aa8f5`.
Commit réel CBD6… (empreinte complète dans active-status09.json), changements
non commités signalés ; aucun commit/push/délégation de ce lot.

Le lecteur commun de publication vérifie maintenant les originaux natifs de
**toutes** les publications antérieures. Le rejeu du dernier événement ne peut
plus contourner un certificat historique altéré. Le nouveau lecteur
`current_managed_git_publication` choisit le dernier candidat cumulatif et
retourne ses coordonnées vérifiées, sans sélection d'ancienne tâche ni autorité.

Trois tests Rust nommés PASS dans core05, dont le vrai cumulatif à six assertions
et la faute ENOSPC après écriture temporaire partielle ; Rustdoc strict PASS,
sources stables et intégrité VERIFIED. ELF effectivement exercé retenu dans
qualified-test05-* / test05-artifacts.json. La réparation de production était
identique dans core04 ; seul le garde de fixture et ses docs ont évolué ensuite.
Core02 reste FAIL de stabilité (README seul), core03 reste FAIL comportemental
avec sources stables ; originaux, inventaires et ELFs rouges conservés.

**Parcours public normal :** cli08 PASS/VERIFIED, trois assertions nommées :
ancien et dernier certificats altérés refusent le rejeu exact de la dernière
publication, restauration puis reprise réussies, lignes Work/événements/workers
et marqueur provider inchangés. Le binaire testé est exactement l'ELF installé.
Le contre-exemple CLI223 antérieur est conservé séparément ; work show refusait
l'ancien certificat alors que le rejeu latest l'acceptait. Aucun résultat rouge
n'a été réécrit. Aucune écriture positive SQL ni revue fabriquée : les deux
publications proviennent de la vraie fixture native inerte et de ses appels
publics, sans modèle payant, acceptation de mission ou approbation humaine réelle.

Build normal : 680 entrées effectives, 480 communes aux tests05 ; seule la note
version_history diffère, ajoutée après qualification. Features cœur vides et
profil sans fixtures ; graphe normal223→225 identique. Trois écarts transitifs
avec les tests inchangés, justification66/70 réutilisée explicitement. Les ELFs
de test et de livraison ne sont jamais assimilés.
Version10 : 56 assertions UX/Version et garde de script PASS/VERIFIED, 25 assets
protégés et notes comparés à l'inventaire compilé ; FR clair/EN sombre, commit,
non-commité, Escape/focus et captures inspectés. Les vues PAEA existantes restent
des lecteurs ; ce lot n'expose pas de nouveau service HTTP de promotion Git.

Revue politiques/fonctions/erreurs/docs liée aux octets ; documentation FR/EN et
recettes permanentes actualisées. Le coût de la vérification suit la chaîne
conservée sous les politiques Git existantes ; aucun plafond de chaîne ou délai
global ajouté. Le diff de nos chemins est propre ; le diff global reste exit2,
avec les mêmes deux lignes étrangères que70. Les changements CLI/Trust/web
concurrents ont été préservés et identifiés, pas certifiés sémantiquement par ce lot.
Le premier build concurrent refusé reste chez son propriétaire ; nous n'avons
ni arrêté ses processus ni réutilisé ce candidat refusé.

## Limites et prochaine action

`repository_request_digest` décrit le cadrage du dépôt géré ; ce n'est PAS une
association à `Action::Git.repository_id`. Propriétaire Codex/PA-03 : provisionner
explicitement le dépôt privé détenu et son identité de service, puis lier le
projet/instance et refuser tout mauvais dépôt ou pin. Revalider le dernier
candidat sous la barrière du service, pas à partir d'une observation périmée.

Raccorder ensuite les fondations65/69/70 : identité PAEA empruntée et permission
exacte courante, admission/rôles/Trust/plan sous réservations, intention durable
avant Git COMMIT, dépôt conservé et référence directe/OLD sous verrou, unicité
métier et réconciliation de la référence après rupture. Existing ou Uncertain
ne donnent jamais un droit de retenter. Ne pas réemployer un accord portant une
autre empreinte ni abaisser la séparation active faute d'admission attribuée.

PA-02 et PA-03 restent partiels ; invariants protégés démontrés : 0/17.
UID/humain/confinement/daemon/E2/E3/PA-13/RETEX final restent ouverts ; les demandes
administrateur et approbations indépendantes déjà pendantes restent telles quelles.
Ce résultat local ne clôt pas PAEA. Le lecteur est une observation, ni capacité
de dépôt, ni mandat, ni reçu terminal d'effet.

## Améliorations conservées, non réalisées

- Origine71, relecture du nouveau parcours : la vérification de l'historique
  complet coûte un parcours par appel à current ; des lectures multi-tâches
  peuvent répéter ce travail. Valeur attendue : diminuer les lectures Git sans
  réduire l'intégrité. Propriétaire Codex/lecteur natif. Suite : mesurer les appels
  sur une fixture cumulative de taille connue, comparer une réutilisation limitée
  au même snapshot Work/réglages/descripteurs ; conserver le refus sur ancien
  original altéré. Aucun cache inter-appels, seuil d'historique ou effet mesuré
  supplémentaire n'est revendiqué. Priorité après raccord sûr du service PA-03.
- Origine71/status60280 : status attend le verrou lifecycle pendant un build
  concurrent, alors que le serveur reste actif. Valeur attendue : diagnostic de
  statut disponible sans attente d'un build, sans promettre un état atomique faux.
  Propriétaire Codex/lanceur local. Suite : définir une observation non bloquante
  avec état busy/indéterminé explicite, puis tester sous verrou d'une fixture
  locale et pendant reprise ; pas de timeout arbitraire ni contournement du verrou
  des mutations. Pas réalisé ici, aucune durée totale précise inventée.
