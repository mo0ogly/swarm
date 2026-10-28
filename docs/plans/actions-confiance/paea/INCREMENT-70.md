# PAEA70 — intention Git durable avant publication

Reprise du 28 octobre 2026, 08:59 UTC : dernier incrément69, version222/PID3014575,
ELF eee53fe5325b220f329a9f95196624f67ed3801ef12bd2384197ae8409115bbf observés.
Aucun build/test concurrent observé. Codex direct sans délégation/commit/push.
Disque1,4Go : écritures adjacentes atomiques ; aucune suppression d'autrui.

Replan PA03 : le protocole69 ne réserve aucune intention durable. Deux routes :
raccorder immédiatement toutes les autorités/revues/dépôts, ou établir d'abord
la réservation persistante qui interdit une seconde publication après rupture.
Choix borné : journal explicitement provisionné dans un RootStore dédié, identité
métier canonique, résultat New/Existing distinct et conservation des incertitudes
SQL. Ni Existing ni une intention ne sont une autorisation de réessayer Git.
Le futur service doit encore retenir les deux stockages/dépôt, vérifier le vrai
candidat/revue/cohorte et l'autorité courante, puis réconcilier la référence.
Pas de nouvelle API HTTP permissive ni de faux service activé.

Critères : absence sans mutation, provisioning explicite, clé transport exclue,
conflit de contenu conservant l'ancien enregistrement, reprise après réouverture,
contention sans duplication et COMMIT incertain conservé ; corruption refusée.
Tests sur Roots jetables, Rustdoc strict, recette permanente et revue octets sur
politiques/fonctions/erreurs/documentation. Installation locale seulement après
qualification. PA02/03partiels,0/17 et clôture/RETEX globaux restent ouverts.

core01 FAIL de compilation : export public de root_store non relayé à la racine
lib.rs, utilisé par les diagnostics/tests. Sept erreurs de résolution, aucun test
exécuté ; original conservé et intégrité VERIFIED. Export racine ajouté ; reprise
sur cette précondition modifiée, sans changer politiques ni critères.

core02 FAIL compilation fixture : apply_settings exige l'environnement en second
argument. Contrat réel relu et invocation corrigée ; aucun test exécuté. Défaut
de méthode : extrait tronqué pris pour signature complète. Relire la signature et
l'appel existant complet avant consommation. Export Linux remis sur chaque bloc
au lieu de déplacer involontairement la garde de l'export settlement existant.

core03 six tests/Rustdoc PASS, intégrité VERIFIED, ELF conservé avant04.
core04 six tests + composition Git réelle + Rustdoc PASS, intégrité VERIFIED.
protocol05 neuf cas Git complets + Rustdoc PASS, intégrité VERIFIED ; élargissement
justifié par ajout du raccord de fixture et modification du graphe diagnostic.
ELFs des deux cibles figés séparément dans test04-artifacts.json. La garde Linux
du nouveau module/export RootStore est explicite ; pas de preuve multi-OS.
La fixture perd l'ACK après effet réellement observé et retrouve Existing après
réouverture : cela ne prouve pas un service protégé ou une reprise automatique.

## Résultat local et limites

Version **v0.1.1-dev.223** installée par restart --prepared-build exact, PID3128318,
ELF `7143f8c23550d1e00d4ec89f7fb7bec40ed7a38ca3af209ad2dee704d388592f`.
Build normal679entrées vérifiées,479communes aux tests04 : seule version_history
change après qualification. Profil sansfixtures ; graphe normal222→223 identique.
Trois écarts transitifs tests/normal identiques à69, justification sémantique66
réutilisée explicitement ; jamais assimilation ELFtest/ELFnormal.

Six tests journal + neuf tests Git distincts PASS, Rustdoc strict PASS, inventaires
stables et intégrité VERIFIED. Deux cibles ELF réellement exercées figées sous
qualified-test04-* ; état03 conservé séparément. Composition réelle : témoin
valide avant faute, SQL lisible depuis une autre connexion avant Git COMMIT,
SIGKILL Git après effet/avant ACK puis Existing identique après réouverture.
Aucun reçu terminal/reprise automatique ou service protégé n'en est déduit.

Version08 :56contrôles PASS et garde de script attendue PASS, intégrité VERIFIED ;
25assets protégés comparés à l'inventaire effectif et notes compilées exactes,
FRclair/ENsombre, commit réel et noncommité, Escape/focus, captures inspectées.
Ces parcours publics existants ne consomment pas la nouvelle bibliothèque.
Documentation FR/EN et notes70 actuelles ; revue des politiques/fonctions/erreurs
liée aux empreintes et originaux préservés. Warnings verify_candidate existants
hors lot conservés. Diff propre sur nos chemins et nouveaux fichiers ; diff global
exit2/deux lignes étrangères strictement identiques à69, jamais annoncé propre.

Le journal est une bibliothèque trusted-caller : pas d'autorité attestée, pas
l'obligation de stockage séparé exécutée à sa place, pas de transport/daemon ou
confinement. Le service à venir doit retenir les stockages/projet/dépôt, vérifier
le vrai candidat/revue/cohorte et l'autorité courante, réserver durablement puis
publier sous contrôle. Existing/Uncertain exigent la réconciliation de la vraie
référence et ne donnent jamais une autorisation de retenter. Propriétaire Codex/PA03.
Critère suivant : service refusant sans ces préconditions, puis fixture native
signée avec vrai candidat/revue et observation directe de l'effet sans duplication.
PA02/03partiels,0/17/UID/humain/E2/E3/PA13/RETEX toujours ouverts ; demandes admin
et validations indépendantes déjà pendantes, pas de nouvelle question ni contournement.
Aucun commit/push/délégation. Tous handles70 sont terminaux à clôture de ce lot.
