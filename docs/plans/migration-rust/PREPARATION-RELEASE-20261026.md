# Autorisation explicite de départ Rust — 26 octobre 2026

## Besoin et choix

Suite du lot conversion, après commit documentaire `e44acb2c0`. APEX/replan,
Codex propriétaire des modifications et GLM consulté en HTTP SSE sans outils.
L'objectif est le parcours préparation convertie → revue du Work actuel →
autorisation explicite, en CLI/API/interface, et non un composant isolé.

Le hash de conversion sélectionne ses tâches ; comme en Go, il ne garantit pas
que leurs instructions n'ont pas été éditées publiquement. Deux options ont été
comparées : refuser toute dérive (durcissement et mécanisme de réparation nouveaux)
ou conserver les éditions et présenter le Work courant avant confirmation. La
seconde est retenue pour cette migration : revue protégée, contrats actuels à
déplier et CAS sur la révision affichée. Aucun refus caché sur une édition de tâche.
Un nouveau brouillon n'est jamais converti implicitement par `release-plan`.

Le premier périmètre exige que les IDs convertis couvrent toutes les tâches du
Work, sans doublon. Les conversions partielles et la révision de missions restent
non portées : refus explicite `preparation-release-scope-unported`. Cela garde le
contrat de la conversion autonome déjà livrée ; aucune parité Go globale annoncée.
L'organisation n'est pas configurée par la commande. Aucun budget ou autonomie
n'est modifié ; un conducteur déjà configuré peut devenir éligible au départ.
L'autorisation ne prouve ni lancement, ni identité humaine nommée, ni acceptation.

## Contrats et contrôle des trois axes

Politiques : version1, lock0/1, révision positive et format d'ID haché sont des
invariants du protocole conservé. `checked_add` protège la représentation i64 ;
aucune borne métier de révision n'est ajoutée. Les paramètres Admin d'écriture et
la réservation existante restent l'autorité ; aucun nouveau timeout, retry,
plafond, limite de concurrence ou fallback opérateur. Les valeurs des tests et
largeurs navigateur sont des entrées de fixture. Le domaine whole-Work est une
limitation fonctionnelle annoncée, pas un plafond opérateur.

Fonctions : `release`, `Witness::verify`, `apply`, `preparation_command::validate`,
les adaptateurs CLI `execute/write`, les routes `query/preparations` et les
contrôles d'état, rendu, revue/confirmation et `show` de l'interface sont modifiés.
Leurs contrats documentent les révisions, effets, erreurs, absence de dispatch,
lecture séquentielle non atomique et issue incertaine. Les docstrings des nouveaux
pilotes de recette et helpers décrivent fixture/IO/originaux et limites des preuves.
La revue sémantique finale est liée aux empreintes dans `source-review.json`.

Erreurs : wire et CAS refusent avant publication ; les locks sont modifiés
conditionnellement dans la transaction immédiate. L'événement, le Work, la
préparation et le reçu sont relus après toutes les écritures, avant commit.
Une erreur SQL, de lecture, de rollback, de commit ou de fermeture garde ses
causes à travers les wrappers existants. Les conflits Work sont exposés en CLI3
et HTTP409 ; les périmètres non portés en HTTP501 ; les incohérences stockées ne
sont pas converties en succès vide. Après issue inconnue, l'interface garde
l'événement exact et le rejoue. Un refus prépublication connu permet une nouvelle
revue ; l'incertitude ne permet pas d'inventer une nouvelle intention.

## Replans et preuves originales

- Baseline dev118 : `release-plan` est refusé sans changement SQL.
- Dev119 compile et passe les premiers refus CLI. La première recette attendait
  un champ `launch_held:false`, alors que le DTO Go-compatible omet les booléens
  faux. Le test compare l'absence à false et l'état logique complet ; le moteur
  n'a pas été modifié pour satisfaire cette mauvaise hypothèse.
- La seconde exécution passe CLI/API puis révèle un défaut de focus : la revue
  asynchrone désactive le bouton avant de mémoriser `document.activeElement`.
  Le helper commun accepte désormais un déclencheur explicite ; cette revue
  transmet le sien. La garde Échap/focus et son échec dev119 sont conservés.
- Dev120 conserve un second défaut : lors de close, le focus peut encore être sur
  le summary du dialogue fermé. La garde ne restaurait que body/dialog, omettant
  ses descendants ; un diagnostic isolé observe BODY même après deux frames.
  La restauration couvre maintenant le descendant du dialogue fermé, sans voler
  le focus à un autre contrôle extérieur. Échecs et diagnostic sont conservés.

## Vérification et limites

La recette permanente `preparation-release-rust` utilise un Root isolé créé par
les opérations publiques, conserve chaque argv/stdout/stderr, teste refus sans
mutation, reprise, nouveau brouillon, éditions publiques, double CAS, verrou,
HTTP CSRF et le parcours navigateur FR/EN × clair/sombre à 1440/390 pixels.
La perte de réponse est injectée après un vrai commit serveur, puis le même
événement est rejoué. Aucune mission locale ni aucun fournisseur réel ne sert de
fixture. Le test interne du Witness couvre création et release, avec témoin valide
avant chaque dommage tardif Work/lock/event, rollback et reprise.

