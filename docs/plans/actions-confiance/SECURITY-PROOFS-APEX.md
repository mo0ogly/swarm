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
| SP-L04 | L02–03 | Administration structurée raccordée au formulaire existant | Preview sans mutation, révision concurrente, reprise/redémarrage, même validation API/CLI | PARTIEL — SP-L04a et initialisation livrés localement en380 ; session Authority et intégration main ouvertes |
| SP-L05 | L02–03 | Adaptateur de recettes, droits, progression/arrêt et preuves durables | Recette inconnue refusée ; pas de chemin/commande arbitraire ; refus/interruption/reprise conservés | À FAIRE |
| SP-L06 | L04–05 | Dossier de qualification, revue, décisions, export protégé | Aucun critère obligatoire manquant/FAIL/périmé accepté ; candidat et réviseur liés | À FAIRE |
| SP-L07 | L01–06 | Recettes publiques, revue, installation locale et RETEX | Image active, notes FR/EN et parcours servis ; revue indépendante distincte | EN COURS — livraison locale380 ; intégration, qualification et RETEX globaux ouverts |

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

### Correction d’initialisation et livraison380 — 15 novembre 2026

[Reçu380](SP-L04a-INIT-DELIVERY-20261115.json) : image native par défaut sans
feature fixture, SHA256 `f0e92ce3749855bee14f2b6ea68037c0e5532ac58308e36988f76ad6b46678a1`,
base Git2d06746f et modifications non commitées déclarées. Redémarrage géré
réalisé après qualification ; processus3575178/exécutable local/sonde Version
autorisée concordent. La nouvelle note FR/EN est observée dans le binaire actif.
Ce constat ne vaut ni fusion Git ni acceptation globale. Le reçu379 reste historique.

La revue CI a trouvé une course d’initialisation : show pouvait finir avant le
montage Sécurité ; le formulaire restait vide sans clic Admin. La jointure à usage
unique attend maintenant lecture et rendu complets ainsi que le formulaire inline.
Un échec de rendu après affectation de state ne rend pas l’état prêt ; une relecture
explicite saine reprend. Les saisies et previews ultérieures ne sont pas rechargées
automatiquement. Cela ne désactive pas tous les contrôles : leurs gates existants
et la relecture Admin restent inchangés. Aucun délai, retry ou nouveau fetch.

Revue indépendante source/synthétique favorable, avec témoins RED et GREEN.
La garde permanente maintenue dans tests/security_initialization_ui.cjs et sa
recette couvrent sept cas ;61 assertions synthétiques réussies, compteurs réels
initial0/reprise1. Les anciens champs constants adminClicks:0 sont NON MESURÉS,
leurs originaux conservés et bornés par erratum. La recette INIT a reçu un contrôle
discriminant de métadonnées/21inputs/archive relue/stabilité/chaînes attendues ;
aucune étape de cette recette complète n’a été relancée pour son emballage.

Le parcours natif sur380 passe59 assertions : vraies réponses API, deux ordres
contrôlés par continuation de GET, six assertions initiales sans clic Admin,
quatre parcours FR/EN/deux thèmes/mobile et refus800×600. Six assets réellement
servis en fixture protégée concordent avec le profil compilé ; quatre attributions
explicites sont persistées exactement. Recette PASS, intégrité VERIFIED. Le
changement de load/adminForm justifie ce parcours affecté ; ni Rust/CLI inchangés,
ni campagnes152, ni garde61 pour le seul commentaire n’ont été rejoués.

Le dialogue Version en fixture380 passe neuf assertions FR/EN, note/commit/dirty,
Escape/focus et absence d’erreurs. Le rapport interne porte un ancien libellé
installed : son erratum et le wrapper limitent explicitement la preuve à la fixture.
La session humaine Authority réelle et ses ressources protégées restent NON
VÉRIFIÉES ; aucune nouvelle tentative avec clé propriétaire, aucune assertion
fabriquée. Les sondes de version permises ne prouvent pas cette session.

RETEX borné : joindre les réussites complètes, instrumenter les gestes réellement,
conserver les échecs et ne pas déduire une livraison d’un libellé de test. La marge
disque a été rétablie par transfert d’une archive libérée, avant build, réserve
20GiB maintenue. Les pages de suivi modifiées après livraison ne font pas partie
des inputs compilés ; aucun rebuild réflexe pour ces états documentaires.
Swarm00/CI garde la revue runtime finale et l’intégration main ; L05/L06 et le
RETEX global restent ouverts. runtime_enforced:false inchangé.

Le producteur de preuve Version a ensuite été corrigé : scope générique
versiondialogue-owned-endpoint, installation/processus et session humaine à
établir séparément. Ancien driver exact et erratum conservés ; commentaire et
libellés de rapport seuls changés, comparaison bornée/consommateurs/schema contrôlés,
sans rejeu des neuf gestes ni rebuild380 (outil hors inputs compilés).
