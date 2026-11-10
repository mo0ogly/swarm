# PAEA — incrément 20 : exemples téléchargeables

## Reprise APEX directe

Le commit demandé de la tranche 19 est `f410034889f00d67a86fb455a1b5b447e812aeb0`.
Les 108 chemins préparés étrangers restent identiques ; aucun push. L’utilisateur
ordonne ensuite la continuation directe, sans GLM : Codex réalise seul cette tranche.

PA-01/08 et le guide exigent un parcours utilisable. Les trois formulaires actuels
attendent des fichiers que l’opérateur doit construire. Ajouter huit JSON fictifs
versionnés, privés derrière la session native, et une aide repliée FR/EN dans les
trois vues. Réutiliser les vecteurs du contrat ; aucune donnée de projet en direct.
Choix : téléchargement statique avec sélection/geste explicites, plutôt qu’un
nouveau contrôleur de chargement automatique. Pas de politique, stockage métier,
API d’effet, accord ou droit ajouté.

Critères : génération déterministe vérifiable, GET/HEAD exacts et refus privé,
Root inchangé, vrais téléchargements puis uploads/vérifications dans les trois
parcours natifs, FR/EN × deux thèmes × 1440/390/320 ; documentation/revue liée aux
octets, build/restart supportés et contrôle de l’instance Version/notes/assets.
Le témoin initial sur le vrai binaire dev95 montre la page reçu 200 sans cette
aide, puis le nouveau fichier 404 ; Root de fixture et arrêt propre.

Les résultats restent à établir. Preuves privées : `evidence/20261026-examples20-06c9zk23/`.
IA3/AC-02/autorités humaines demeurent requis pour les parcours protégés ;
**0/17 invariants de bout en bout**, E2/E3 et RETEX final restent ouverts.


## Résultats, livraison et RETEX de la tranche

Le commit de la tranche 19 est : **`f410034889f00d67a86fb455a1b5b447e812aeb0`**,
27 fichiers, identité Git configurée, parent `3f1adcd50`. L’index isolé ne contient
que PAEA ; les 108 chemins préparés étrangers sont conservés. Les fichiers
partagés ne prennent que leurs passages PAEA, traductions et note FR/EN. Le
premier patch README ne s’appliquait pas à son contexte de fin partagé ; quatre
substitutions exactes et la section PAEA seule ont reconstruit le blob depuis
HEAD. Ce refus et la preuve de conservation sont retenus sous la tranche 19,
`evidence/20261026-compare19-5jvm093y/publication-01/`. Aucun push.

Cette nouvelle tranche 20 est réalisée **directement par Codex, sans GLM et sans
délégation**. Aucun commit supplémentaire n’est déduit de « commit puis fais ».

| Critère | Résultat observable et preuve |
|---|---|
| Avant modification | `baseline-public.json` : vrai natif dev95, page valide200 sans aide, nouvel exemple404, Root isolé et arrêt0. |
| Génération et contrat | `example-recipe-02` PASS : huit sorties `--check`, deux tests de schéma/liens/traductions et un test HTTP natif nommé. Huit assets exacts, absence de session refusée, HEAD vide, méthode refusée, reprise exacte, Root inchangé ; les fichiers sont consommables par les fonctions communes. |
| Vrais essais navigateur | `native-recipe-01` PASS : **1 743 assertions**, **12 configurations par vue**, **36 parcours d’exemples**, **120 téléchargements terminés aux octets exacts**, puis sélection et contrôle natif. Trois vues, deux domaines applicables, FR/EN, deux thèmes et 1440/390/320. |
| Non-régression et erreurs | Les **62 refus natifs attendus** et leurs reprises passent toujours ; zéro diagnostic inattendu. Perte de script injectée après témoins valides : sortie1 attendue, navigateur/serveur de fixture fermés. Aucun watchdog global. |
| Lecture/ergonomie | Aide initialement repliée et traduite, aucun chargement implicite ; clavier pour ouvrir puis télécharger, focus visible et contraste des liens >=4,5. Captures FR/clair/390 et EN/sombre/320 examinées, pas de débordement. |
| Analyse statique | Clippy du lib web `--no-deps --lib -D warnings` PASS ; 18 avertissements étrangers du moteur conservés. Aucun nouveau contrat public Rust de production ; le commentaire existant de `asset` reste exact. Docstrings du générateur, parseur et recettes mises à jour. |
| Construction qualifiée | `candidate.json` : dev96, profil natif normal `VERIFIED_DEFAULT_NATIVE_PROFILE`, features des composants sélectionnés vides, sans fixture ; **14 entrées PAEA compilées** identiques à l’archive effective. |
| Installation réellement observée | **dev96**, PID **549019**, SHA256 `bcb68e0f9e67f67e07a94115056717b75a9dfc85dc5231010caf3e1a235b080f`, commit compilé `f410034889f00d67a86fb455a1b5b447e812aeb0`, dirty explicite. Identité du processus et octets exécutés vérifiés. |
| Instance locale | `live-recipe-01` PASS : **40 assertions en lecture seule**, **19 ressources protégées** identiques aux sources, liens d’exemples dans trois vues, Version/notes FR/EN exactes, lien du commit et focus ; zéro écriture ou diagnostic inattendu. Échec de script séparé attendu. |
| Intégrité des preuves | Recettes exemples, navigateur et instance : VERIFIED/PASS. Première recette : VERIFIED/FAIL original conservé. Revue politiques/fonctions/erreurs et archive finale liées aux octets. |