## Résultat installé et RETEX du lot

`release-plan` est raccordé en CLI, API et interface native. Le service local
observé exécute `v0.1.1-dev.121`, commit `e44acb2c0`, `modified=true`, SHA256
`87479ae625e55bf37322b71e46655d1b721702ffbd04f86b9b9f9d9281e033dd`.
Le commit demandé avant reprise ne contient que les deux documents précédents ;
les changements du présent lot restent non committés. Aucun push.

La recette finale sur CE binaire passe 52 assertions CLI/API et42 navigateur.
Elle comprend deux tâches dépendantes : témoin valide, échec sur le second lock
avec rollback du premier, puis reprise avec le même événement. La première
extension de fixture avait masqué la fonction task par un dictionnaire local ;
un alias de fabrique explicite corrige le pilote, sans changement produit.
La non-régression conversion passe50+51 assertions sur le candidat antérieur,
et la recette de libération passe aussi87 assertions sur ce candidat. Le seul
delta effectif ultérieur concernait un prérequis de lancement, hors dispatch
synchrone de release ; la recette finale actuelle ne dépend pas de cette reprise
comme substitut à son propre résultat. Le test Witness passe1 test nommé couvrant
création/release et les dommages tardifs Work/lock/event. Les traductions passent.

Les deux premières constructions refusées sont conservées : manifeste Cargo
concurrent en avance sur son lock, puis trois entrées effectives modifiées pendant
la compilation. `cargo metadata --locked --offline` passe après synchronisation du
lock par son propriétaire. Notre première tentative de restart refuse à son tour
un changement d'entrées candidates et conserve le serveur118. Le redémarrage
concurrent suivant, par le même lanceur pris en charge, active121. Son binaire est
ensuite vérifié directement ; aucune deuxième interruption inutile n'est lancée.
Les478 entrées effectives sont identiques au candidat de livraison précédemment
testé ; seuls les métadonnées injectées et le SHA du binaire diffèrent.

Sur le service réellement actif :16 contrôles HTTP/processus/ressources +14
assertions Version FR/EN +6 préparation en lecture seule passent. Les six ressources
contrôlées correspondent aux entrées compilées ; les nouvelles notes FR/EN sont
présentes, le commit réel et l'état modifié sont affichés. Aucun POST de recette
sur l'instance locale. Réglages identiques, et révision/pause/autonomie/slots/
consentement/archive des deux missions conservés. Le profil livré est normal,
sans feature de fixture ; empreinte du processus, du reçu et de l'archive concordent.
Les recettes finales retenues sont vérifiées par `recipe.py verify --evidence`.

GLM : le premier cadrage trop large a produit une longue sortie de raisonnement
répétitive sans avis final et a été interrompu côté client. Originaux conservés,
consommation et annulation serveur non établies. La revue réduite aux deux fonctions
transactionnelles aboutit après516,189s :2035 tokens d'entrée,23636 de sortie,
25671 au total, identifiant demandé/déclaré `glm-5.2`. Conclusion : aucun défaut
démontré, aucun test exécuté par le modèle. Les limites d'identifiants et de
planning citées ont été recoupées avec le validateur et le Go fournis ; ce n'est
ni une acceptation moteur indépendante ni une revue globale de l'application.

Apprentissages vérifiés : attendre l'événement natif close avant d'affirmer le
retour de focus ; couvrir le focus encore dans un descendant fermé ; consommer
les omissions DTO au lieu d'exiger des booléens faux ; vérifier le schéma du
producteur d'inventaire avant de lire un champ ; arrêter les étapes dépendantes
après un prérequis refusé. Les reprises de lecture de métadonnées et la première
invocation verify avec mauvaise option sont des erreurs de notre méthode, pas
des défauts Rust. Elles ne sont pas comptées comme des contrôles réussis.
Les recettes permanentes et ce rapport transmettent ces apprentissages aux
prochaines exécutions ; aucune modification des contextes natifs déjà lancés
n'est supposée. Une revue GLM ciblée peut rester lente : aucun gain de coût
ou de vitesse n'est revendiqué sans témoin.

Limites restantes : conversions partielles, organisation, révision des missions
et dialogue IA restent hors de ce lot ; pas de parité différentielle Go globale,
de fournisseur réel, d'identité humaine nommée ou d'acceptation utilisateur déduite.
Les budgets et droits réels restent requis au lancement. Le chantier général et
son RETEX final restent ouverts. Preuves et SHA de revue dans
`evidence/20261026-preparation-release/` ; originaux privés hors cache, ignorés Git.

Le diff de notre périmètre passe `git diff --check`. Le contrôle global reste
refusé sur deux lignes vides finales préexistantes hors lot :
`DELIVERY-PREPARATIONS-20261025.md:85` et
`task_validation/managed_receipt.rs:293`. Ces octets ne sont pas modifiés ici ;
leurs propriétaires doivent les traiter dans leur incrément. Le refus global
est conservé et ne devient pas un PASS par la réussite du contrôle ciblé.
