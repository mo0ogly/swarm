# Biba transversal — APEX, reprise du28octobre2026

Propriétaire : Codex. Demande : terminer la couverture Biba restante avec APEX.
Le noyau trust/G01–G05, la gouvernance et les recettes existantes sont conservés.
Aucune acceptation générale, activation de compte ou mission native autorisée par ce plan.

## État observé et décision

`swarm-biba-linux` fournit un profil Landlock/seccomp I0, mais aucun appelant moteur ne le consomme. Les quatre lanceurs CLI convergent sur `native_provider_exec::PreparedExec` : agent_execution_gate, planning_native, reviewer_native/gate et preparation_call_cli. Leur canal privé vérifie des droits ; il ne confine pas les écritures du fournisseur. HTTP/SSE, contrôles, mémoire et publication exigent leur propre inventaire avant qualification.

Deux voies : ajouter des contrôles aux outils déclarés seulement, ou médiatiser les effets réels au niveau commun des processus puis les promotions. La première laisse le code du fournisseur contourner les outils. Décision : frontière commune, refus sans repli et objets candidats séparés des objets officiels. Un nom de crate ou une déclaration I0 ne prouve aucune protection active.

Préconditions à résoudre : le profil existant accepte un chemin de workspace ; seule une racine I0 possédée et attribuée par le moteur peut devenir inscriptible, jamais un répertoire officiel choisi arbitrairement. Les worktrees Git partagés, caches fournisseurs, copies et descripteurs demandent des témoins réels. Un simple raccord casserait les parcours existants sans démontrer leur remplacement sûr.

## Exigences et ordre

| ID | Parcours et contrat observable | Vérification / reprise |
|---|---|---|
| BT01 | Inventaire des accès et reproduction publique d'un fournisseur écrivant l'autorité privée ; témoin valide avant attaque, fixture seulement | Origins/argv/image et état avant/après ; aucune base vivante ni matière réelle |
| BT02 | Exécution locale : contexte I0 possédé, accès privés et écritures officielles refusés, sortie candidate autorisée ; agent/planning/reviewer/préparation passent par la frontière commune | Refus sans exec non confiné, perte de parent, reprise, erreurs et nettoyage ; disponibilité négative explicite |
| BT03 | Contrôles et autres commandes provenant de données I0 : inventorier tous les lanceurs et fermer les contournements | Un contrôle ou enfant du fournisseur ne doit pas récupérer des droits I3 ; aucune commande réelle hors fixture |
| BT04 | Mémoire, imports/restaurations, propositions et effets : labels attribués par les producteurs et promotions déterministes liées aux octets | Ancien résultat, révocation, mutation, acteur ou champ client ne confèrent pas un niveau supérieur |
| BT05 | API/CLI/Admin, FR/EN/Mermaid, profil livré, recette permanente et revue applicable | Recette bout en bout, sources figées, installation locale identifiée, revue distincte puis RETEX ; critères non vérifiés ouverts |

BT01 précède le choix final de BT02 ; BT02/BT03 précèdent une qualification des promotions BT04. Les travaux mémoire et Git concurrents sont préservés ; ne pas éditer leurs fichiers partagés ni construire sur des intrants changeants. Les droits Linux ne sont pas augmentés ; Landlock/seccomp ne réduisent que les droits du helper possédé.

## Méthode et preuves

Zone dédiée : docs/plans/actions-confiance/verification-biba-transverse-20261028.
Ne pas répéter les campagnes historiques inchangées. RisqueA : revue politiques/fonctions/erreurs/docs liée aux octets, cas de refus/reprise, profil production séparé des fixtures ; les avis ne valent pas acceptation indépendante globale. Toute nouvelle politique opérateur doit être persistée et exposée Admin/API/CLI avant consommation ; les invariants Biba ne sont pas un interrupteur implicite de contournement.

Statut : analyse des producteurs réalisée ; BT01 à reproduire. Aucun correctif produit encore revendiqué.

