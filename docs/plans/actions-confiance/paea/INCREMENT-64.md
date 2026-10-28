# PAEA — incrément64 : préconditions Linux dans le produit

Reprise63 livrée/vérifiée215 ;45Rust/7nouveaux, profilnormal653/69,
CLI/UI même ELF. Le dernier tour est un progrès vérifié, pas une attente.
PA02 partiel,0sur17/E2/E3/humain/confinement/daemon/PA13/RETEX ouverts.
Codex direct sansGLM/agents/commit/push. Helpers uidmap absents ; action admin
précédemment demandée reste externe, pas de nouveau sudo ni contournement.

Besoin : rapprocher le diagnostic de la mise en œuvre effective. Le lecteur
Python du dépôt n'est pas accessible par le binaire installé ou le menu PAEA.
Le consommateur protégé ne peut pas se replier sur l'UID partagée. Fournir un
rapport natif purement observatoire commun CLI/API, puis une page PAEA liée.
Observer le processus serveur via API, le processus CLI via commande : ne pas
présenter ces observations comme celles du navigateur ou de l'agent futur.

Choix : lancer Python du serveur introduirait une dépendance/interpréteur et
une divergence de profil ; non retenu. Lecteur Rust partagé sans processus
externe retenu. Aucune écriture Root/hôte/compte/service ni activation. Pas de
nouvelle politique/délai/retry/plafond, chemins proc/etc du protocole Linux,
helpers recherchés sur le PATH effectif (métadonnées seulement, pas exécution).
Rapport version2 distinct de l'outil Python historique ; erreurs de lecture et
métadonnées invalides visibles, causes IO retenues dans le type interne et
codes/errno sûrs dans JSON. Aucun texte privé des sources dans le diagnostic.

Source identité : UID/GID/groupes noyau et passwd local (pas de requête NSS
réseau), mappings courants, seules plages attribuées au nom/UID courant,
helpers setuid root/ancêtres non modifiables par groupe/autres. Observation
métadonnées non atomique, sans FD détenu, exclusivité ni preuve de lancement.
Tous booléens de lancement/2UID/confinement/humain restent faux.

Acceptation : test public rouge puis parsers/IO/refus/reprise, vrai rapport
CLI/API protégée (méthode/query/body/auth refusés), schema versionné ; UI
loading/error/empty/success FR/EN clair/sombre/mobile/clavier/focus et scripts.
Source review trois axes, Rustdoc exact, recette permanente, profilnormal,
build/restart autorisés et contrôle identité/notes/assets servis. Pas de
pourcentage global ni conformité déduite de présence des helpers.

## Qualification intermédiaire64

Le lecteur est extrait dans `swarm-linux-prerequisites`, réexporté sans changer
l'interface moteur. Deux compilations unitaires moteur (419s/461s) ont exécuté
les6tests mais refusé l'inventaire global modifié concurremment ; originaux gardés,
jamais considérés qualifiés. Cargo public/API interrompus explicitement après
preuve de leurs entrées périmées, pas après timeout. Replan structurel conserve
les options et la décision ; nouveau composant ne dépend d'aucun moteur/Root.
Le lock ne change qu'un paquet local et la dépendance moteur, pas de paquet externe.

core13 PASS :6unitaires+1processus réel, Rustdoc strict et1source/4pages exactes,
4étapes/recette permanente `paea-linux-prerequisites-core.json`, intégrité vérifiée.
ui14 :105contrôles/8présentations FR/EN clair/sombre desktop/mobile via HTTP double,
aucune erreur navigateur inattendue. Refus metadata_safe contradictoire, données
vides visibles, focus restauré sans vol de focus déplacé, download exact.
Ces preuves UI ne sont ni backend natif ni livraison. En attente : qualification
native CLI/API, recette UI portable/intégrité/garde script, profil normal et
activation locale avec notes/assets réels. Runtime qualifié demeure215.

## Livraison locale64 vérifiée

**v0.1.1-dev.216**, ELF `3a5291dac03c91865b90acc8a57f6cbd116620868b8007bb8c499f8f37263b0c`, profil normal665entrées/81propres,
composant Linux sans featurefixture.10Rust distincts :core13(6unit+1réel), CLI2
sur premier stepPASS de capsule24 (FAIL global conservé pour ancien testWeb faux),
Web1corrigé dans capsule25PASS. Les665entrées normales de la capsule restent
exactes ;49entrées d'exemples et un lib Biba servent seulement au chargement des
manifests, ni compilés ni exécutés. Aucune capsule n'est installée.

111contrôles composant/8présentations, scriptStartuprefusé,3témoins/3refus
intégrité,Rustdoc strict1source/4pages,
37contrôles natifs/52CLI/16HTTP retenus (+session privée non conservée) ; Rootfixture
inchangé.60contrôles de page installée/8présentations et56Version/vues existantes
+gardescript, même ELF. CapturesFRclair/ENsombre et mobile inspectées. API GET
scopeprocessusserveur, pasclient ; helperuidmap toujoursabsent dans rapportréel.

Les deux défauts de décodage sont reproduits séparément : UTF8invalide et BOM
étaient acceptés après response.text ; TextDecoder fatal/ignoreBOMtrue les refuse,
avec reprise et exportoctetexact. Le défaut de champHttpReply.error concernait
le nouveau test : corrigé vers producteur réel.failure, aucun changement produit.

Basis216 : inventaire/notes COMPILÉS retenus. Modifications étrangères ultérieures
(rootlib nouveau module privé, i18n-en.js, trust-native.css) non livrées dans216,
non revendiquées revues.25ressources protégées correspondent toutes à la compilation.
La recetteVersion30 échoue sur comparaisondeworkspace ;31passe contre inventaire
compilé+notes dont SHA correspond à cet inventaire, sans affaiblir les checks.
Aucun rebuild/refonte/histoire réécrite pour faire disparaître ces différences.

PA02partiel,0sur17/E2/E3/humain/confinement/daemon/réel2UID/PA13/RETEX ouverts.
Le diagnostic ne remplace ni action adminuidmap déjà demandée ni preuve de lancement.
Prochaine étape : consommateur protégé/PA03 dans les droits et dépendances réels.
