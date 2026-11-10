# PAEA31 — documents privés atomiques pour le coordinateur personnel

## Reprise et décision

30a39tests/7étapes PASS, intégrité VERIFIED, Rustdoc et Mermaid FR/EN vérifiés.
Le build normal reste actif sous pression mémoire/swap ; ne pas modifier ses
sources effectives pendant sa compilation. Le suivi actif doit rester borné,
avec un point utile au moins toutes les60secondes, distinct du heartbeat de nuit.
Le codec30fournit des octets exacts mais aucun fichier protégé ni fsync/checkpoint.

Le Store de configuration possède déjà le parcours Unix sûr : dossier privé
retenu par FD, modes0700/0600 et single-link, NOFOLLOW, temporaire même répertoire,
write+fsync+close puis rename et fsync du dossier ; causes secondaires conservées.
Choix : extraire ces seules opérations documentaires communes par responsabilité,
puis les réutiliser dans un Documents générique privé, sans faux Pins/configuration
ni copie indépendante de la machine d'autorité. Garder l'ancien comportement et
ses hooks fixture. Ne pas élargir vers un refactor général du Store/Coordinator.

31a : fonctions partagées read/persist/sync à partir du vrai FD et owner ; Store
existant les délègue sans modification de son autorisation ou de ses fichiers.
31b : Documents ouvre un dossier existant protégé, garde FD et lockfile propres,
réservation mutex/flock immédiate, noms simples validés, read/create/replace,
fermeture explicite et erreurs de cleanup retenues. Aucune source d'identité ou
autorité n'est inférée du dossier. Les remplacements attendus lisent/valident
l'existant ; aucun refus ne devient un document vide ou une réinitialisation.
31c : témoin valide avant défauts ; liens/types/modes/hardlinks, chemin remplacé,
contention et reprise, annulation, erreurs write/partial/ENOSPC/sync/close/unlock.
Un échec avant publication doit préserver l'original ; après rename/erreur, état
visible/durable inconnu conservé, pas de promesse de non-mutation. Injection sur
fixtures isolées seulement, guards absents du profil normal. Régression Store et
Coordinator parce que le parcours physique partagé change ; pas de suite globale.
31d : contrats des fonctions, docs FR/EN, recette et revue liée aux octets. Aucun
plafond/délai/retry opérateur nouveau. Les modes, noms de lock et nonces de temp
sont des invariants de stockage, pas une permission humaine ou du confinement.

Suite : coordinateur personnel avec checkpoint indépendant et intent signé durable
avant application exacte du lot, puis acquittement/checkpoint et reprise après
crash sans reconstitution depuis la seule base. L'authentification lit le checkpoint
privé sous réservation puis relâche ce verrou avant la réservation SQLite courante,
qui garde l'autorité pendant le callback ; refus de lecture/cleanup avant callback.
Cela évite un verrou FS gardé sur un effet externe et l'inversion compte→témoin.
Service/transport/Admin/UX, compte OS séparé et confinement suivent. Codex propriétaire,
enrôlement humain et évaluation indépendante non inventés. Aucun commit/push.


## Progression réelle

30est installé/qualifié dev128 (39tests,6CLI,56UI+garde).31 :8tests initiaux PASS.
Revue découvre le risque de substitution du nom du verrou dans l'ancien Store ;
reproduction distincte puis correction au contrat commun, pas exception Documents.
Ajout d'un vrai refus NOREPLACE et d'un Drop avec échec d'unlock sans panique.
Recette attend43tests/5étapes ; qualification/livraison restent à établir.


## Résultats et livraison31

43tests nommés/5étapes PASS, intégrité VERIFIED ;9documents privés,1NOREPLACE
réel,15Store,9Coordinator et9control. Contre-exemple de substitution de verrou :
1test échoue avant correction (`lock-red.*`), passe après ; ancien résultat
conservé. L'absence initiale d'API est un refus de compilation, aucun scénario
n'y est compté. Le premier lot8tests PASS reste conservé avant renforcement.
Rustdoc config PASS avec refus des liens internes cassés. Revue locale liée aux
13fichiers, source-review.json ; pas de revue indépendante revendiquée.

Build normal `swarm.sh` dev129 sans fixture,497sources effectives archivées et
relues, dépendances effectives stables.6tests du harnais CLI figé PASS (5processus
sur le candidat exact ;1témoin bibliothèque ancien, distinct des43tests actuels).
Redémarrage pris en charge réussi, version `v0.1.1-dev.129`, SHA256
`a5772a1b1d6f4fb6103aca46d97c5cee70d23979d094ec2d5b40a2806f97db98`,
commit cbd6fecd7e40016d6c259296ac3a21373a4bbd60, modifications déclarées.
56assertions UI protégées en lecture seule et refus du script absent PASS ;
Version FR/EN, notes, lien commit et thèmes état/sombre vérifiés, captures relues.
[Preuves31](evidence/20261026-private-files31-s4jplcta/).

Les fonctions de fichiers sont internes, sans autorité humaine ou service exposé.
Le contrôle d'inode prévient le domaine de verrou divergent observé ; il ne constitue
pas une cage OS et ne défend pas contre un même UID non coopératif/root. Apprentissage
transmis dans les docs FR/EN et recette permanente, aussi au Store antérieur ;
aucune réécriture des preuves historiques. PA-02partiel,0/17invariants globaux ;
confinement/E2/E3/enrôlement humain/PA13 restent ouverts. Suite32 : intent durable
avant consommation, checkpoint indépendant et reprise exacte. Aucun commit/push.
