# PAEA66 — grand livre fictif séparé sous autorité courante

Reprise après65 vérifié/livré218. Objectif global PA00…13/RETEX conservé ; ce lot
implémente le noyau de service SQLite séparé, pas un daemon ou le profil protégé.
Codex direct, sans GLM/délégation/commit/push. Sources/fixtures d’autrui préservées.

Deux options : ajouter encore des entrées simulées dans Trust, ou utiliser un
stockage effectivement distinct avec témoins détenus et droits PAEA explicites.
Choix du second : trois tables dédiées, provisionnement explicite avant requêtes,
unicité facture/intention, séparation physique projet/ledger et publication SQL.
Le RootStore est réemployé pour ses descripteurs et son stockage vérifié ; aucun
nouveau moteur de custody fondé uniquement sur un chemin n’est inventé.

Plan : (1) requête détenue liant projet, ledger et action canonique ; (2) noyau
sous autorité personnelle/métier retenue, plan et mapping effect/révision Trust
courants réservés jusqu’à engagement ; (3) refus/reprise, effet ancien et erreur
COMMIT réelle ; (4) revue politique/fonctions/erreurs, docs FR/EN, build/install et
parcours Version normaux. Aucune étape suivante n’est considérée acquise.

Le contexte canonique fourni reste une donnée : ce backend n’atteste pas les
sources, le mandat, l’instance agent ou l’accord PAEA. Le futur contrôle attribué
reste requis avant émission de droit ; un compte de fixture n’est pas un humain.
Le noyau vérifie séparément le droit PAEA courant, le projet épinglé, le plan réel,
la révision Trust, le mapping canonique effect et les termes privés de la dette.
Les trois permissions et fonctions de l’ancien Trust ne sont pas détournées.

Erreurs : cause typée, nettoyage indépendant, observation NotAttempted/Uncertain/
Known. Un COMMIT en erreur reste incertain même après rollback ; réconciliation
explicite sous nouveau droit, avec résultat ReconciledNotCommitted distinct d’un
succès vide. Effet existant lu avant comparaison des nouveaux termes : changement
d’action pour même facture conserve le reçu ancien sans nouvel engagement.
Rollback explicite sans seconde tentative silencieuse par Drop. La revue a
identifié ce risque dans Transaction::commit de rusqlite0.40.2 avant qualification.

Recette permanente `paea-separated-settlement.json`, six tests prévus ; aucun PASS
présumé. Fixture de contention : réglage Admin storage_busy_timeout_ms=0 persisté
et consommé réellement, journal SQLite DELETE vérifié, témoin réussi avant faute.
Aucun watchdog global. Le timeout produit reste celui du RootStore existant,
sans nouvelle politique. DDL, version1, singleton et I2 sont des invariants de
représentation/protocole/Biba ; montants, délais et rôles de fixture sont données
de test. Fonctions publiques/non triviales documentées en Rustdoc ; revue finale
à lier aux octets. Preuves `evidence/20261027-settlement66-jxj0c9b6/`.

PA02partiel,0sur17 globaux, helpersUID/humain/daemon/confinement, PA03complet,
E2/E3/PA13/RETEX restent ouverts. Disque observé7,4Go libres après changement
extérieur ; aucun nettoyage réalisé ici. Test/qualification/livraison en cours.

Replan core02 : FAIL de compilation E0432 (module SQL réellement enregistré sous
RootStore par attribut path, pas à la racine crate) et diagnostic de type associé.
Aucun test exécuté ni PASS. Producteur relu à root_store.rs789–790 ; import corrigé
vers super::mission_supervision_store, processus original terminal avant édition.
Originaux conservés. Nouveau run sous précondition source corrigée, pas répétition
inchangée ni traitement du simple chemin physique comme namespace Rust.

