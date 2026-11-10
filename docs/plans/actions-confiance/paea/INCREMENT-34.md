# PAEA34 — déclaration indépendante et ouverture du raccord authentifié

## Reprise et choix (plan, non réalisé)

33 :39tests/5étapes PASS, archive VERIFIED, Rustdoc et diagrammes FR/EN PASS,
revue locale liée aux20fichiers/172liens. Erreur Debug privée reproduite/corrigée,
double autorité et deux erreurs de close observées. Normal Cargo réussi mais
profil refusé pour deux sources web concurrentes ; preuve conservée, dernière
installation observée dev129. Ne pas réessayer sans précondition changée.

Le loader bootstrap actuel possède un seul parcours read/hash/canonicalisation/
contrôles d'owner/modes/types/NOFOLLOW/FD/close, décliné v1etv2 via accesseurs typés.
Réemployer ce parcours en lui donnant un layout de racines/journaux déclarés par
le codec, plutôt que copier un second loader. Aucun callable fourni par JSON.
L'ouverture34assemble les deux noyaux33 depuis ce document indépendant et refuse
les sources confondues. Cela ne démarre pas encore un listener ni un compte humain.

## Tâches et critères observables

34a : schéma fermé/canonique v3 AuthenticatedDocument, ControlDocument v2imbriqué
inchangé et PersonalDeclaration explicite : journal_path, checkpoint_directory,
authority_public_key, project_context_digest, generation et bootstrap_head positif
indépendant. Expected personnel installation/principal provient du même contexte
protégé installation, jamais d'une requête ; la génération personnelle est distincte
du mandat. bootstrap_head est le témoin initial de provisioning, pas un head actuel
à adopter après enrôlement. Aucun secret/clé privée/KDF par défaut dans ce fichier.
Valider deux clés/projets, head/génération positifs, chemins absolus canoniques et
absence de recouvrement entre les5racines de stockage (2config/2journaux/1checkpoint).

34b : layout interne emprunté explicite dirs+journals pour loader v1/v2/v3. Pin SHA256
exact fourni indépendamment, même contrat qu'aujourd'hui ; aucun fallback de version.
Bootstrap parent séparé de toutes racines, dossiers privés de l'euid service,
comparaison dev/inode de toutes racines détenues et fichiers journaux réguliers0600
single-link existants. Aucun SQLite, lock, directory creation ou mutation lors du
load ; toute erreur de lecture/stat/close et ses secondaires conservées.
Conserver v1/v2etleurs hooks ; pas de régression par refactor général.

34c : authenticated_startup::open charge/valide le document, ouvre le Control métier
et le Source/Coordinator personnel, et les remet au wrapper propriétaire33avec le
head métier indépendant attendu. Une ouverture partielle ferme tous les objets déjà
acquis et conserve échecs primaires/secondaires. Les opérations restent liées aux
connexions SQLite réellement retenues, pas un pathname/UID réutilisé par requête.
Ne pas qualifier une observation de fichier comme authentification humaine ou OS
confinement. Aucune adoption de checkpoint manquant, bootstrap ou clé privée créée.

34d : cas API absente puis codec exact/faults sans canonicalisation masquante,
pins/principal/install/générations/domaines, recouvrements lexicaux et racines
physiques, modes/liens/types/owner, read+postread+close, refus sans création de lock,
recovery et v1/v2inchangés. Témoin valide avant chaque injection. Recette permanente,
Rustdoc, docs FR/EN, revue politiques/fonctions/erreurs liée aux octets ; profil normal
et installation toujours requis après stabilité des sources parallèles.

Suite : lifecycle/provisioning initial CLI protégé (signer et preuve d'installation
hors agent), délégation enroll/recover sans exposition de Control brut, service
local/transport et UX, puis compte OS séparé/confinement et fermeture des anciennes
routes. Ne pas ajouter une API de preuves client ni un acteur libre. Rotation/
récupération du secret personnel demeure le suivi explicite33, avant clôture globale
PA-02 ; aucune nouvelle identité/journal ne contourne une révocation. PA-02partiel,
invariants0/17, E2/E3/humain/PA-03…13non clos. Codex propriétaire, sans GLM/délégation.


