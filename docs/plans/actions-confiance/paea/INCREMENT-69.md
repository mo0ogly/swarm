# PAEA69 — protocole Git natif détenu

Reprise221/68, objectifPA00…13/RETEX actif. Codex direct sansGLM/délégation/commit/push.
Les six essais Git68 prouvent les primitives, pas le runner natif. Choix : composer
run_controlled_command avec stdin nonbloquante et parser ACK, plutôt qu'un nouveau
spawn/wait bloquant. Ce lot réalise la primitive de protocole ; service d'effet,
intention durable, dépôt retenu et admission restent obligations du futur appelant.
Une API de protocole n'accorde aucun droit PAEA et n'atteste pas le Git fourni.

Critères : Git réel direct/symbolique/OLD, abandon et erreurs de décision, annulation
réveillable, échéance fournie avant COMMIT, ACK manquant/après COMMIT incertain,
nettoyage et propriété enfant conservés, matrice de wrappers. Tester le chemin
normal avant chaque faute ; pas de watchdog global ou plafond opérateur nouveau.
Code/documentation FR/EN et erreurs relues aux octets, profil sansfixture, build/
installation locale par chemin pris en charge si cette bibliothèque est qualifiée.
PA02/03partiels,0sur17/UID/humain/confinement/E2/E3/PA13/RETEX ouverts.

core01 FAIL compilation : find avait une durée de vie de retour ambiguë. Cause
établie dans le diagnosticRust, corrigée par liaison explicite à l'erreur empruntée
comme dans le consommateur66. Aucun test n'avait couru. Warning control inutilisé
dans notre adaptateur66 retiré (import seul) ; warning verify_candidate dans le
module partagé étranger conservé. Reprise sur une précondition réellement modifiée.

core02 sixcas/RustdocPASS, avant le nouveaucas d'annulation sous prepare.
core03 FAIL/incomplet : sixcasPASS, septième boucle CPU sur Interrupted persistant
rejoué par Write::write_all ; historique/ELF/PID/statut/enfant conservés, SIGTERM
sur seultest identifié parpidfd. L'enfantGit a quitté et disparu après fermeture
stdin, aucun processus étranger touché. Intégrité core03 VERIFIED/FAIL conservé.
Correction à la frontière Write : erreur Interrupted après état consommé devient
Other avec cause IO originale, afin de ne pas rejouer un ACK. Pas de timeout ajouté.
ELF02 non figé avant remplacement03 : ne pas lui attribuer le hash03. Qualification
actuelle sera refaite sur7cas et son propre ELF retenu avant autre compilation.

## Résultat local vérifié

Huit tests natifs distincts et Rustdoc strict PASS/core05 ; sources stables,
intégrité VERIFIED. ELF de test réellement exercé figé séparément :
`afd02f37f6406db41c12f3df3c03a12bdcfad36e48baa533b2a7ddeb0180b03c`.
Le témoin de durée d'une seconde a publié en660352210ns ; la faute attend seulement
l'échéance réellement fournie. Ce n'est ni un watchdog ni un objectif produit.
Le cas annulation sous prepare qui bouclait/core03 termine maintenant et conserve
le processus récolté, OLD et le verrou Git résiduel. Le runtime ne supprime pas
ce verrou ; seule la fixture jetable vérifie l'inode retenu puis le nettoie pour
observer sa reprise. Aucune récupération de service protégé n'est revendiquée.

Source-managed **v0.1.1-dev.222** installée par restart --prepared-build, PID3014575
observé, ELF `eee53fe5325b220f329a9f95196624f67ed3801ef12bd2384197ae8409115bbf`.
Profil normal678entrées,478communes aux tests05 : seule version_history diffère,
notes69 ajoutées après le gel de la preuve. Bibliothèques core sans features et
test=false, aucun featurefixture. Graphes normaux221/222 identiques ; trois écarts
transitifs test/normal inchangés examinés dans la preuve66 réutilisée explicitement.

56contrôles Version/UX et garde de script PASS/version08, intégrité VERIFIED ;
25assets protégés et notes comparés à l'inventaire compilé, FRclair/ENsombre,
commit réel/noncommité/focus et captures inspectés. Ces vues existantes n'appellent
pas la nouvelle primitive. Son corps générique est instancié dans les tests ;
aucun consommateur public installé ne le déclenche encore. Source/profile et
livraison de la bibliothèque restent distincts d'un effet Git dans l'ELF serveur.
Aucun nouveau daemon, service PAEA, paiement ou parcours de publication Git activé.

La revue source-review.json lie politiques, fonctions, erreurs, docs et limites
aux octets. related-sinks-review.json réexamine huit fichiers après la découverte
Interrupted ; aucun retour persistant semblable après consommation n'a été trouvé
dans les sinks examinés. Ce n'est pas un audit exhaustif ni une conformité globale.
La recette permanente reproduit les cas. Originaux core01/02/03/04 conservés ;
core03 reste FAIL/incomplet et ne devient jamais un PASS. Aucun commit/push.

## Suite du besoin initial

PA-03 : implémenter le service Git avec stockage détenu distinct, intention SQL
durable avant COMMIT, vrai candidat/revue/cohorte existants, CAS direct, unicité
métier et réconciliation après rupture. Les appels actuels de publication SQL ne
mettent pas à jour une référence Git et ne valent pas admission PAEA. Ne pas
réutiliser une approbation Trust liée à une autre empreinte comme accord exact.

Amélioration conservée pour ce raccord : le contrôleur privé69 reçoit le PID réel
après pidfd, mais ne l'expose pas au consommateur avant son préambule. Si le futur
job durable doit enregistrer l'enfant avant toute entrée, comparer un hook interne
started et un contexte préparé emprunté ; dériver l'identité du runner, jamais de
la proposition. Propriétaire Codex/PA03 ; critère : réception du PID réel sous
propriété retenue, décision/refus persistés avant ouverture de la barrière, reprise
sans nouveau Git. Pas réalisé ni nécessairement exigible sans le contrat du job.
Cette décision appartient au raccord, pas à une mutation ad hoc de la bibliothèque.

PA02/03partiels,0sur17/UID/humain/confinement/daemon/E2/E3/PA13/RETEX ouverts.

Finalisation : compteur Rustdoc supposé6 refusé ; catalogue réel4types publics
et1fonction=5 inspecté et empreintes retenues. Script initial et cause conservés,
seuls artefacts manquants publiés ensuite ; aucun append ni archive antérieurs
rejoués. Les huit tests et la livraison restent ceux des preuves originales.