Replan core03 : FAIL E0277, le helper auxiliaire conserve MissionSupervisionError,
pas RootStoreError. Alias réel relu ; cause désormais conservée dans l’enveloppe
SQL du Root, sans conversion implicite inventée. Aucun test exécuté. Ajout de deux
cas nécessaires pour contrat consommé : révision/mapping courants et schéma/reçu
corrompus, huit tests au total. Champ temps renommé commit_requested_at : instant
observé avant écriture, jamais présenté comme heure exacte de publication.

Replan core04 : compilation achevée, neuf tests exécutés mais tous en FAIL dans
la même précondition config Trust, avant backend/injections. prepare_identities
exige au moins trois identités ; fixture corrigée avec requester/approver explicites
et mapping humain effect, sans changer la validation produit. Reconstitution des
jetons des seules identités locales de fixture pour les changements publics de
configuration, sans fallback du mapping humain. Motif statique de refus disponible
dans le diagnostic de fixture. Recette désormais séquentielle : un vrai témoin de
règlement/rejeu/réouverture d’abord, campagne de neuf seulement après sa réussite.
Originaux FAIL conservés ; aucun ancien résultat n’est compté PASS.

core05 PASS : témoin puis neuf scénarios réels, publication unique/rejeu/réouverture,
révocation avant premier effet, COMMIT SQLITE_BUSY/absence réconciliée/reprise,
concurrence deux connexions et contrôles de schéma/rôle/plan. Aucun cumul du témoin
répété : neuf tests distincts. ELF de test figé, métadonnées Cargo06 confirment le
même hash exact et bibliothèques Swarm de production features=[]/test=false, aucun
test-fixtures (sauf serde/timestamp de protocoles) ; seul target paea_settlement
est un test. Cargo06 n’a pas réexécuté les tests ; sa recompilation a reproduit le
même ELF. Après05, un autre propriétaire ajoute parse/tests à Trust Operation :
changement conservé, sans altérer held_action ni paea_action. Diff retenu, pas
d’acceptation de ce travail extérieur. Livraison et Rustdoc encore ouverts.

Amélioration conservée (hypothèse de revue, pas anomalie reproduite) : validation
plus complète des métadonnées historiques de reçu, notamment instant texte et
association du digest d’autorité au pin du ledger. Valeur : refuser une corruption
de métadonnées avant leur exposition. Propriétaire Codex, périmètre PA09/transport,
critère : témoin stocké valide puis altération isolée/refus/reprise. En attente de
reproduction avant format de reçu exposé : aucun transport de ce reçu n’est actif,
ce kernel ne fournit ni signature ni détection de réécriture hostile du propriétaire.
Ne pas confondre vérification canonique actuelle et authenticité du reçu complet.

Replan doc08 : FAIL normal (pas test) : le nouveau raccord Trust d’un autre
propriétaire importe swarm_enrollment_state::Head, déclaré seulement en dev.
Déclaration normale ajoutée sans retirer le dev et sans modifier son code ;
aucun prix/budget/rôle changé. Aucun processus Cargo actif à l’édition. Ce refus
est conservé distinct de core05 PASS et du futur profil de production.

Raccord courant concret ajouté : call_paea_settlement, enveloppe Personal/Authority/
Action/MissingAction, résultat connu et incertitude séparés, branches originales
et nettoyage. Enregistrement dans le marcheur native_error_links existant, qui
retourne un graphe de children et non un itérateur ; contrat réel relu avant
compilation. Nouveau dixième test : règlement réel puis annulation personnelle/
métier, AfterUse converti dans la même variante que l’adaptateur et huit wrappers
Box/IO/Root/nettoyage secondaire. Même matrice sur vrai COMMIT SQLITE_BUSY.
Cette injection de bibliothèque ne prouve pas un transport ou un daemon protégé.

Replan red11 : contre-exemple réellement exécuté et FAIL conservé : avec
require_separation=true et sans admission attribuée, la nouvelle facture est
Recorded(replayed=false). core09 PASS reste historique et insuffisant pour ce
contrat ; aucune acceptation du noyau pré-correction. Correction de compatibilité
sur valeur persistée : Commit refuse séparation/participants non établis, sans
modifier les paramètres réels. Recettes positives désormais en profil référence
explicite require_separation=false ; politique active testée en refus et reprise.
Raccord d’admission permettant le profil protégé complet reste une tâche PA03.