### Échecs traités et portée exacte

La première recette Rust et le premier build ont rencontré une erreur moteur
étrangère : la closure `reviewer_native/publication.rs` n’avait pas un argument
emprunté de durée de vie suffisamment général. Son propriétaire parallèle a
ajouté le type explicite ; cette précondition nouvelle a permis l’instantané
suivant et le build dev96. Les deux instantanés ne remplacent aucune preuve
antérieure : chacun conserve 923 entrées déclarées, dont les nouveaux includes.
Aucune source moteur étrangère n’a été modifiée par cette tranche.

La relance normale a ensuite détecté d’autres octets du chantier parallèle et
tenté dev97 : compilation refusée, `capture::Capture::new` absent (E0599).
Le serveur dev95 a été préservé. **L’incident de compilation courant reste ouvert**,
propriétaire : chantier reviewer natif ; reprise : code cohérent puis qualification
du profil normal. Ne pas présenter le succès PAEA comme une réparation de ce défaut.

Option retenue après diagnostic : activation ponctuelle du binaire natif dev96
immuable déjà qualifié par `./swarm.sh restart --binary CHEMIN_QUALIFIÉ --no-open`.
Le lanceur supporte ce choix sans appeler `configure`. Les octets PAEA courants
sont comparés avant activation ; `deploy/local-web.json` est conservé octet pour
octet, donc les prochaines commandes sans `--binary` suivent toujours le build
source configuré. Aucun binaire déployé n’est patché et aucune garde de build
n’est supprimée. `replan-restart-01.json`, `restart-profile-errors.json` et
`restart-pinned-result.json` conservent cause, décision et résultat. Le prochain
build source doit encore corriger/qualifier le moteur avant une nouvelle livraison.

Le test HTTP exécuté ici est **un test ciblé nouveau**, pas une nouvelle exécution
des huit tests du target complet. Les manifestes complets attendent désormais
huit tests ; les résultats antérieurs gardent leurs compteurs. Les parcours natifs
complets sur le candidat dev96 vérifient les trois routes réelles et leurs refus.
Un ajout final documente seulement l’initialiseur du parseur Python : substitution
inverse identique aux octets de la recette, puis les deux tests Python rejoués PASS
(`test-docstring-delta.json`, `examples-final-python.stdout`). Aucun code produit
n’a changé après qualification. Les captures ne remplacent pas les interactions.

Le diff propre à cette tranche passe. Le contrôle global conserve les deux blancs
EOF étrangers déjà connus ; aucune conformité globale du dépôt n’est déduite.
Les notes, 16 traductions et chemins possédés sont inventoriés séparément.

### Apprentissages et reprise

L’exemple téléchargeable versionné relie désormais guide et formulaire : les
120 téléchargements ont réellement abouti, les champs restent vides jusqu’au
choix explicite, et leurs octets donnent les résultats attendus. L’activation d’un
candidat déjà qualifié est une solution supportée lorsque le checkout partagé
évolue ; elle exige profil/empreintes/ressources exacts et maintien explicite de
l’incident source. Le plan canonique transmet ce fait aux prochains contextes ;
aucune consigne embarquée ou session déjà démarrée n’est prétendue mise à jour.

Défaut de méthode retenu : `read_thread(includeOutputs=false)` inclut encore de
longues commandes, malgré la borne par item ; une lecture de diagnostic a donc
produit une sortie excessive. Suite décidée pour Codex : filtrer localement les
retours en statut et derniers messages utiles avant affichage. Le comportement
de l’outil est observé ; aucun correctif de cet outil ni effet futur n’est affirmé.

Cette tranche d’aide/données est installée et vérifiée localement, avec revue de
même session. Les campagnes de laboratoire ne sont toujours pas lancées depuis
l’UX ; ces liens ne sont pas une autorisation d’effet. IA3 EP-01/04/05/07/10,
AC-02 et personnes habilitées restent les préconditions de la suite protégée.
**0/17 invariants de bout en bout** ; E2/E3, acceptation indépendante et RETEX
final du programme restent ouverts.


Pendant l’audit final, le commit parallèle `20237b4a95cb35cd2a107a35e7e50d85dd4c90a4`
a fait évoluer 23 entrées d’index, toutes expliquées par ses fichiers. L’hypothèse
d’un index globalement immuable était trop large ; elle a été corrigée sans
écriture d’index. Les 108 chemins préparés d’origine restent identiques, notre
note20 n’est pas incluse dans ce commit et les octets produit revus sont inchangés.
`replan-index-observation.json` conserve cette distinction. Le binaire dev96 garde
son vrai commit compilé f41003488 ; un commit parallèle ne le reconstruit pas.


## Commit demandé après livraison

Après ces preuves, l’utilisateur demande « commit ». Le commit porte sur les
35 fichiers de la tranche20, avec documentation FR/EN et recettes permanentes.
Les 108 chemins préparés étrangers et les passages de préparation ajoutés depuis
la revue dans `native_admin.rs` sont exclus. Les traductions et la note de version
sont isolées depuis HEAD. Aucun push, build ou redémarrage supplémentaire : le
produit n’a pas changé depuis la qualification dev96 ; son commit compilé reste
historique. La preuve d’isolation/résultat est conservée dans `publication-02/`
du répertoire privé de preuves de cette tranche. Les critères protégés et
l’incident de compilation parallèle ne sont pas clos par cette opération Git.
