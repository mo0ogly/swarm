# Reprise après73 — writer réel et formats vérifiés, engagement à raccorder

Codex direct sansGLM/délégation/commit/push. Preuve :
`docs/plans/actions-confiance/paea/evidence/20261028-barrier73-d4e4muti`.
Revalider heure, dernier lot, sources, travaux concurrents et vrai ELF ; aucun
numéro/PID/commit de cette note n'est un état futur garanti.

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

## Reprise pratique

Tous handles73 terminaux : fixture01/55660, core02/93195, integration03/94106,
build04/22288, restart05/29134, version06/66311. Sources et résultats finaux dans
CURRENT.json/source-review-final.json/local-result-final.json. Origine du premier
échec : formatter invoqué avant création de son module de test ; aucun Cargo
lancé après, source partielle conservée, module créé puis formatage sous verrou.
Ne pas rejouer un script d'extraction/append/finalisation déjà appliqué.

`managed_git_current.rs` : public observer crée son snapshot deferred ; corps
read_managed_git_publication partagé ; require_native_git_writer appelle la vraie
transaction_state(main), current_managed_git_publication_for_effect n'ouvre pas
de transaction. `paea_git_repository.rs` : inspect_using est la factorisation ;
inspect_for_effect vérifie writer puis le même observateur interne. Root.read_work
est un read_recorded_work direct sur self.db, pas un BEGIN caché ; source relue.
Aucune connexion/setting/validation-flag fournie par le client ne remplace Root.
Les réservations settings/matériel internes ne durent que la lecture ; l'owner
reste responsable des réservations extérieures à maintenir pendant l'effet.

inspect_object_format est public en Rust, mais n'ouvre pas un transport de service.
Il utilise Git réel --git-dir . depuis cwd FD, et lit true+sha1/sha256. Incompatible,
inconnu/non-bare refusent ; lire les vraies erreurs, pas un succès vide. Les format
et writer n'autorisent aucun COMMIT. Les trois helpers d'effet inutilisés en build
normal sont conservés comme contrat non raccordé, pas masqués par allow(dead_code).

Prochaine action PA03 : transfert d'objets et couplage de l'autorité courante.
Comparer réutilisation du fetch existant et streaming pack dans la classe de
capacité ; ne pas réintroduire un /proc/PARENT/fd qui puisse être réutilisé après
erreur d'enfant non récolté. Garder la source effectivement détenue et tous les
processus dans les erreurs jusqu'à récupération ; ne pas retourner "aucun effet"
si des objets ont été écrits avant erreur. Pas de limite opérateur cachée ou de
capture en mémoire non qualifiée d'un pack arbitraire. Aucun transfert n'est
réalisé73 et son budget/contrat devront être lus avant lancement.

Sous writer, utiliser les helpers internes et l'admission Scope/Trust/rôle/current
65/66 ; policy66 refuse séparation active sans admission attribuée. Ne pas baisser
une vraie politique ; les témoins positifs éventuels restent explicitement fictifs.
Pas de copie SQL positive, vraie fixture native via CLI et observer indépendant.
Le journal70 est pending-only ; Existing/Uncertain exigent réconciliation, pas
nouveau COMMIT. La déclaration72 reste immuable et n'est pas une autorité.

Éditions/formatter sous deploy/.local-build.lock nonbloquant, libérer avant swarm.sh.
Respecter les cibles Cargo/propriétaires et garder ELFs/preuves. Disque et version
se recontrôlent à chaque reprise. Ne pas nettoyer le cache normal partagé d'autrui.
Pas de corps workflow modifié ; règles documentées ne reconfigurent pas un agent
déjà lancé. Mission native utilisateur et données préservées.

Réveils existants bornés au28oct19h20Paris ; ne pas démarrer après, pas de nouvelle
automation/goal. API goal auparavant paused distincte de l'autorisation utilisateur.
UID/humain/approbations déjà pendants : ne pas répéter une question ou fabriquer
leur acceptation. Continuer PA00…13 et RETEX sans confondre ce lot avec clôture.