Politique courante réellement attribuée dans la préparation (deux champs seulement),
empreinte domaine paea-trust-policy-v1 et recomparaison à engagement. Contexte
historique conservé pour Reconcile sous droit courant. Nouvelle responsabilité
policy.rs extraite sans copie d’un noyau de stockage ; autres champs de contexte
toujours déclarés. Douzième test : source/immutabilité du contexte, politique
ancienne refusée sous nouveau grant, observation historique et préparation fraîche.

Inventaire : les cinq ressources tools/agent-workflows réellement embarquées par
les depfiles Rust sont ajoutées aux entrées déclarées. Aucune consigne embarquée
modifiée. Sourcestable du runner restait un contrôle déclaratif, pas une preuve
complète du graphe compilateur ; qualification native normale distincte requise.
Minimum trois identités de fixture : contrat historique observé aussi dans
internal/engine/trust_actions.go542–543 et trust_actions_init.go94, préservé sans
introduire de nouvelle politique. Source/justification de ce choix historique
restent hors affirmation de conformité globale ; aucune règle opérateur nouvelle.

## Résultat local vérifié — 27 octobre 2026

La capsule13 a terminé PASS : témoin valide, 12 tests distincts et Rustdoc strict.
Son intégrité est VERIFIED. core12 reste FAIL pour changement concurrent, red11
reste le contre-exemple du contrôle de séparation absent avant correction.
La capsule conserve 779 entrées, sans liens ; les 22 fichiers de scaffolding des
packages CLI/Web/Biba/API ne sont pas compilés dans cette recette. Leurs packages
sont absents des artefacts observés. Les instructions embarquées n'ont pas changé.

Le build normal14 qualifié possède 677 entrées. Sur les 477 entrées communes avec
la capsule, seule version_history.json diffère (note66 ajoutée après gel). Les
bibliothèques métier ont features vides/test=false, aucun feature fixture. Trois
écarts transitifs time/macros, smallvec/const_generics+const_new, once_cell/default
sont examinés dans feature-review14.json ; pas d'identité entre ELF test et produit
revendiquée. Le test ELF13 est conservé séparément avec son hash et ses artefacts.

Le candidat source **v0.1.1-dev.220** a été activé par le chemin pris en charge
avec --prepared-build et son SHA complet. PID observé2848776, ELF
`90019b2d94ddeaef6e3422257ae2962e21994e9ca4d964feec53b7e93da4dd09`.
56 contrôles Version/UX et garde de script PASS, intégrité VERIFIED ; les25assets
protégés et les notes FR/EN sont comparés à l'inventaire compilé, captures FRclair/
ENsombre inspectées. Ces pages existantes ne consomment pas le nouveau backend.
Aucun paiement réel, nouveau daemon, service protégé ou parcours UX ajouté.

La revue locale source-review.json couvre les octets propres, politiques,
fonctions et erreurs. Les fichiers partagés contiennent aussi des travaux d'autrui ;
leur inventaire ne vaut pas revue exhaustive de ces changements. Les6pages types
Rustdoc et RootStore sont retenues avec hashes. La recette permanente et sa variante
figée ont les mêmes étapes, à l'exception de l'identifiant et de la cible de cache.

Méthode : deux erreurs de lecture/lancement sans mutation (README supposé sous
recipes et option --evidence au lieu de --output pour run) ont été corrigées par
inspection des chemins/aide réels. Aucun test échoué n'a été réécrit. Les preuves
historiques, bases et processus étrangers sont préservés ; aucun commit/push.

Suite : admission attribuée réellement vérifiée, puis service Git distinct et
transport/profil séparé selon PA03-SERVICE-INTEGRATION. Hypothèse de validation de
métadonnées de reçu conservée avant toute exposition publique. PA02/03 partiels,
0/17 invariants globaux, UID/humain/confinement/daemon/E2/E3/PA13/RETEX ouverts.
