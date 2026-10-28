# Reprise après71 — origine historique vérifiée, service Git encore à raccorder

Codex direct, sans GLM/délégation/commit/push. Preuve :
`docs/plans/actions-confiance/paea/evidence/20261028-current-git71-9e6mqvss`.
Vérifier l'heure, le dernier incrément, les processus et les sources effectives
avant toute action. Les numéros/PID ci-dessous ne sont pas un état futur garanti.

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

## Reprise pratique et apprentissages vérifiés

Tous handles71 sont terminaux : fixture01/38436, core02/17995 (FAIL README),
core03/98531 (FAIL historique), contre-exemple04/23821, core04/33383,
core05/72229, build07/53325, cli08/3820, restart09/89480, version10/23126.
La commande status/60280 attendait le verrou de livraison concurrent ; elle a
terminé, sans arrêter de processus étranger. Ne pas recréer de test déjà actif.

Relire `managed_publication/writer.rs::current` : tous les originaux antérieurs
sont désormais revalidés, pas seulement celui de la tâche demandée. La chaîne SQL
et la dernière opinion de cohorte ne suffisaient pas. Le nouveau
`managed_git_current.rs` expose la dernière observation, sans dépôt détenu ou droit
courant ; ne pas la transformer en booléen d'autorisation. Les versions223 et225
ont leurs preuves séparées. Anciennes qualifications concernées restent historiques.

Le modèle `ManagedRepository` n'a pas de repository_id de service. Lire ses vrais
champs dans models.rs, ne pas deviner une convention autorisante. Le service doit
lier explicitement ce dépôt à sa configuration privée et au projet, puis garder
les descripteurs jusqu'au point d'effet. Le journal70 reste pending-only : aucune
conclusion terminale sur un Git effect à partir de sa seule présence/absence.
`paea_settlement/policy.rs` porte le contrôle courant rôle effect/Trust/Biba et le
refus des séparations non attribuées ; comparer extraction commune préservant
codes/causes avec un adaptateur fermé, sans duplication ni faux succès. Une
modification du contrat partagé justifiera les13cas settlement pertinents.

`managed_repository.rs` prépare Git durci via la politique persistée ;
`local_storage.rs` contient LocalDirectory ; `RootBinding` retient l'instance.
Ne pas deviner de chemin à partir d'un nom de symbole. Vérifier la signature et
un appel existant complets, pas un extrait tronqué. Aucun corps de workflow/agent
embarqué n'a changé dans ce lot ; les sessions natives déjà lancées restent
séparées de la réparation du nouveau binaire. Mission native utilisateur préservée.

Concurrence : prendre non bloquant le verrou public deploy/.local-build.lock
pour nos éditions Rust et formatter avant de le rendre ; ne pas appeler swarm.sh
sous ce verrou (ordre du lanceur lifecycle puis build). Ce garde protège nos
écritures contre un build coopératif, pas celles d'un autre éditeur. Toujours
vérifier les inventaires de sources et les preuves du candidat après compilation.
Ne pas réutiliser le premier candidat concurrent refusé. Ne pas envoyer de message
à un autre chat sans autorisation ; ses processus/travaux restent préservés.

Disque : ENOSPC réel pris en charge, original Python atomique resté intact.
Deux inventaires de nettoyage des seuls rlib régénérables de notre cible PAEA
sont conservés dans cache-cleanup06-before.json et cache-cleanup06b-before.json,
avec les ELFs réellement exercés figés au préalable.
Ne toucher ni preuves, ni données, ni cache normal partagé d'autrui. Les fichiers
source étaient écrits via tempfile adjacent/fsync/replace ; ne pas rejouer un
script finaliseur/append déjà appliqué. Les rlib du target PAEA sont à régénérer
si un nouveau test le nécessite ; ne pas recompiler sans contrat changé.
Un inventaire de secours reste dans /dev/shm/paea71-cache-before-3xc5uur0.json ;
sa copie durable est cache-cleanup06b-before.json. Ne pas le confondre avec un original perdu.

Les réveils existants sont bornés au28oct2026 19h20Paris. Ne pas en créer d'autre,
ne pas engager de reprise programmée après cette échéance. API goal précédemment
paused distincte de l'autorisation de continuer par heartbeat ; ne pas prétendre
qu'un moteur d'objectif supervise. Admin UID/humain/approbations déjà pendants,
pas de nouvelle demande répétée ni de clôture globale inventée.

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
