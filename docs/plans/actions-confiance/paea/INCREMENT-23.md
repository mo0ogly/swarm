# PAEA — incrément23 : campagnes de laboratoire depuis le menu

[English](../../../en/plans/actions-confiance/paea/INCREMENT-23.md) · [Plan](../PAEA-APEX.md)

## Reprise et décision

Commit22 `51a3ac828d75620f7e95108b045bbca80432b4fc`,42fichiers ;108entrées préparées d’autrui préservées, sans push. Deux préparations arrêtées avant mutation : aide CLI déjà incorporée par le commit moteur, puis base avancée par le commit de préparation. Les contrôles de précondition ont conservé ces essais. Aucun test supplémentaire n’est attribué au simple commit.

Besoin PA-08/09 : accéder aux campagnes réellement disponibles et comprendre leurs résultats. État : huit tests de laboratoire existent, sans accès UX. Choix : commande locale explicite utilisant la recette maintenue, rapport portable lié aux preuves et lecture depuis le menu. Alternative : lancer Python depuis le serveur natif ; écartée ici car elle ajouterait dépendance de livraison, gestion de jobs/ressources et politiques sans contrat existant. L’exécution depuis le navigateur reste une suite ouverte, pas une fonction prétendue livrée.

## Contrat et critères

Codex seul, sans GLM. Aucun provider, mission vivante, approbateur ou protection créée. La commande exige binaire/empreinte explicites et dossier neuf ; pas de compilation cachée, retry ou watchdog. La recette et son intégrité doivent être PASS/VERIFIED avant export. Le rapport ferme ses champs et ses huit tests nommés ; le contre-exemple brut d’alias reste une réfutation, distincte du succès de son test. Le lecteur n’authentifie pas le rapport fourni.

1. Commande : vrais huit tests/oracles sur racines isolées, originaux et empreintes conservés ; refus d’empreinte incorrecte/dossier existant sans écrasement ; reprise dans dossier neuf.
2. UX : menu/language/thème, rapport valide, refus vide/invalide/UTF8 et reprise, export exact, aucune requête métier, clavier/focus/320/390/1440, FR/EN et deux thèmes ; diagnostics suivis sans watchdog.
3. Livraison : profil normal/ressources réelles, version/notes/commit/dirty, lecture de l’instance locale ; revue politiques/fonctions/erreurs sur octets modifiés.

## État initial et limites

Runtime observé dev108, commit embarqué72e0e32dc, SHA5ef899642fae0a8a7fb2048dfa458a17a0cb5f06496255ffd22cdcb7231384ac, PID1006203. L’instance peut évoluer concurremment : recontrôler avant installation. PA-02/03 protégés, IA3/AC-02, humains, E2/E3,0/17invariants et RETEX global restent ouverts. Résultats et RETEX de cette tranche à compléter après exécution.


## Résultats vérifiés et livraison

Preuves : `evidence/20261026-lab23-o7z4kgiq/`. `component-recipe-01` conserve
la première qualification explicite par quatre assets substitués dans le navigateur
(422assertions/dev108), sans assimilation à une preuve native. `native-recipe-01`
exécute ensuite la vraie commande et les huit tests de laboratoire sur dev109,
puis **506assertions** sur12configurations,12téléchargements terminés et le refus
de script après témoin. Aucun asset substitué dans cette seconde recette. Les
deux recettes et les deux dossiers de campagne sont PASS/VERIFIED ; ils ne sont
pas des évaluations indépendantes. JSON malformé/BOM/UTF8, noms manquants ou
doublés, statut FAIL, autorité prétendue, hashes vides, champs supplémentaires,
chargement/double geste, reprise et export exact sont observés.

La commande publique refuse empreinte erronée et dossier existant sans écraser
le rapport ; une preuve de recette altérée en fixture refuse aussi l’export.
Schéma `schemas/paea/laboratory-report-v1.schema.json` vérifié sur le vrai rapport.
Les compteurs sont ceux des assertions, pas des essais humains indépendants.