## BT01 observé — stockage privé accessible

private-root-before/result.json conserve une imageSHA9d3346aebb56f76fdb1d58415475aef500e7f569d01258081afcb00f8e7fb272. Un témoin modèle valide précède le fournisseur de sonde : lecture privée, ouverture O_RDWR de state.db sans écriture et création de canari sous .swarm réussissent. Aucune base réelle touchée ni acceptation de tâche.

BT02 décidé : raccord obligatoire du profil existant à PreparedExec::execute(root), consommé par les quatre helpers après leurs contrôles actuels. Aucun mode non confiné en repli. Les workspaces chevauchant la racine privée/autorité refusent ; la migration des parcours encore fondés sur un cwd non isolé et les témoins positifs doivent être traités avant qualification/install. La seule fermeture négative ne clôt pas le parcours.

Écritures prévues : nouveau native_provider_biba.rs ; transport native_provider_exec.rs ; appels agent_execution_gate, planning_native, reviewer_native/gate, preparation_call_cli ; dépendance locale Cargo.toml/lock ; documentation FR/EN et recettes. planning_native comporte déjà une modification concurrente : ajout de l’argument seulement, conservation du reste et refus d’écraser un changement observé. lib.rs/root_store.rs/validation_run.rs/PAEA restent sous leur propriétaire actuel. Compilation et livraison exigent un inventaire stable.

### Observation de compatibilité — chantier migration hors PAEA

Le28octobre, candidat238 SHA7ca15168f4a9d69347b0effe1ee02fecbf9355126216603cfdf419868996962a
non installé : recette mission-manual-automatic-controls refuse dès le premier
planning step de setup, gate biba-workspace-overlaps-root (errno22, SR1 vérifié).
Preuves originales : .cache/migration-next-20261028/watch-controls-review238 et
.cache/mission-manual-auto-controls-j9oltyf8/run/positive/setup ; Root inerte
possédé /tmp/swarm-model83-vs_blwo7, aucun compte/modèle réel.
La frontière reste obligatoire. planning_run_material construit Launch.workspace
vide, résolu vers Root : le remplacement par un workspace I0 attribué est une
précondition réelle des recettes planner et du candidat conducteur concurrent.
Le chantier migration poursuit ses lectures/unités indépendantes et ne modifie
pas la frontière Biba ni les sources PAEA. Clôture du conducteur non revendiquée.

### BT02 candidat et diagnostic natif du 28 octobre

Candidat d78b3c22ab989069f5d7ffc5cd7d184ebb5b96b88f9f4250513753c028b96087 : build normal stable sans features de fixture, non installé. Premier worker géré : échec avant sonde, CPython refuse FIOCLEX (trace réelle managed-trace). Garde syscall-before : script témoin valide sans profil puis refus avec ancien profil. Le filtre autorise désormais uniquement FIOCLEX en plus des opérations précédentes : pose CLOEXEC, aucune permission de fichier ni suppression CLOEXEC.

Correction de méthode : annonce de passage prématurée rétractée ; boundary-after était lié à une ancienne rlib, deux artefacts coexistaient par unification de features libc. Originaux conservés. boundary-linked-current utilise la sortie Cargo explicite debug/libswarm_biba_linux.rlib : témoin non confiné valide, puis Python/sortie I0 autorisés ; lecture privée, écriture privée/officielle, enfant et socket Unix refusés ; hardlink refusé. Recette permanente tests/rust_biba_linux_boundary.py/driver.rs ; cette preuve native autonome ne qualifie pas encore le candidat moteur.

Propriétaire Codex ; reprise : nouveau candidat moteur incluant ce seul changement de profil puis nouveau worker public attribué. Planificateur/reviewer cwd Root, contrôles distincts, mémoire/promotions et identité du répertoire à la frontière restent ouverts ; aucune livraison globale ou installation revendiquée.
