# PAEA — incrément 50 : credentials Linux validés avant usage

Le correctif est installé et vérifié dans **v0.1.1-dev.180**. Il clôt ce défaut de représentation des credentials, pas PA-02 ni le programme PAEA. Codex travaille seul, sans GLM, délégation, commit ou push supplémentaire.

## Besoin et diagnostic

Pendant la préparation du canal attribué, le [signalement primaire Rustix](https://github.com/bytecodealliance/rustix/issues/1698) et la source installée 1.1.5 ont révélé un contrat incompatible : `UCred.pid` utilise un PID non nul, alors que Linux peut renvoyer zéro quand le créateur de connexion est invisible dans le namespace du lecteur. Un témoin connecté valide précède chaque injection. Le cas réel utilise des namespaces user/PID/mount éphémères et un parent immédiat visible ; il ne modifie aucun compte, sysctl ou namespace de l’hôte.

L’ancien ELF 175 refusait les trois gates avant tout grant ou Root, mais affichait un faux `os-errno:0`. Aucune exploitation réussie n’a été observée. Les originaux sont conservés dans `raw-zero-peer-result.json` et `gate-before-fix.json` du dossier de preuve.

## Correction et limites

Le getter commun lit les entiers bruts de `nix::UnixCredentials`, refuse PID non positif et UID/GID sentinelles, puis laisse les vérifications existantes de parent, UID, PDEATH, grant, source, fraîcheur et droits s’appliquer. Les trois gates de production et le helper SDK de revue réservé à `cfg(test)` l’utilisent. Une erreur syscall conserve son véritable errno ; une observation invalide conserve sa catégorie sûre, sans valeur privée ni errno inventé.

Choix retenu face à une mise à niveau ou un fork de dépendance : utiliser le getter brut existant de Nix 0.30.1. La feature `socket` ajoute `memoffset` 0.9.1 et `autocfg` 1.5.1 ; aucune version existante du lockfile n’a changé. Aucun `unsafe`, droit, politique opérateur, timeout ou retry n’est ajouté. SO_PEERCRED reste une identité de connexion ; il ne remplace pas SCM_CREDENTIALS/SCM_PIDFD ni une admission du pair par message.

## Preuves conservées

Dossier : `evidence/20261027-zero-peer50-7n84xnp1/`.

| Vérification | Résultat et portée |
| --- | --- |
| Bibliothèque `recipe-01` | Sources stables ; 4 tests moteur et 6 tests CLI PASS. Recette globale FAIL sur quatre diagnostics Rustdoc existants, conservés. |
| Documentation `rustdoc-02` | PASS, sources stables, intégrité vérifiée, `-D warnings`. Les deux réparations ne modifient que des commentaires ; les corps restent identiques. |
| Profil normal | 581 entrées effectives archivées et vérifiées, sans feature fixture ; 10 entrées de production propres liées aux octets. Le helper `cfg(test)` est correctement absent de cet inventaire. |
| Binaire installé | PID 4016445 à la clôture, SHA256 `e274363a532af0a0fff6147efb099a1aff736f07593fc30d9ee63fef43168b91`, build/restart pris en charge. |
| Parcours sur ce même ELF | 5 parcours personnels / 56 commandes ; 3 témoins puis 3 refus PID invisible ; 56 contrôles UI FR/EN et garde de script défaillant. PASS, sources stables et intégrités vérifiées. |
| Lecture Markdown | Une clôture Mermaid anglaise héritée était en ligne avec le texte. Le lecteur CommonMark reconnaît désormais le diagramme et le titre suivant ; original conservé. |

Les refus initiaux restent distincts : dépendance non disponible hors ligne avant fetch ; diagnostics Rustdoc ; comparaison d’archive ignorant les liens physiques ; build partagé anticipant quatre API non encore écrites ; collector exigeant à tort une source `cfg(test)` dans le profil livré. Chaque reprise conserve sa cause et sa précondition modifiée. Le retrait du seul cache régénérable possédé a eu lieu après archivage et relecture des 86 exécutables ; aucun cache partagé, original de preuve, source ou donnée utilisateur n’a été supprimé.

## Reprise et critères ouverts

Retour au canal attribué et au client lié. Les wrappers installés ne couvrent pas encore SO_PASSPIDFD/SCM_PIDFD ; l’option retenue doit préserver la propriété des FD et traiter les refus/troncatures sans fuite. Aucun fallback vers PID déclaré ou créateur de connexion n’est admis.

Les helpers `newuidmap`/`newgidmap` restent absents et demandent une installation administrative locale. Le parcours SDK fonctionnel complet, l’enrôlement humain, le daemon personnel, la séparation OS, E2/E3, PA-13 et **0/17 invariants globaux** restent ouverts. Les trois anciens reçus de migration manquants dans le README gardent leur propriétaire et leur condition de reprise ; nos liens PAEA sont contrôlés séparément. La communication dépassant parfois 60 secondes reste un défaut de méthode ouvert, sans prétendre qu’une consigne l’a corrigé. Revue locale, pas approbation indépendante.