Builds normaux01/02 refusés : source changée pendant compilation, puis moteur
concurrent non compilable (module writer absent/types de transaction). Après
modification réelle de ces sources, build03 reconnaît un candidat normal dev109
déjà qualifié par le chemin partagé. `candidate.json` relie six entrées compilées
PAEA exactes et le profil normal sans fixture. La relance source tente dev110 et
échoue dans le chantier moteur ; dev108 reste préservé. Le choix ponctuel supporté
`./swarm.sh restart --binary CHEMIN_QUALIFIÉ --no-open` active alors le dev109 exact
testé. Configuration `deploy/local-web.json` inchangée, futures constructions source
toujours actives. Aucun garde ni source étrangère modifié. Le défaut de compilation
courant reste ouvert, propriétaire chantier moteur ; reprise : sources cohérentes
et profil normal qualifié avant prochaine livraison source.

Installation observée : **v0.1.1-dev.109**, commit embarqué
`51a3ac828d75620f7e95108b045bbca80432b4fc`,modifié=true,
SHA256 `fb1dcac07bd0794a0f0d45c8b1f379c685ac2a1953e830f4bf784e647e1adc6e`,
PID1257214 au contrôle. `live-recipe-01` : **56contrôles de lecture seule et25sets
de ressources protégées** PASS/VERIFIED ; version, notes FR/EN, lien du commit,
divulgation non commitée, fermeture/focus et menu laboratoire vérifiés. Aucun
document métier envoyé, aucune base vivante utilisée comme fixture.

Une lecture `status` a attendu le verrou de cycle de vie détenu concurremment.
Diagnostic effectué ; la tentative d’interruption ciblée s’est arrêtée sur le PID
déjà disparu, sans signal envoyé. Le statut normal et la recette se sont terminés
sans voie de secours. `replan-status-lock.json` conserve la limite de cette méthode.

## Revue et RETEX de la tranche

Revue de même contexte, liée aux27fichiers réellement modifiés dans
`source-review.json` : politiques, fonctions et erreurs, liens et octets exacts.
Huit cas et64caractères SHA256 sont des invariants du format/recette, pas une
politique opérateur ; aucun plafond, retry, délai produit ou réglage Admin ajouté.
Les sources CLI/cœur/sink de la recette restent celles des tranches antérieures ;
aucune qualification globale n’est déduite de leur simple réutilisation.

Contrats de `digest/export_report/main`, lecteur `clear/status/validate/read/
download/navigation` et helpers de test documentés. Refus avant export, conservation
des causes/originaux, écriture atomique existante et effacement du succès périmé
tracés jusqu’au consommateur. Syntaxe Python/JS et diff propre sur les chemins
possédés. Le diff global conserve deux EOF étrangers déjà présents, sans correction
incluse dans PAEA. Visuels natifs mobiles FR/EN relus ; tests clavier et contraste
AA présents dans la recette. Revue indépendante et conformité globale non établies.

Apprentissage : associer un rapport de campagne à ses originaux et séparer test
réussi, propriété réfutée et authenticité non établie. Dev108/composant,
dev109/natif et dev109/installé ont des preuves distinctes. Ne jamais choisir un
candidat par l’ancien `bin/swarm` ; retenir le résultat du builder, son profil et
les octets servis. La page et les documents persistants transmettent ces limites ;
aucun contexte d’agent embarqué ou processus déjà lancé n’est réputé mis à jour
par cette documentation.

Suite concrète PA-08 : lancement de campagne depuis le navigateur encore non
implanté ; propriétaire intégration UX/moteur, reprise après contrat d’exécution
/jobs, ressources et livraison du laboratoire. La commande explicite et le lecteur
sont disponibles maintenant. IA3/PA-02/AC-02, services protégés, humains/E3,0/17
invariants et RETEX final du programme demeurent ouverts. Aucun commit23/push.


Contrôle après rédaction : le PID/hash installé reste dev109. Deux fichiers
partagés ont reçu des ajouts étrangers après la recette vivante : catalogue anglais
et notes. `final-live-profile-check.json` vérifie les25assets contre le profil
effectif installé, les36traductions possédées et notre note encore identiques aux
sources courantes. Ces ajouts étrangers ne sont pas annoncés installés ; la
conformité du catalogue global courant reste ouverte à leur prochain build.
