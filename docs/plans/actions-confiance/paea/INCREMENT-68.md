# PAEA68 — protocole Git conditionnel sous verrou préparé

Reprise après67 installé/vérifié221. Objectif PA00…13/RETEX actif, Codex direct,
sansGLM/délégation/commit/push. Ce lot qualifie une précondition réelle du second
service PA03 ; il ne livre pas encore ce service ni une nouvelle version produit.

## Découverte et décision

Le probe01 sur Git2.51.0 a démontré que `update-ref --no-deref REF NEW OLD`
remplace une référence symbolique par une référence directe lorsque OLD correspond
à sa cible résolue. Aucun effet n'est publié dans le dépôt canonique : dépôt nu
jetable et objets synthétiques uniquement. Un contrôle préalable `symbolic-ref`
laisserait une course entre lecture et acquisition du verrou Git.

Options : supposer no-deref suffisant, réimplémenter les fichiers de références,
ou conserver le protocole transactionnel Git. Choix : `start`, `update` avec OLD,
`prepare`, puis lecture du type sous verrou détenu par Git et `commit` ou `abort`.
Aucun nouveau plafond/timeout/retry n'est décidé ; l'intégration native devra
consommer le `git_command_timeout` enregistré et le runner à propriété contrôlée.
Le test Python n'est pas un runner de production et ne prouve aucun confinement.

## Critères et suite

Six tests permanents : publication directe et abandon explicite ; contre-exemple
no-deref ; refus de référence symbolique sous verrou et reprise ; ancienOID périmé
et reprise ; écritures concurrentes directes/symboliques empêchées sous prepare ;
erreur de décision après prepare sans publication et reprise. Témoin valide avant
chaque faute. Git réellement résolu/version/hash conservés et stables ; six tests
nommés et aucun watchdog global. Pas de réseau, modèle, base mission ou commit au
checkout utilisateur. Les objets de fixture ne constituent pas une attribution.

À vérifier par recette puis intégrer au runner natif : parser les accusés exacts,
garder stdout/stderr/cause/cleanup et propriété enfant, annulation et échéance
monotone configurée. Une écriture de COMMIT peut publier avant échec du canal :
intention durable avant protocole et réconciliation de la référence après erreur
obligatoires. Le probe ne résout pas cette frontière distribuée.

Les candidats/revues réels existent dans managed_publication::verify_published_current
et ses originaux ; une chaîne publication SQL n'est pas une référence Git promue.
Le lecteur for-each-ref actuel du dépôt géré ne prouve pas le type direct. Réutiliser
la garde actuelle de candidat, pas un champ d'approbation fourni par la proposition.
L'admission PAEA reste distincte des accords Trust à autre empreinte.
PA02/03partiels,0sur17,E2/E3/PA13/RETEX restent ouverts.

## Résultat68 vérifié

core02 : six tests nommés PASS, intégrité VERIFIED, sources déclarées stables.
Git réel `/usr/bin/git`2.51.0, SHA256
`3446b26047255f16d588eda4527aeedf02bd173449382d93bfe8a141257876c9`.
Probe01 conserve le contre-exemple du no-deref seul ; le test permanent préserve
explicitement cette observation pour empêcher la réintroduction de cette hypothèse.
Aucun service de production corrigé ou installé dans68. Les677entrées effectives
221 sont inchangées après ces tests/docs ; pas de rebuild ni numéro artificiel.
Le runner Python de qualification n'est pas destiné au runtime et sa gestion
finally/pipe n'est pas une preuve de nettoyage natif ou de publication durable.

Suite native choisie : réutiliser run_controlled_command(input, deadline, sinks,
OwnedProcessControl). Le démarrage notifie après pidfd ; un sink stdout parse les
ACK exacts et transmet prepare au contrôleur, qui conserve l'entrée jusqu'à décision.
Sous verrou Git : contrôler type direct, OLD et admission courante. Intention SQL
durable avant toute possibilité de COMMIT ; après envoi, toute rupture conserve
UNKNOWN puis réconcilie le ref sous propriété retenue. L'erreur doit garder résultat,
causes indépendantes, cleanup, enfant nonrécolté et état de publication. Aucune
intégration de ce protocole ni autorité supplémentaire n'est déduite des six tests.

Ne pas créer un subprocess bloquant qui contourne les délais Admin, la capture,
la propriété pidfd et l'annulation du runner. Vérifier avant code les interfaces
réelles, dont OwnedProcessControl dans validation_run.rs (pas un fichier supposé
validation_control.rs). Les lectures exploratoires à chemins inexistants ont été
corrigées par rg ; ne pas reconduire cette erreur de méthode.
PA02/03/0sur17/E2/E3/PA13/RETEX restent ouverts. Aucun commit/push/délégation.

Complément : documentation explicite des méthodes de fixture/test affinée ;
6cas documented03 PASS/stable/intégritéVERIFIED, nouvelle revue/archive03.
Les originaux02 restent conservés et ne sont pas réécrits.
