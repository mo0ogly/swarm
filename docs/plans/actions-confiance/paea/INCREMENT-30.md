# PAEA30 — proposition exacte restaurable, avant stockage du service

## Replan et contrat

29 :35tests isolés/6étapes et Rustdoc PASS, revue locale liée aux21fichiers.
Les7tests CLI observés passent mais la qualification refuse une modification
concurrente de sources étrangères. Build officiel dev126 compilé, profil refusé
pour3assets/traductions de préparation changés ; serveur parallèle dev125 observé,
aucune installation28/29 annoncée. Les preuves et leur limites restent conservées.

Prepared29 permet seulement le rejeu tant que l'objet reste en mémoire : ses
champs privés n'ont pas de codec public. L'instruction de le conserver en stockage
protégé est donc un prérequis futur, pas une capacité actuelle. Corriger cette
lacune avant de construire le stockage intent/checkpoint et le transport.

30a : prévisualisation cryptographique de lot signé sur les pins du journal,
sans SQLite/lock/état courant ; succès ne signifie pas application ni autorité.
30b : codec fermé v1 de proposition personnelle (base indépendante +deux événements
signés exacts). Restaurer vérifie canonicalisation, signatures/contexte/chaîne,
ordre/opérations/IDs exacts et record personnel lié à l'installation/principal/
génération indépendants ; aucun acteur ou pin appris du fichier. Export/restore
ne modifient rien, ne recréent pas de mot de passe ou sel et ne consomment rien.
30c : reproduction absence API puis roundtrip après nouvelle ouverture, reprise
exacte sans doublon, mauvais cadre/base/signature/contexte/kind refusés, avant
stockage même sous writer tenu ; erreurs de représentation avec cause conservée.
Revue aussi du decode base64 de28 : les causes étaient réduites en catégorie
Record. Conserver le type concret sans exposer les octets privés en diagnostic.

Le codec manipule des vérificateurs sensibles et ne protège pas un fichier.
Stockage atomique protégé intent/checkpoint, réception de décision après crash,
Admin/UX/transport et compte système séparé restent requis ;30 seul ne les prouve
pas. Codex propriétaire technique ; humains/bootstrap/E3 restent indépendants.
Reprendre livraison quand un profil effectif stable est disponible ; aucun retry
identique ni arrêt du travail technique indépendant.


## Qualification et livraison observées — 26 octobre 2026

39 tests nommés /7étapes PASS, intégrité VERIFIED (`recipe-02`) après renforcement
des témoins : autre installation et autre génération refusées distinctement,
prévisualisation exacte après fermeture incluse. Les39tests initiaux restent dans
`recipe-01`. Rustdoc state/authority et rendu Mermaid FR/EN PASS. La revue locale
est liée aux octets dans `source-review.json`, sans indépendance revendiquée.

Build normal soutenu par `swarm.sh`, profil dev128 sans feature fixture :495sources
effectives archivées et relues par empreinte, dépendances effectives stables.
6tests du harnais CLI figé PASS ;5exercent le candidat normal exact et1est un témoin
bibliothèque du harnais. Redémarrage pris en charge réussi ; serveur PID2128478,
version `v0.1.1-dev.128`, binaire SHA256
`eb8ee30e890f3c9e91c5cd83b1942e62d468fdf27c3fe7e5f89bf481ff0f6663`,
commit cbd6fecd7e40016d6c259296ac3a21373a4bbd60 avec modifications déclarées.
56assertions navigateur en lecture seule PASS et garde de refus du script attendu
PASS ; dialogue Version FR/EN, thèmes état/sombre, notes et lien commit observés,
captures relues. Cela installe aussi les bibliothèques28/29 sans exposer de service
personnel. Preuves : [dossier30](evidence/20261026-proposal30-5decuk_6/).

La lenteur du build/navigateur est reliée aux mesures de paging et à l'attente
initiale du verrou Cargo partagé ; progression réelle puis succès, aucun processus
d'autrui arrêté ni timeout ajouté. Une invocation avec mauvais argument de recette
a été refusée avant toute étape, puis corrigée sur son schéma public. Les preuves
incomplètes et refusées28/29 restent historiques et ne deviennent pas PASS.
La recette02de29reste une acquisition incomplète REFUSED, non une archive vérifiée.

Ce résultat qualifie les bibliothèques internes et leur présence dans le profil
livré, pas un transport personnel, une session humaine, le confinement ou E2/E3.
PA-02reste partiel et les invariants globaux restent0/17 établis. Suite immédiate31 :
stockage privé atomique, puis coordinateur intent/checkpoint et service protégé.
Aucun commit/push réalisé. La reprise automatique autorisée est enregistrée jusqu'au
27octobre2026à19h20Paris ; elle ne prouve ni progression continue ni acceptation.
