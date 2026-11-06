# Sécurité et preuves — APEX

Plan adopté par « ok on lance », le 5 novembre 2026. Propriétaire : Codex,
développement direct ; aucune délégation, publication ou consommation fournisseur.
Complète [BLP-APEX](BLP-APEX.md), sans clôturer ses sept lots.

## Analyse et décision

L'administration BLP existe dans `web/trust-confidentiality.js`, derrière les
opérations authentifiées `blp-show`, `blp-policy-preview`, `blp-policy-apply`.
Le formulaire n'est ni une couverture transversale ni un registre de qualification.
Les preuves conservées par `tools/verification/recipe.py` sont des vérifications
de sources déclarées, distinctes du graphe compilé et de l'acceptation indépendante.
Au démarrage : HEAD d91d2391a4, instance annoncée dev.332 ; dépôt partagé modifié,
notamment admission/dialogue et service HTTP. Ces observations ne qualifient pas
leurs modifications. Préserver ces travaux et leurs preuves.

Deux options : étendre seulement le formulaire Biba (simple, mais mélange les
domaines et duplique les preuves), ou une projection commune consommable par les
interfaces (retenue, livrée progressivement). Premier raccord : BLP. Biba et la
composition BLP/Biba ont leurs propres critères ; aucune conformité par héritage.

## Exigences et états

SP-01 distingue configuration, application, vérification et acceptation.
SP-02 décrit critères/frontières, limites, propriétaire et reprise.
SP-03 lie résultat et sources/politique/configuration/profil/binaire exercés.
SP-04 conserve résultats historiques et distingue fraîcheur actuelle/inconnue/périmée.
SP-05 réutilise validation, prévisualisation et application révisionnée communes.
SP-06 n'exécute que des recettes enregistrées, autorisées, en fixtures isolées.
SP-07 sépare consultation, configuration, exécution, revue et acceptation.
SP-08 protège les rapports et exports comme des données potentiellement sensibles.
SP-09 exige FR/EN, deux thèmes, clavier/focus et mobile.
SP-10 conserve un socle commun et des critères spécifiques BLP/Biba/composition.

Pas de score global déduit du nombre de tests. Résultat, fraîcheur et acceptation
sont trois axes indépendants. Aucun catalogue déclaratif n'est une preuve PASS.
Une empreinte lie des octets, sans attester auteur ou autorité. Une certification
externe exige un organisme/référentiel/périmètre réellement établis ; l'application
parle de qualification interne. `runtime_enforced:false` reste affiché.

## Lots et clôture

| Lot | Dépendances | Livrable / périmètre | Acceptation et reprise | État |
| --- | --- | --- | --- | --- |
| SP-L01 | — | Contrats, sources, droits, périmètre BLP et inventaire | Chaque observation a une source ; inconnus visibles | CADRÉ — contrat lecture et inventaire de sources |
| SP-L02 | L01 | Projection moteur/API/CLI en lecture, métadonnées sans contenu classifié | Refus sans identité/admin, schéma strict, zéro mutation, politique absente/périmée distinguée | RÉALISÉ / VÉRIFIÉ LOCAL (périmètre lecture) |
| SP-L03 | L02 | Vue Sécurité et preuves : politique, couverture, vérifications, preuves, qualification | Pas de réussite inventée ; FR/EN, thèmes, clavier, erreurs, mobile et vrais assets | PARTIEL — UX fixture vérifiée ; session Authority réelle ouverte |
| SP-L04 | L02–03 | Administration structurée raccordée au formulaire existant | Preview sans mutation, révision concurrente, reprise/redémarrage, même validation API/CLI | À FAIRE |
| SP-L05 | L02–03 | Adaptateur de recettes, droits, progression/arrêt et preuves durables | Recette inconnue refusée ; pas de chemin/commande arbitraire ; refus/interruption/reprise conservés | À FAIRE |
| SP-L06 | L04–05 | Dossier de qualification, revue, décisions, export protégé | Aucun critère obligatoire manquant/FAIL/périmé accepté ; candidat et réviseur liés | À FAIRE |
| SP-L07 | L01–06 | Recettes publiques, revue, installation locale et RETEX | Image active, notes FR/EN et parcours servis ; revue indépendante distincte | EN COURS — image active ; revue/qualification/RETEX globaux ouverts |

Codex possède les lots d'implémentation. L'opérateur habilité possède les décisions
de politique ; le réviseur indépendant possède sa revue et l'autorité d'acceptation
sa décision. Aucun rôle n'est accordé par ce document. Le premier accès utilise
l'admin I3 courant existant : matrice de consultation distincte ouverte en L06.
Le catalogue commun n'ajoute aucun droit Authority et réutilise le parcours Trust
protégé. Une session Authority n'est pas un jeton Trust I3.

## Vérification et livraison

Tests ciblés avec témoin valide avant injections : refus de session/jeton invalide,
principal non admin ou désactivé ; requêtes malformées/clé inconnue ; politique
absente/courante/périmée ; aucune fuite d'habilitations, contenus, chemins ou secrets ;
zéro mutation à la lecture. API/CLI doivent rendre le même contrat.
Pour l'UX : erreurs/chargement/vide, rendu texte non HTML, FR/EN, deux thèmes,
focus, clavier et largeurs 1440/1024/760/390/320. Vérifier les octets HTTP réellement
embarqués, profil sans fixtures, version active et notes FR/EN.

Pas de plafond/timeout caché pour le futur exécuteur : toute politique nouvelle
est persistée et administrable. Arrêt et reprise doivent conserver le résultat
incertain, pas transformer une interruption en PASS. Aucun fournisseur payant.

