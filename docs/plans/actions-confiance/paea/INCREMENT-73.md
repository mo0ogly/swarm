# PAEA73 — format du dépôt et lecture sous writer réel

Reprise28oct11:17UTC : dernier72, version229/PID3583154 et ELF
1e2384be40e0383520204fa86bad0cb56f578316330b44626e4ca2ae4a93285e observés.
Aucun Cargo actif,118Go libres. Codex direct sansGLM/délégation/commit/push.
Échéance des réveils28oct19h20Paris ; aucune nouvelle automation.

Replan PA03 : avant transfert/COMMIT, il faut refuser un format privé incompatible
avec les OIDs et lire le candidat sous la transaction réelle du futur effet.
Deux routes : dupliquer le lecteur/ouvrir une transaction imbriquée, ou extraire
son corps et exiger le vrai writer SQLite sur la connexion Root ouverte. Choix :
corps partagé, garde sqlite3_txn_state(main)==Write, pas un booléen de dispense.
La feature modern_sqlite est déjà effective (rusqlite0.40.2 bundled), source locale
primaire inspectée ; aucune feature de fixture ou dépendance nouvelle nécessaire.

Inspecter le format privé réel (SHA1/SHA256 du protocole existant40/64), sans
transférer d'objets, changer une ref ou admettre une autorité. Le futur service
reste responsable des réservations settings/Trust/plan et du point d'engagement.
Critères : manque de writer/read-only refusent, vrai IMMEDIATE permet le lecteur
natif courant sans BEGIN imbriqué ; autre writer refuse puis reprise après release.
Vraie fixture native neuve produite par CLI229, jamais copie SQL positive ou mission
utilisateur. Compatibilité observée et mismatch refusé, restauration/reprise.
PA02/03partiels,0/17/humain/UID/confinement/daemon/E2/E3/PA13/RETEX ouverts.

Préparation source interrompue au formatter : module de tests déclaré avant son
fichier. Aucun Cargo lancé après cet échec. Fichier créé ensuite et formatter
repris sous verrou, sans rejouer l'extraction/append. Fixture01 native neuve PASS.
Les deux SDK utilisent cette fixture73, pas l'ancienne preuve71 en écriture.
Le peer SQL de test est un observateur technique à attente0 pour mesurer BUSY,
pas une politique du moteur modifiée. Aucun événement de travail créé par SQL.

core02 PASS/VERIFIED : trois tests unitaires ciblés + Rustdoc strict, source stable.
ELF unitaire figé avant intégration03. Ils compilent cfg(test)=true pour accéder au
helper interne : ce n'est pas le profil livré. Une preuve supplémentaire normale
(test d'intégration lib cfg(test)=false) contrôle format compatible/incompatible
et rejoue les cinq cas antérieurs affectés par la factorisation inspect/read.
Pas de feature test-fixtures ajoutée. Les OIDs40/64 sont des représentations Git
existantes, pas un plafond de politique ni une assurance cryptographique nouvelle.

Intégration03 PASS/VERIFIED : six tests, bibliothèque normale cfg(test)=false ;
format réel compatible et mismatch spécifique, intention toujours0, reprise.
Régression des cinq cas72 justifiée par factorisation du corps public inspect et
du lecteur natif. Les trois tests02 cfg(test)=true restent distincts, avec leur
ELF retenu : pas d'assimilation à l'image normale. Source des helpers partagés
sans branche cfg dans leurs corps ; hooks unitaires en dehors de cette portée,
pas de feature test-fixtures ou booléen de dispense ajouté.

## Résultat local vérifié

Version **v0.1.1-dev.230**, PID3682612, activée via le candidat source préparé :
`0b8cf6947596da380ed2f9ee862ed80597f949b2d3607d2147a787fd0d051287`.
Commit effectif : 2496c031bee4d7d5ad36b2bbbf13f2c95d184dca, modifications non commitées
signalées. Le HEAD a évolué ailleurs pendant ce lot ; sa sortie git show est
retenue dans concurrent-head.stdout. Ce chat n'a exécuté aucun commit/push et
n'attribue pas le travail simultané à sa propre revue.

Core02 : trois tests unitaires ciblés PASS/VERIFIED, source stable, Rustdoc strict.
- Pas de transaction ou snapshot read : refus writer-required.
- Vrai IMMEDIATE : même lecteur natif/capacité, pas de BEGIN imbriqué ; l'observer
  ordinaire imbriqué refuse sans clore la transaction. Le writer reste détenu.
- Autre connexion : SQLITE_BUSY pendant le writer, succès/release après rollback.
- Source réelle73 et formats bare réels : SHA1 compatible, SHA256 incompatible
  avec candidat SHA1 refuse spécifiquement avant intention ; reprise compatible.

Le test de contention utilise un observer de fixture à attente0, sans modifier
une politique moteur. Les données positives de73 sont nées par CLI229/native
inerte, pas par SQL ou revue fabriquée. Le SDK n'ajoute aucune acceptation.
Intégration03 : six tests PASS/VERIFIED, bibliothèque cfg(test)=false, dont format
public et les cinq cas72 affectés. Intention toujours0 lors du contrôle ; pas de
transfert/ref mutation. Les trois tests internes cfg(test)=true sont séparés,
ELFs unit02 et integration03 figés. Ne pas présenter le binaire unitaire comme
production. Les corps modifiés sont partagés sans branche cfg(test), le chemin
public normal est exercé. Les helpers internes d'effet ne sont PAS encore appelés
par un consommateur normal public : warnings dead_code conservés, non supprimés.

Build normal :681 entrées,481 communes avec l'intégration03 ; seules les notes
version_history diffèrent après qualification. Cœur features vides/test=false,
aucune fixture ; graphe normal229→230 identique. Trois écarts transitifs avec le
profil d'intégration inchangés, justification historique réutilisée explicitement.
Version06 :56 assertions UX/Version et garde de script PASS/VERIFIED ;25 assets
protégés/notes comparés aux octets compilés, FR clair/EN sombre, commit réel,
noncommité, Escape/focus, captures inspectées. ELF actif vérifié séparément.
Documentation FR/EN, recettes et revue octets de politiques/fonctions/erreurs
actualisées. Diff de nos fichiers propre, diff global exit2 étranger conservé.

## Raccord restant

Le helper interne exige le vrai writer main sur LA connexion Root, et réutilise
le même corps natif chaîne/originaux/cohorte que l'observation publique. Il ne
commence ni ne termine la transaction de l'appelant. Ce n'est pas un mandat ni
une barrière complète : caller doit maintenir settings/Trust/plan/autorité jusqu'au
COMMIT. Aucune permission n'est dérivée de Write ou d'un booléen client.

Le contrôle de format est une précondition de représentation40/64 déjà existante,
pas une assurance cryptographique nouvelle ou une politique opérateur. Il ne
prouve pas que les objets soient présents dans le dépôt privé. Il reste à
transférer les objets avec processus/descripteurs retenus, vérifier leur identité,
composer l'admission courante65/66, réserver l'intention70 avant69COMMIT, vérifier
référence directe/OLD sous le verrou, puis réconcilier après rupture. Pas de
lecture d'observation transformée en approbation ou clôture automatique.
PA02/03partiels,0/17/humain/UID/confinement/daemon/E2/E3/PA13/RETEX final ouverts.
Demandes administrateur et approbations indépendantes déjà pendantes conservées.