## Codec implémenté, chargeur encore à réaliser

API absente reproduite (0scénario),3tests purs initiaux PASS. Déclaration v3fermée
imbriquée, générations personnelle/métier distinctes,5noms de racines sans recouvrement,
contextes projet personnel/métier distincts (séparation de protocole, pas politique
opérateur), même clé protégée autorisée. Validation Pins partagée conservée comme
Binding typé avec catégorie document sûre/source, aucun défaut crypto/dépendance ajouté.
Aucun IO/chargement/observation de racine/enrôlement dans ce codec. Étendre maintenant
le layout du chargeur existant ; leurs v1/v2 restent inchangés à ce stade.


### Composant de chargeur protégé vérifié

6scénarios v3+1alias FD réel+4v2+5v1=16tests nommés/4étapes PASS dans
bootstrap-recipe-01.3flags fixture existants réutilisés, chaque descripteur acquis
fermé explicitement ; alias physique testé sur vrais FD détenus après témoin distinct,
pas montage privilégié ni confinement OS. Documentation courante corrigée :32n’est
plus présenté comme coordinateur durable futur ; anciens octets conservés ici et
chronologie dans les plans. Adaptateur startup des deux sources encore à réaliser.


### Ouverture des deux sources vérifiée localement

3scénarios réels de startup PASS ; quatrième défaut de cleanup PASS : même refus
primaire de contexte métier avec/sans défaut, erreur indépendante de close personnel
conservée, reprise réussie. Aucun humain/service réel. Resolve startup réemployé privé
au crate sans changement de comportement ; feature fixture config propagée explicitement
pour ne pas masquer un support manquant par all-features. Chargeur16tests/4étapes
PASS/intégrité VERIFIED ; agrégat59tests/10étapes à qualifier (16bootstrap+4startup+
39composition/credentials/consommateurs). Opens partiels peuvent créer des locks,
contrairement au loader metadata. Livraison normale encore à établir.


## Qualification et livraison34 observées

Recette agrégée59tests nommés/10étapes PASS, intégrité VERIFIED :16bootstrap,
4startup et39composition/consommateurs. Anciennes v1/v2 préservées. Dernières corrections
uniquement de commentaires de contrat : bridge byte-à-byte conservé, instructions
Rust inchangées ; Rustdoc courant PASS. Profils de bibliothèque distincts contrôlés :
fixture active config+authority test-fixtures, défaut n’active ni l’un ni l’autre.
Diagrammes FR/EN rendus et relus ; revue locale liée aux octets, sans indépendance.

Own build135 refusé pour module de préparation parallèle temporairement absent,
preuve entière conservée. Une autre livraison supportée a activé135 ; ses13sources
PAEA de production correspondaient exactement. Les commentaires courants ont ensuite
été construits dans le candidat supporté136,515sources effectives archivées/relues,
features fixture absentes.6tests du harnais CLI figé PASS :5parcours processus sur
le candidat exact,1témoin bibliothèque ancien, distinct des59tests actuels.
Redémarrage supporté réalisé ; PID2562141, `v0.1.1-dev.136`, SHA256
`8102c9abfa0757ed8680a156efb7f636e786c034b863d9f410bc3ab0d21ef631`,
commit cbd6fecd7e40016d6c259296ac3a21373a4bbd60, modifications signalées.
56assertions navigateur protégées en lecture seule et garde de refus du script
manquant PASS, Version FR/EN/thèmes état-sombre/lien commit/notes/captures vérifiés.
[Preuves34](evidence/20261026-authenticated-startup34-wqrdnklk/).

Cette livraison contient les sources internes32/33/34 ; elle ne démarre pas de
service personnel et n’enrôle aucun humain réel. Custody clé, provisioning initial,
lifecycle public, transport/UX, compte OS séparé, confinement et médiation effective
des deux domaines restent à réaliser. PA-02partiel,0/17invariants globaux ; E2/E3 et
PA-13non clos. Aucun commit/push. Suite35 : clé de signature protégée et lifecycle.
