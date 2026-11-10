# IA3c.2f-01 — décision local pilote, distant prévu

17 octobre 2026. **Décision adoptée et documentée ; service et UX non livrés.**

L’utilisateur retient le service local sous compte distinct avec confinement
pour le développement/pilote, et demande de prévoir une autorité sur machine
séparée pour l’avenir. Installation initiale et modal Admin devront gérer le
choix. [Décision conservée](decision.json), [plan canonique](../../identites-agents-replan.md)
et [contrat cible](../../identites-agents-enrolement-IA3c2.md).
Aucun besoin client mesuré n’est déduit de cette anticipation.

## Sources et décision

install.sh gère Docker/natif et écrit chemins/port/UID/GID dans install.env ;
compose.yaml définit un service Swarm avec l’UID/GID opérateur. Aucun compte ou
service d’autorité dédié n’est installé. trustRemoteOrigin exige une origine
HTTP loopback et un port explicite : le client Trust n’est pas le futur canal
d’autorité distante. web/admin.js montre un modèle existant de preview/révision,
pas une configuration d’autorité déjà présente. Sources liées par empreinte
avant/après dans [baseline](baseline.json) et [contrôles](checks.json).

Contrat commun retenu plutôt qu’accès local direct au journal. Il doit permettre
un adaptateur local puis distant sans divergence des droits ni fallback permissif.
L’UX cible sépare choix enregistré, état appliqué et preuve effective. La migration
ultérieure doit conserver historique, réservations, consommation et checkpoints,
sans importer sa racine depuis un projet ou apprendre une clé du pair.
Le principal humain initial reste à désigner ; aucun bootstrap réel n’est exécuté.

## Périmètre et vérification

Six documents modifiés : plan, cible d’enrôlement, contrats FR/EN et installation
FR/EN. [Avant](before-docs.tar.gz), [diffs](documentation.diff),
[après](after-docs.tar.gz). Aucun source moteur, installateur, Compose, config,
frontend, consigne commune ou processus modifié. Pas de fournisseur, création de
compte, clé durable, service, commit, déploiement ni revue indépendante.

La cible et les contrats exigent configuration protégée persistante, validation
commune setup/API/CLI/Admin, preview sans mutation, contrôle de révision,
sauvegarde/réouverture et refus visibles. Ces exigences restent non implémentées.
Le choix Docker/natif est distinct du choix local/distant ; le mode pilote ne
retire aucun contrôle. Installer avec succès ou obtenir un healthcheck ne prouve
pas l’isolation. Les schémas sont explicitement cibles ; le distant est prévu.

Les contrôles documentaires sont dans checks.json : cohérence de la décision,
liens locaux, whitespace, préservation des références et empreintes des archives.
Rendus FR/EN dans diagrams.json et inspection dans visual-inspection.json.
Les tests Go précédents ne sont pas réexécutés : aucune entrée de ces modules
n’est modifiée ; leur preuve reste locale et ne s’étend pas au service cible.
Aucune recette navigateur n’est revendiquée : aucun écran livré ou modifié.
Le checker de configuration des agents n’est pas applicable, aucun de ses inputs
n’est modifié. Les exigences setup/API/CLI/Admin seront testées à leur réalisation.

## Replan et améliorations

Deux recherches initiales ont inclus des noms de fichiers supposés et ont retourné
une erreur de chemin absent ; inventaire réel puis lecture ciblée des fichiers
existants, sans relance inchangée. Une sortie de lecture groupée trop longue a
été bornée en réduisant les lectures suivantes. Commandes, extraits disponibles, limites de conservation et cause dans
[incidents de découverte](discovery-issues.json). Cela ne prouve aucun défaut du
produit ; la consigne existante « chercher les chemins réels » suffit, sans ajout
redondant de règle commune.

I2f-01 (utilisateur, éviter un verrouillage local) : décision et contrat documentés,
portabilité réelle ouverte. I2f-02 (audit du parcours, éviter un faux état protégé) :
choix/appliqué/preuve distingués dans le contrat, effet runtime/UX ouvert.
I2f-03 (inspection, éviter la confusion Docker/natif et local/distant) : distinction
transmise dans les deux guides, vérification produit future. Propriétaire Codex ;
les droits d’activation et le principal exact relèvent de l’opérateur.

Seul IA3c.2f-01 est réalisé dans cette tranche documentaire. -02 à -06 et les
préconditions EP-01 à EP-10 restent ouverts/partiels selon leurs preuves
antérieures. Prochain travail technique : définir schéma/stockage protégés et
opérations publiques de configuration, puis service local et preuves OS avant
raccord moteur. Le principal et les preuves OS sont nécessaires à l’activation,
pas à ces travaux préparatoires. F01–F07, IDA, IA3 public, IA4/IA5 et RETEX global
restent ouverts ; aucune supériorité scientifique n’est démontrée ici.
