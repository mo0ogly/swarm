# PAEA — incrément36 : lifecycle personnel détenu depuis démarrage protégé

Reprise35 installé dev137, 12 tests ciblés/6 harnais CLI/56 contrôles UI vérifiés.
PA-02 partiel ; aucune authentification humaine réelle, service personnel ni
séparation OS établi. Codex direct, sans GLM/délégation/commit/push. Préserver tous
les travaux et preuves parallèles ; reprendre la livraison sur candidat stable.

Deux routes : exposer la clé et des heads libres à un appelant HTTP, ou ajouter
un propriétaire interne construit uniquement depuis le bootstrap épinglé et le
chargeur privé. Choix : seconde route, sans ouvrir de listener dans ce lot.
PersonalLifecycle possède AuthenticatedControl, ProtectedSigner et les heads
initial personnel/métier indépendants. Le constructeur reste privé au crate.

| Étape | Critère observable |
| --- | --- |
| 36a | open_lifecycle lit une seule fois la déclaration v3 protégée et charge la clé attendue avant les sources mutantes |
| 36b | initialize sans head client ; enroll sans clé/acteur client, preuve d’installation et mot de passe empruntés ; recover sans nouvel input |
| 36c | execute réutilise les deux autorités actuelles et le head métier épinglé ; close consomme l’objet et détruit la clé même après erreur |
| 36d | Démarrage froid réel de fixture, mauvais secret/clé/heads/cancel, reprise d’intention exacte, refus sans mutation et réouverture |
| 36e | Recette permanente, revue liée aux octets, Rustdoc/FR/EN, profil normal et installation |

Le démarrage n’initialise pas implicitement, ne crée pas de secret ni de compte
humain et n’adopte aucun head de DB. Les emprunts secrets restent à effacer par
leur propriétaire. Les générations personnelle/métier restent indépendantes.
Aucune nouvelle politique, borne ou valeur KDF. close consomme le wrapper ; après
échec de close, rouvrir explicitement les sources protégées après diagnostic,
sans annuler les effets connus ni réinitialiser l’histoire. Les preuves de
publication restent les CommitError/Progress et Outcome existants. La rotation/
récupération de mot de passe conservée33, le transport, l’UX, l’UID séparé et le
confinement restent nécessaires. E2/E3/PA-13 et0/17 invariants globaux ouverts.


Premiers9 tests démarrage/lifecycle PASS. Ajout du cas fermeture consommante avec
erreur réelle conservée/réouverture ; recette43 tests/4 étapes (10startup+8clé+
9coordinateur+16Control). Comptes des anciens manifestes actualisés sans réécrire
les preuves4/9 tests. Nom de lock/code d’erreur supposés corrigés depuis leurs vrais
producteurs AVANT exécution ; aucune suppression de lock, témoin indépendant puis
fixture froide de refus. Champs internes restreints au crate pour le propriétaire,
sans head/acteur/clé de requête.


Profil actif141,6 CLI et56 UI PASS. Relecture sémantique des captures : anciennes
local_notes annonçaient encore comme futurs des raccords désormais réalisés.
Consolidation des9 éléments internes propres en une capacité actuelle, tous les
autres éléments/vues conservés et octets141 archivés. Nouvelle livraison requise
avant clôture36 ; comparaison exacte des notes ne prouve pas leur justesse.
Aucune instruction Rust ni test modifié par cette correction documentaire.


## Qualification locale et livraison

43 tests nommés/4 étapes PASS, intégrité VERIFIED :10 startup/lifecycle,
8 clé privée,9 coordinateur durable et16 Control. Les6 nouveaux cas lifecycle
couvrent bootstrap froid, refus sans consommation, annulation, head périmé,
reprise aux trois phases durables et close consommant avec erreur post-close
injectée après vraie fermeture. Les9 cas coordinateur incluent les trois sorties
réelles de processus73 ; cela ne démontre pas une panne d’alimentation.
Un commentaire du getter interne a été désambiguïsé après les tests : pont
avant/après conservé, seules lignes Rustdoc changées ; instructions Rust identiques.
Rustdoc courant PASS ; diagrammes FR/EN rendus et relus.

Candidat139 qualifié/6 tests CLI PASS. Redémarrage supporté a construit140 pour
les changements parallèles ; première recette UI lancée prématurément sans son
fichier de statut, FAIL avant navigateur, original conservé. Le premier diagnostic
liait ensuite140actif au dernier141préparé, refus correct mais hypothèse fautive.
Collecteur corrigé : sélection par SHA réel du processus parmi les profils
supportés qualifiés, état préparé observé séparément.141actif :6 CLI et56 UI PASS.
Relecture sémantique a ensuite révélé neuf notes internes obsolètes : consolidées,
originaux conservés, nouvelle livraison supportée effectuée.

Livraison finale observée : `v0.1.1-dev.142`, PID2684589,
SHA256 `21afe989a7265902ec4f49527df3a656a888417781bd99f792531b6a6e99bcea`, commit `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`,
modifications non commitées signalées. Profil normal sans fixtures,
519 entrées effectives archivées et vérifiées, sources propres et
bibliothèques réellement compilées liées au profil, dalek zeroize présent.
6 tests du harnais CLI figé PASS :5 parcours processus sur le binaire exact,
1 ancien témoin bibliothèque, distinct des43 tests de bibliothèque courants.
56 contrôles UI protégés et garde du script manquant PASS ; captures Version
FR état/EN sombre relues, note consolidée exacte et sémantiquement actuelle.
Ces tests CLI existants ne sont pas encore le parcours CLI d’enrôlement personnel.

Revue locale du même contexte liée aux octets sur politiques/fonctions/erreurs,
sans acceptation indépendante. Aucun humain réel, service personnel, transport,
confinement ou compte OS séparé établi. PA-02partiel,0/17 invariants globaux,
E2/E3 et PA-13 ouverts ; aucun commit/push. Suite37 : commandes natives publiques
init/enroll/recover avec secrets hors argv et diagnostics gardant toutes les causes
et observations de publication ; puis transport/UX et rotation/récupération33.
