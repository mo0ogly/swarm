# PAEA — incrément58 : domaine retenu et dispatch du paquet réel

Reprise57 installée/vérifiée dev205 ; Codex seul, aucun agent/GLM/commit/push.
APEX/replan. Besoin PA-02 : raccorder le paquet réel au dispatcher existant sans
perdre la garde des identités noyau ni les erreurs après effet. Les helpers
newuidmap/newgidmap sont toujours absents ; succès métier sous deux UID et
approbation indépendante du domaine restent des critères ouverts.

Deux routes : adopter le namespace du premier appelant, ou recevoir des handles
attendus déjà conservés par le lanceur protégé. Retenir la seconde : comparer
objets montage/PID sous FD retenus, jamais chiffres de namespace fournis par la
requête ni lecture /proc/PID par son numéro. L’appelant doit fournir cette attente
depuis une source approuvée indépendante ; aucune première observation n’est
promue en autorité par cette tranche. Pas de nouvelle politique persistante,
listener, taille limite, délai ou tentative. Le même domaine n’est pas une preuve
d’image approuvée, confinement, humain ou droits : le contrôle existant garde
ses vérifications métier courantes.

Ordre : UID réel du paquet versus bootstrap retenu ; namespaces réels par pidfd ;
comparaison aux deux objets attendus ; décodage sans copie ; target et dispatch
existant ; fermeture des handles observés après usage. Packet/attente restent
empruntés et vivants chez l’appelant, qui possède leurs fermetures. Si fermeture
échoue après dispatch, erreur conserve Reply et cause ; aucun succès vide ni
inférence de rollback. Refus avant dispatch n’invente pas de Reply.

Vérification : tests noyau exacts et refus de domaine avec témoins/reprise ; tests
privés ordre/compteur/décodage/effets connus ; vraie route Service refus mauvais
UID et état intact ; régressions lifecycle/transport ; Rustdoc strict et recette
pérenne ; revue politiques/fonctions/erreurs et livraison normale/Version FR/EN.
Une réussite du callback synthétique n’est pas celle d’un service sous UID distinct.
Propriétaire Codex ; PA-02partiel/0sur17/E2/E3/PA13/RETEX restent ouverts.

Noyau vérifié :8nouveaux cas,41anciens cas Rust distincts (49total ciblé),3méthodes schéma ; Rustdoc strict3sources/4pages et intégrités PASS. Fermeture OS défaillante et succès métier deux UID non démontrés. Construction/livraison locale en cours.

Découverte liée à PA-02 : les ioctls PIDFD_GET_*_NAMESPACE appliquent un contrôle ptrace READ_FSCREDS dans le [code noyau primaire](https://kernel.googlesource.com/pub/scm/linux/kernel/git/torvalds/linux/%2B/0d508f1745b57272d4d4a3d50dc35aa2a3682238/fs/pidfs.c). Notre noyau est6.17.0-41, UID1001 sans capacité effective ; la source distante ne prouve pas ses backports exacts. Inférence à qualifier : deux UID distincts exigent aussi une permission d’inspection adaptée, ou une capacité de processus fournie par le lanceur protégé. Aucun relâchement de gate, changement ptrace/capacité hôte ou succès distribué déduit. Options/propriétaires/reprise dans followup-cross-uid-namespace.json ; refus58 conservé, qualification de lancement suivante.


## Livraison locale vérifiée — tranche58

Première qualification **dev206**, ELF
`4a72de36ffaaabc99535a1734e1586212855e256adb33ebfcefefae54c8a7c35`.
Instance courante **dev207**, ELF
`bd44dd4c806a0666aade0f2408eb02dbe0d6842ea437b7011691d75f3f39b385`.
Les deux profils normaux636entrées/65PAEA n’ont aucune feature test-fixtures.
Le seul changement d’entrée effective est web/native-runtime-health.js, étranger
à la tranche ; tout Rust/Cargo/PAEA reste identique. Recettes appel/personnel206
conservées comme résultats206 ; UI/Version/peer207 vérifiés fraîchement. PID1348314 au contrôle natif ; identité finale dans
closure-integrity.json. Version FR/EN vue, notes/commit réel/modified et assets
protégés contrôlés. Le nouveau raccord est une bibliothèque compilée dans ce
profil ; les parcours natifs existants ne prétendent pas exercer un nouveau daemon.

| Portée | Preuve finale | Résultat |
| --- | --- | --- |
| Deux objets retenus, vrais namespaces différents, reprise | fixture-causes17 public2 | PASS |
| Substitution indépendante montage/PID, ordre UID/domaine/décodage, Reply et causes | final-private18 privé1+6 | PASS |
| Route Service réelle mauvais UID, checkpoint intact et reopen | regressions05 service22 inclut1nouveau | PASS |
| Régressions distinctes |21autres Service+1pending fixture+10startup+9canal ;3méthodes schéma | PASS |
| Rustdoc strict et octets/prose | doc04 3sources/4pages ; shared-doc16 6sources/6pages ;19HTML archivés | PASS |
| Appel/personnel206 ; interface/canal courant207 | call10 5parcours/56commandes ; personal11 7+3gardes sur206 ; current-peer22 3paires/6commandes et current-ui21 56contrôles+garde sur207 | PASS, ELF séparés |

Les51cas Rust distincts sont10nouveaux+41antérieurs, pas un dénominateur global.
15dossiers de recette ont leur intégrité vérifiée après résultat final. Sources
et revue politiques/fonctions/erreurs liées aux octets dans source-review.json,
reviewed58-sources.tar.gz et closed-local-result.json, sous
`evidence/20261027-dispatch58-igie_85k/`. Revue Codex, aucune revue indépendante.

L’échec de build06 reste conservé : source étrangère changée pendant compilation,
qualification refusée avant installation. Candidat suivant07 qualifié réutilisé
après vérification65octets/notes/profil, puis restart08 pris en charge. Diagnostic
initial trop large comparait2917candidats à636entrées effectives : projection
corrigée à leur intersection, un seul vrai changement mission_status_runtime.rs,
sans réécrire le diagnostic original. Le vérificateur documentaire a refusé une
page CLI périmée en15 ;16 la régénère avant de revérifier, sans relâcher l’égalité.
Le compteur hérité6/6 est désormais dérivé des listes ;3/4 et6/6 vérifiés.

La revue a aussi ajouté un vrai autre domaine devant le dispatcher et conservé
les causes typées des fixtures jusqu’à libération de l’enfant. Aucun échec OS
simultané ni close réellement défaillant injectés : agrégation synthétique et
source actuelle seulement. Les callbacks positifs restent synthétiques. Les
preuves plus anciennes sont réutilisées uniquement pour leurs comportements
consommés inchangés ; leurs inventaires entiers ne sont pas déclarés finaux.

Suite : résoudre le contrat de lancement à moindre privilège, qualifier permission
d’inspection inter-UID puis raccordement consommateur/daemon et PA-03. Le fichier
followup-cross-uid-namespace.json conserve la découverte liée aux lots51/52/58,
options, propriétaire et reprise. Approbation du domaine/humain, helpers uidmap,
vrais UID séparés, confinement, E2/E3/PA13 et RETEX restent ouverts. PA-02partiel,
0sur17globaux. Aucun commit/push/GLM/agent, mission ou configuration hôte modifiés.
Les défauts de méthode (spéculations, chemins supposés, sortie trop large,
collision historique du backup) restent ouverts, sans réparation globale annoncée.
