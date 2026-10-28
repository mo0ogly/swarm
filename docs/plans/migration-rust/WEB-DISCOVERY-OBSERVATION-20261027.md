# Conducteur web : observation et verrou Admin

Owner Codex ; suite du lot216, sans nouvelle autorité ni appel modèle réel.
1. Défaut public reproduit sur216 : le conducteur boucle sur settings/control_view avec réservation partagée ; conflit Lock errno11 sous Admin ferme tout HTTP, alors que configuration déjà enregistrée.
2. Choix retenu : observer le document atomiquement publié et les contrôles sous snapshot SQL, avec Root physique vérifié. Observation seulement ; aucune admission sur un droit ancien.
3. Conserver settings()/mission_control_view() réservés pour leurs autres consommateurs ; seul le conducteur de découverte emploie la nouvelle observation.
4. Politique/cadence toujours persistées, aucun délai, retry ou défaut ajouté. Les sessions réadmettent sous les gardes actuels ; leurs erreurs restent fatales, pas de retry de fournisseur.
5. Tester témoin valide, verrou exclusif, données invalides, remplacement Root et parcours CLI/browser Admin initialement en échec. Conserver les causes/fermetures, pas de faux succès vide.
6. Documentation FR/EN/Rustdoc, revue des octets, profil normal, recette et livraison locale exacte, données conservées.

Fichiers annoncés : nouveau root_store/mission_discovery.rs, déclarations additives root_store.rs/lib.rs, caller native_web/conductor.rs ; tests/recettes/docs dédiés. Les fichiers gate_evaluation/native_diagnostics en attente appartiennent aux autres chantiers ; aucun changement de leur logique ici.

Alternatives : répéter sur quota de retry nouveau ou ignorer toute erreur aurait changé la politique/masqué les refus ; modifier settings() globalement aurait affaibli les contrats réservés d’autres consommateurs. Une observation bornée au conducteur évite cette réservation inutile, sans prétendre une transaction commune fichier/SQL ou une autorité de lancement. Après un vrai échec de lecture/intégrité, l’arrêt conservateur actuel reste inchangé.

Premier check : imports erronés, modules controls/supervision_store sont déclarés par #[path] sous root_store, pas sous crate malgré leurs chemins physiques. E0432/E0433 originaux conservés ; références corrigées vers super, sans élargir la visibilité des helpers. Root_store est cfg(unix) mais ces contrôles sont Linux : déclaration/export nouveaux explicitement cfgLinux, aucune portabilité hors Linux revendiquée.

Erreur de méthode conservée : notes FR/EN ajoutées alors que Cargo test moteur
compilait encore ; version_history est une entrée embarquée du moteur, même si
ces tests de découverte ne consomment pas ses valeurs. Ce premier run reste
nommé/diagnostique, ne prouve pas un graphe source stable. Après stabilisation de
ces entrées, recontrôle ciblé requis ; ne pas traiter la documentation des notes
comme indépendante de la compilation. Aucun binaire de production qualifié sur
ce run ni archive source « corrigée » après coup.

Premier run intégration debug2 achevé11m43 juste avant demande d’arrêt ; contrôle PID absent a refusé le signal, aucun processus interrompu. Deux tests PASS (conflit Admin réel et Root remplacé), restauration de fixture refusée local-storage : octets corrects mais fichier temporaire mode0644 au lieu de0600. Métadonnées observées conservées ; fixation du producteur de restauration avant rename, sans assouplir la sécurité du lecteur. Nouveau run debug0 explicite (profil de debug livré, pas assertion d’identité avec l’ELF de production) ; originaux FAIL et changement d’entrée embarquée conservés.

Précision de mesure : la restauration défectueuse était réellement en0664 (umask
du processus), pas0644 supposé dans la première description. Le JSON de diagnostic
conserve stat et égalité des octets ; mode fixé0600, comportement de refus conservé.

Reproducteur public supplémentaire : première acquisition EX nonbloquante a
rencontré un lecteur déjà présent ; témoin HTTP valide mais faute pas injectée,
ce run n’établit pas le défaut. Précondition corrigée : attente noyau de la
réservation exclusive du seul Root détenu, sans boucle d’essais/plafond/délai PASS.
Le témoin défectueux216 doit réellement quitter avec la cause Lock ; le candidat
doit montrer deux progrès d’ordonnancement de son thread détenu puis HTTP intact.
Cadence de sampling = politique25ms sauvegardée, aucun watchdog global.

Clôture bornée218 : test intégration3PASS (ELF debug0 distinct), reproducteur216
GAP_OBSERVED3 avec arrêt réel Lock/errno11 ; candidat public5PASS sous verrou
Admin exclusif. Recette web2steps PASS/intégrité VERIFIED. Régression préparation
4steps PASS/intégrité VERIFIED (11schema,24arrêt,33replay,35API,40+48browser).
Boundary CLI2steps PASS/intégrité VERIFIED : journal47 CLI29/browser60 ;
journal46 CLI30 avec ancien worker v1 lisible/v2 refusé avant exécution.

Build qualifié fixture-free normal218 : rust-ubccave4, SHA complet
`e4b1b5adeb3a83073367656de85cc8c9c677096f83d1615ef2dd2d42fa89eb41`.
Les quatre fichiers produit propres correspondent à l’archive effective. Build
propre précédent pqhb8u36 refusé après quatre modifications concurrentes, jamais
promu.218 était déjà actif ; start --prepared-build idempotent confirmé, aucun
redémarrage prétendu. Parcours installé20PASS, Version FR/EN14, page11 ; ressources
protégées comparées aux octets archivés, paramètres et cinq travaux conservés.
Aucun appel métier ni acceptation globale des changements tiers.

Preuves permanentes : evidence/20261027-web-discovery-observation :74archives
vérifiées par roundtripSHA/fsync,50Roots inertes attribués, deux images hardlinkées
(test et production séparés), originaux préservés et Root vivant exclu. Inclut le
supplément CLI du lot précédent, sans remplacer ses41archives initiales. Revues
quatre axes des deux lots liées aux octets/profils ; erreur de chemin dans le
script de revue corrigée via recherche réelle swarm-web, aucune revue écrite
sur un fichier supposé. Le contrôle source reste borné aux changements décrits.

RETEX : l’observation de découverte n’a pas besoin d’une réservation Admin ;
l’admission conserve ses contrôles frais. Une restauration de fixture doit
préserver mode0600 et publication atomique, pas seulement les octets. Un témoin
défaut doit montrer l’acquisition effective du verrou avant toute conclusion.
Les journaux anciens doivent rester identifiés46, sans env implicite qui change
le scénario47. Deux temps de compilation (debug2 11m43, debug0 4m48, charges
différentes) ne forment pas un benchmark. Notes embarquées dans le moteur :
BLD-FREEZE-1 reste une proposition, pas une accélération réalisée.

Suite : requalifier la matrice devenue partiellement périmée à partir des routes
réelles ; agent list/history restent absents, mission watch et budget show sont
raccordés. Ni leur présence ni cette recette n’établissent l’acceptation globale.