Preuves futures : vérifier montage Lexar, espace/écriture ; fixtures et staging Unix
restent dans l'aire configurée. Archiver avec métadonnées, relire et vérifier les
empreintes sur USB, conserver un index local. Ne supprimer aucun original historique.
Si support indisponible, noter le repli local et vérifier l'espace.

## Replan du 5 novembre

Le plan précédent n'était présent que dans le chat : ce fichier et son miroir EN
assurent sa reprise. Les états BLP plus anciens ne sont pas reconduits : BF-16a a
du code et une preuve publique encore déclarée UNVERIFIED au dernier commit.
Ne pas transformer cette présence en réussite. Comparer les contrats et empreintes
avant réutilisation ; les modifications concurrentes restent hors de notre acceptation.

Premier incrément : lecture commune et UX BLP (L01–03), avec liens vers
l'administration existante ; commandes de recettes/acceptation indisponibles tant
que leurs contrats L05–06 ne sont pas livrés. Cette découpe ne retire aucun lot.


## Premier incrément activé — 5 novembre 2026

Image v0.1.1-dev.336, SHA256
`23e4a1d64d958cbaed472e5a44f6b103f9b2e1f2997bba25488976e02b3f5214`,
commit compilé d91d2391a4c5e0bf767c9cd1f09e1a2e34986965, modified:true.
Build qualifié `rust-xzx_n63t` : profil normal, 4 artefacts cœur sans features,
aucune fixture, 764 entrées effectives archivées ; 15 entrées de notre périmètre
revérifiées contre leurs octets compilés. Restart préparé exact : zéro entrée
compilée plus récente en attente ; instance prête PID observé 2268485.
Authority active sur 38433, moteur interne 38434, installed_matches_build:true.
Ce constat daté n'est pas une garantie sur les futurs redémarrages.

Preuves dans `.cache/security-proofs-20261105-01`, index durable
[livraison et restauration](security-proofs/DELIVERY-20261105.json).
Recette moteur `engine-after-02` : VERIFIED/PASS, 1 test nommé ; recette publique
`public-recipe-04` : VERIFIED/PASS, 22 assertions CLI/HTTP et 77 navigateur
FR/EN/thèmes/clavier/5 largeurs/contrastes normaux après fin des animations.
Recette inventaire `inventory-recipe-02` : VERIFIED/PASS, 69 références,
25 flux OPEN et 5 contre-exemples/refus/reprises. Version/notes FR/EN/lien commit/
modifications non commitées : 56 assertions supplémentaires sur la même image en
fixture détenue (`version-fixture-01`). Aucun de ces nombres n'est un taux de
conformité ni une acceptation indépendante.

`installed-01` conserve PARTIAL_AUTHORITY_UI_NOT_VERIFIED : image/version/
22 sources BLP connues vérifiées, mode Authority et trois refus UI owner-key
confirmés. La vue réelle a été ouverte : Connexion à Swarm, aucun compte utilisé,
aucun droit augmenté ni identité fabriquée. Critère ouvert, propriétaire utilisateur :
connexion humaine existante, puis vérification de la vue et du dialogue Version.
Les 77 assertions de la nouvelle vue restent une preuve de fixture, pas cette recette.

Suite concrète : L04 administration structurée (sans dupliquer les producteurs),
L05 recettes enregistrées avec droits/progression/arrêt/reprise et politiques Admin
persistées, L06 dossier protégé et décision liée au candidat, L07 revue indépendante
et RETEX transversal. L03 doit encore recevoir ces fonctions et leur parcours réel.
Aucune preuve historique importée, aucun export de contenu, aucune décision de
qualification, aucun raccord Biba/composition, aucune donnée réelle classifiée.
Le programme reste ouvert ; runtime_enforced:false est conservé.

### RETEX de l'incrément et découvertes

- Plan uniquement dans le chat : persisté FR/EN et lié depuis BLP-APEX ; futur
  repreneur doit lire ce suivi. Aucun contexte natif embarqué n'a été modifié.
- Régression initiale `before.*` : la commande n'existait pas ; le test nommé
  échouait. Échec original conservé, contrat maintenant vérifié.
- `public-recipe-01` FAIL : Chrome Puppeteer attendu absent, zéro interaction.
  Sélection du Chrome installé et collecteur commun Node/npm/Chrome ; nouveau
  témoin positif, versions/empreintes effectives conservées. Ne pas hériter d'un
  cache navigateur supposé ni d'un watchdog CDP implicite.
- Inventaire périmé : ancrages de sept fichiers relus, y compris évolutions
  concurrentes, empreintes/lignes actualisées ; aucun acquittement fonctionnel
  rétrospectif de BF-16a ou du stockage. Le FAIL original est conservé.
  `inventory-recipe-01` restait FAIL malgré le checker vert car son témoin
  attendait encore 67 références. Manifeste corrigé à 69 et deux sources ajoutées,
  puis nouvelle recette complète et ses cinq tests ; ne jamais confondre sortie
  du composant et réussite du lanceur.
- Captures mobiles trop longues : listes de frontières/recettes repliables,
  raisons bloquantes visibles ; effet revérifié sur 5 largeurs et au clavier.
- Pression disque/IO observée pendant un écrivain Docker dremio_ai : question
  transmise sans arrêt d'autrui ; processus terminé naturellement, espace rétabli.
  Premier build 9m50, second 1m30 ; cette corrélation n'est pas une mesure causale
  ni une nouvelle politique de délai. Aucun nettoyage de preuves ou watchdog.
- Toute reprise des contrôles après changement d'entrées garde les anciens résultats ;
  contrôle partagé inchangé réutilisé seulement dans sa portée déclarée.
