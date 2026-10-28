# PAEA — incrément59 : contrôleur créé et sujet noyau retenu

Reprise58 vérifiée, courant207 ; Codex seul sans agent/GLM/commit/push. PA-02 et
0sur17 globaux ouverts. APEX/replan. UIDmap administrateur toujours absent.

La source51/58 observe le sujet kernel et ses namespaces, sans lier ce sujet à
un processus choisi indépendamment par le lanceur. Comparer UID/PID numérique
ou créateur de connexion ne prouve pas cette liaison (PID réutilisable, sender
pouvant différer du créateur). Choix : pidfd du Child réellement créé, conservé
sous FD ; capture avant requête et sans adoption du premier paquet. Child doit
rester sans reaper externe pendant try_wait/pidfd_open ; try_wait peut observer
et récolter sa fin, jamais signaler/arrêter/recommencer le processus.

ProcessAnchor capture un Child existant nonrécolté, vérifie pidfs et identité
inode64, observation immédiate de terminaison par poll zéro (sondage technique,
pas délai de test ni politique d’exécution), puis compare le pidfd du paquet.
Ancien anon_inode et représentation32 sans génération refusent, sans fallback
PID. Aucun mandat, image approuvée, humain, confinement, gel de namespace ou bail
ne sont déduits. SCM_CREDENTIALS privilégiés restent dans le modèle de confiance OS.

Sources primaires : [identifiant pidfd systemd](https://www.freedesktop.org/software/systemd/man/261/sd_pidfd_get_inode_id.html),
[pidfs Linux](https://linux.googlesource.com/linux/kernel/git/dhowells/linux-fs/%2B/2cc699b3c2fe5ea84081059c88c46373cbebe630/fs/pidfs.c).
Sur64bits, pidfs expose l’identité stable via inode ; sur32bits, il faut aussi
la génération (route nonimplémentée ici). Source de magie Linux : header installé
/usr/include/linux/magic.h PID_FS_MAGIC, absent de libc verrouillée0.2.186.
Cette représentation n’est pas une politique opérateur. Nix poll déclaré dans
la dépendance normale ; pas de feature de test supposée transitive.

Critères : vrai Child/paquet lié avant message, autre processus mêmeUID et même
namespace refusé puis enfant attendu de nouveau accepté ; fin/récolte refusées ;
nonpidfs rejeté, fermeture et causes retenues. Nouveau dispatch strict exige
UID bootstrap, ancre Child, puis TOUS les contrôles namespace58/décodage/cible et
contrôle métier existants. Garder les handles pendant usage ; erreurs/Reply/close
conservés. Aucun consommateur daemon ni succès métier deux UID réel implicites.

Propriétaire Codex. Source/recettes/docs/docstrings FR/EN et revue exacte des
politiques/fonctions/erreurs ; tests nommés, profil normal et livraison locale.
Le prérequis ptrace inter-UID58 reste ouvert, sans baisse de protection ni
privilège hôte ajouté. Après cela, consommateur/lancement puis PA-03 et RETEX.

Noyau courant vérifié : final-core06 PASS12Rust (6nouveaux+6antérieurs),2Rustdoc stricts et exact4sources/6pages ; regressions07 PASS15autres cas.27Rust distincts au total de cette tranche, jamais pourcentage global. core04 conserve son FAIL final d’attendu typographique Rustdoc, tous ses cas Rust et compilations avaient passé. supplement05 corrige uniquement la phrase rendue et vérifie la fermeture via compteur FD. Livraison locale encore à qualifier.


## Livraison locale vérifiée, lancement encore conditionnel

Dev208, PID1530170 au contrôle natif, ELF
`06592796d06752f478e03a82ac17d6c95c6ad177a364f321afa74dc79d1aaf90`.
Profil normal637entrées/65PAEA, sans test-fixtures, tests cfg(test) exclus.
Même ELF réel pour call12(5parcours/56commandes), personal13(7+3gardes),
peer14(3paires/6commandes) et ui15(56contrôlesFR/EN+refus script), tous PASS.
Version/notes/commit réel/modified/assets protégés et diagrammes FR/EN vus.
27Rust distincts =6nouveaux+21antérieurs : final-core06 12, regressions07 15,
tous PASS. Rustdoc strict2compilations, exact4sources/6pages,10HTML archivés.
11intégrités vérifiées :8PASS conservés et3FAIL originaux intacts, jamais PASS
fabriqués. Les FAIL d’API/méthode absentes et d’apostrophe typographique restent
historiques ;06 et07 actuels sont PASS. Politiques/fonctions/erreurs liées aux
octets dans source-review.json et reviewed59-sources.tar.gz ; résultat local
closed-local-result.json, sous evidence/20261027-spawn59-o0mma_ab/.

Build08 a consommé un raccord partagé SqlTransaction incomplet (Cargo JSON
E0599 conservé). Producteur présent/hash changé à la reprise, sans modification
étrangère par cette tâche : build09 puis candidat10 et restart11 pris en charge.
Pas d’after.json après Cargo échoué : première supposition diagnostique refusée,
puis traitement par phase, aucun inventaire après compilation inventé.

La capture Child59 n’est pas atomique : garde contre auto-récolte/autres waiters
à établir par le lanceur, pas prouvée par Child. Aucun daemon, vrai succès métier
deux UID, humain/confinement ou acceptation indépendante. Callbacks positifs
synthétiques ; liaison Child/paquet et namespaces réels. Pas d’injection OS32,
ancien pidfs ou échec fstat/poll/close prouvée. Garde de lancement ouverte.

Suite : followup-atomic-launch.json. Sonde réelle rustc1.96 create_pidfd E0658,
pas de nightly ; GNUglibc2.43 expose pidfd_spawn (symbole seulement, aucun appel).
Lire contrat/headers/ABI puis vérifier création atomique et propriété du processus
à chaque échec avant adoption, sans baisser58/59. Ensuite consommateur/PA03.
uidmap administrateur toujours attendu, ptrace inter-UID58 ouvert. PA-02partiel,
0sur17/E2/E3/PA13/RETEX ouverts. Aucun commit/push/GLM/agent, mission ou réglage
hôte modifiés. La première création du script de clôture a refusé une ligne
de patch mal préfixée, sans fichier créé ; script corrigé avant toute exécution.
