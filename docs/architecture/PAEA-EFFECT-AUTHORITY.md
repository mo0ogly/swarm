# Autorité exacte des services d’effet PAEA

## Observation des erreurs après promotion Git

Le chemin Rust de promotion conserve désormais une enveloppe typée d’observation
distincte de la cause d’erreur : résultat durable connu, ou effet possible sans
résultat durable établi. Ce second cas expose le code sûr
`paea-git-promote-unknown` ; il reste une erreur, sans autorisation de réessayer.
Une expiration ou une référence différente observée après la transaction Git
n’établit pas l’absence de publication. Les erreurs SQL originales et les causes
indépendantes de nettoyage restent parcourables sans afficher leurs données
privées. Un résultat enregistré ne doit pas disparaître lors d’un échec de
libération, de fermeture ou de sortie CLI. Une ligne de résultat CLI peut déjà
avoir été écrite avant l’échec ultérieur : le code de sortie reste alors non nul.

La CLI encode le JSON avant toute écriture, puis distingue écriture, saut de
ligne et flush par des codes sûrs, en conservant les erreurs typées originales.
Cela évite de perdre une erreur IO à travers la sérialisation. Un défaut de
fermeture de l’autorité conserve aussi l’enveloppe personnelle/métier complète,
avec le résultat connu et les autres causes de fermeture. Les erreurs JSON CLI
et HTTP exposent séparément `paea_git_effect.state`, avec les valeurs `recorded`
ou `unknown`. Un refus d’autorité avant un nouvel engagement laisse l’histoire
de l’intention inconnue ; il ne prouve pas son absence d’effet antérieur. Une
réconciliation sans ligne d’effet conserve un historique inconnu. Les
diagnostics texte CLI exposent aussi `effect_state`, sans masquer la cause.
Cette normalisation intervient dans le renderer après parcours des wrappers de
fermeture ; les codes sûrs des causes restent visibles en français et en anglais,
sans afficher les messages privés des erreurs imbriquées.
Les gardes de projection portent sur les vrais producteurs JSON en mémoire ;
ils ne prouvent pas la transmission réseau, l’effet Git ni l’installation.

Le [lot article v6](../plans/actions-confiance/ARTICLE-V6-ADAPTATION.md) conserve
le candidat, la reproduction et les contrôles. Ce changement de source ne prouve
ni installation, ni conformité globale PV2. La lecture de référence par Reconcile est intégrée comme candidat source,
sans validation native à ce stade. Un bail d’admission est aussi intégré comme candidat, sans garantie du point de publication ; l’absence de ligne
dans le journal n’établit toujours pas l’absence d’effet Git. L’identification
d’une intention versus d’une tentative doit rester explicite lors de l’oracle.

[English](../en/PAEA-EFFECT-AUTHORITY.md)

Le module Linux `swarm_enrollment_authority::paea_action` distingue les permissions
d’effet PAEA des commandes Trust historiques. Il fournit une capacité empruntée
pendant la réservation du compte courant et de l’autorité métier. Il ne fournit
pas encore de daemon, de service de paiement/Git, de transport authentifié ou de
séparation effective des comptes Linux. La qualification et la livraison restent
suivies dans l’incrément 65 du plan PAEA.

## Permission et liaison

`Request::new(root, service, operation, action)` valide deux empreintes SHA-256
canoniques. Les seules valeurs de service sont `FictitiousPayment` et
`GitPromotion` ; les opérations sont `Commit` et `Reconcile`. Ces constantes
nomment le protocole, sans sélectionner un exécutable, une URL ou une politique.
La permission signée demandée est exactement :

```text
operation: effect.execute
resource: paea-action-v1/<root>/<payment.settle|repository.promote>/<commit|reconcile>/<action>
```

Une requête est une donnée, jamais un droit. Son constructeur ne vérifie ni
stockage, ni action réelle, ni identité. Le futur service doit dériver `root`
d’un témoin conservé de son stockage ouvert et `action` des octets canoniques
PAEA attribués par le contrôle ; il doit les recomparer lors de l’usage avec
service, projet, plan, révision, politique, accord et termes métier courants.
Une permission `trust-action/...`, un autre service, une autre opération ou une
empreinte voisine ne correspond pas. Aucun wildcard ni fallback n’est accepté.
`Reconcile` n’autorise pas un nouvel engagement.

## Usage et erreurs

`AuthenticatedControl::with_paea_action` vérifie le mot de passe du compte
configuré. Le noyau commun privé vérifie le principal et l’installation, conserve
la réservation métier, relit le droit actif signé, vérifie la permission exacte
et l’expiration, appelle une seule fois le consommateur puis revérifie le temps.
Les mêmes contrôles servent désormais au parcours Trust, dont l’API est conservée.
Le verrou et les réservations ne sont pas des garanties de confinement OS.

La capacité `Identity` ne se construit pas par l’appelant, ne se désérialise pas
et ne devient pas un droit persistant par copie de son principal. Ses getters
exposent le droit retenu, le projet signé, le plan, la révision, l’expiration et
la requête exacte ; le consommateur conserve ses propres obligations de contrôle.
Aucun consentement humain réel ne se déduit du mot de passe d’une fixture.

La réponse `Use` conserve séparément `authority` et `action`. `action=None`
signifie que le callback n’a pas été appelé. `Some(Err(...))` conserve le refus du
consommateur, qui peut lui-même contenir un effet connu ou incertain.
`Some(Ok(...))` reste présent si l’autorité échoue ensuite. La couche personnelle
peut à son tour renvoyer `UseError::AfterUse` contenant ce résultat complet.
Inspecter toutes les branches : une erreur ne prouve ni rollback ni absence
d’effet. Les causes de refus et d’échec de journal restent indépendantes.
Pas de retry, timer, relance, initialisation ni réconciliation automatique.

La recette `tools/verification/recipes/paea-exact-effect-authority.json` utilise
deux journaux privés de fixture et des droits signés explicites. Elle contrôle
les permissions exactes et leurs voisins, les namespaces Trust/PAEA, le mot de
passe, le refus du callback, la révocation, l’annulation après usage et le CAS.
Elle réexécute les trois régressions Trust concernées par le noyau commun. Ces
preuves locales ne qualifient ni un effet séparé, ni l’installation protégée,
ni les invariants PAEA globaux, ni E2/E3.

## Réponse d’approbation connectée

Les deux surfaces web connectées (`web/paea-approval-form.js` et
`web/paea-consent.js`) n’interprètent un succès de la consommation POST
d’approbation que sur un état exact `state:"approved"` : l’objet ordinaire
complet et l’accusé rédigé `trust-status-ack-v1` (`content_redacted:true`)
portent tous deux cet état et sont acceptés sans imposer la forme rédigée.
`denied` reste un refus visible avec sa raison ; `admitted`,
`pending_approval`, `committed`, `unknown`, `reconciled` et tout état manquant,
nul ou vide s’affichent en échec fermé et ne sont jamais un succès. C’est une
interprétation de présentation côté navigateur, sans changement producteur ni
serveur ; elle ne qualifie aucune installation ni acceptation globale.

Qualification locale : recette permanente `tools/verification/recipes/paea-approval-state-fail-closed.json` — `python3 tools/verification/recipe.py run --recipe tools/verification/recipes/paea-approval-state-fail-closed.json --output FRESH`, puis `python3 tools/verification/recipe.py verify --evidence FRESH` sur ce même répertoire FRESH produit par la recette (manifeste d’exécution, entrées et sorties brutes conservés dans FRESH) ; 36 contrôles nommés FR/EN attendus, 0 diagnostic. Le navigateur crée séparément son propre TMPDIR privé, frais, imprimé dans la sortie d’étape ; `verify` ne consomme pas ce TMPDIR. Périmètre de preuve : API synthétique et scripts Chrome réels pilotés par gestes DOM ; aucune vérification du service des ressources par l’ELF normal, du pointeur physique ou de la disposition de l’interface n’est revendiquée, et cette preuve ne qualifie ni installation ni acceptation globale.

## Réconciliation Git : histoire et référence observée

Le candidat Rust `Reconciled` retourne `intent_id`, `target_ref`,
`observed_oid` et `recorded_effect`. Ce dernier contient l’action, la référence
et l’OID historiquement journalisés, lorsqu’un effet durable validé existe.
Il reste historique si la référence a depuis changé ou disparu. Une référence
actuellement au candidat ne permet jamais d’attribuer sa publication à cette intention.
Sans ligne d’effet, l’histoire reste inconnue même si Git retourne OLD ou le candidat.

Reconcile ouvre le dépôt lié aux témoins physiques et compare son identité à
l’action préparée, puis lit la référence exacte par le runner Git durci existant
(`symbolic-ref` silencieux et non récursif, puis `for-each-ref`). Le code
de sortie 1 documenté indique une référence non symbolique et autorise la lecture ;
les autres erreurs conservent leurs causes typées. Une référence symbolique pendante
est également refusée. Le journal et Git sont des lectures distinctes, sans instantané
atomique entre les deux stockages. `observed_oid: null` désigne une lecture réussie sans cette
référence ; une référence symbolique, une sortie malformée ou une erreur de
processus reste une erreur typée. Une erreur après lecture du journal conserve
le résultat historique connu et les causes originales, sans observation Git fabriquée.
La libération des ressources conserve les observations et les erreurs indépendantes.

Cette opération ne lance aucune publication, ne réserve pas d’intention et
n’écrit pas le journal. Le transport web exige l’accord capturé pour Commit ;
Reconcile conserve les contrôles de permission, plan, politique et témoins,
sans exiger un nouvel accord Commit. Les sorties CLI et HTTP distinguent
`outcome: reconciled`, l’effet historique optionnel et la référence observée.
Leur forme change : les consommateurs de l’ancien résultat négatif doivent
être adaptés. Aucun droit de nouvelle publication ni installation n’en découle.
Les contrôles composants et la recette native restent des preuves distinctes.

## Bail d’admission de la promotion Git — candidat source

Le match d’accord expose désormais `expires_at`, optionnel, en RFC3339 : il
provient du mandat actif persisté de l’action approuvée, lu sous la même
réservation partagée. Il ne vient ni du navigateur ni d’un champ proposé par
l’agent. Sans mandat, cette métadonnée vaut null et l’expiration signée de
l’identité reste la borne. La chaîne de délégation validée impose déjà au
fils une expiration au plus tard à celle du parent.

Commit retient ce match, puis construit une échéance monotone fixe à partir
de la plus proche expiration signée/mandat. Le monotone est échantillonné avant
l’UTC ; la conversion reste relative à l’UTC observée, supposée digne de confiance
pendant la dérivation. Une fois dérivée, l’échéance n’est plus renouvelée par un
recul de l’UTC. Une expiration exacte ou passée refuse l’admission ; les deux
horloges sont contrôlées avant la transaction et après la lecture sous verrou Git.
Les prévols gardent leurs limites propres ; aucune borne globale FS/SQL du
callback n’est ajoutée ni revendiquée. Un prévol lent ne recrée pas de validité.

Le helper Git réutilise le runner durci et ses causes/sorties. Sa limite est
le minimum du délai opérateur et du bail fourni. La lecture synchrone dans le
callback respecte aussi l’échéance déjà consommée de la transaction parente :
elle ne redémarre pas le délai opérateur de cette transaction. Le contrôle
après publication possible garde la même échéance de bail. Une erreur de
lecture ne devient jamais une absence, même lorsque OLD est nul ; son état
d’effet reste unknown si aucune observation durable n’est établie. Les erreurs
de bail, de processus et de fermeture conservent leurs causes typées.

Le réglage existant `git_command_timeout_seconds` reste stocké dans
`.swarm/engine-settings.json`, validé par `EngineSettingsValues::git_command_timeout`
et consommé par le runner. Il s’exprime en secondes ; zéro expire immédiatement,
null désactive ce seul délai opérateur, une ancienne configuration sans champ
refuse. Le catalogue natif décrit ce champ ; le composant Admin générique, les
routes show/describe/preview/apply et la CLI engine-settings utilisent la même
validation. Ce raccordement est constaté dans le source, sans nouvelle recette
UI exécutée dans ce lot. Le bail n’ajoute aucun réglage, marge, retry ni défaut.
Désactiver le délai opérateur ne désactive pas l’expiration de l’autorité.

La deadline côté client borne l’admission et l’attente contrôlées, pas le point
physique où Git publie après COMMIT : ordonnancement et stockage peuvent dépasser
l’expiration. Le protocole actuel ne transmet pas de bail authentifié imposé au
récepteur à ce point. Le résultat tardif reste inconnu ou historiquement connu
selon les preuves, sans nouvelle autorité. Aucun PA/H2/PV2 global ne découle de
ces gardes. Les horloges injectées et dépôts Git isolés testent des composants ;
le parcours signé natif, le retrait concurrent et le profil livré restent distincts.

Recette permanente : `tools/verification/recipes/paea-git-promote-lease-rust.json`.
Elle déclare douze contrôles composants nommés, le typage des consommateurs et
la compilation Rustdoc. Son inventaire est déclaratif, pas le graphe natif qualifié.
Pour une cible Cargo externe, utiliser le contexte d’admission/maintien de montage
existant avant de lancer Cargo ; la recette n’accorde aucun droit et n’admet pas
elle-même un support amovible. Choisir un dossier de preuves neuf et conserver
les refus originaux. La recette entière n’a pas encore été exécutée dans ce lot ;
les commandes ciblées et leur composition figée sont conservées séparément.


## Observation native du retrait concurrent (fixture)

Les points de synchronisation PAEA sont conditionnés à Linux+test-fixtures,
dans la section partagée réelle et avant le verrou exclusif du retrait. Ils
n’ajoutent ni politique opérateur ni voie d’admission en production. L’image
par défaut f429 a été qualifiée séparément avec features moteur/web/CLI/bin
vides et absence du module fixture dans ses entrées effectives ; cette preuve
ne s’étend pas à des profils arbitraires.

Le scénario signé Q3B-R1 observe contention EAGAIN sur le FD/inode attendu,
Trust inchangé pendant SH, puis engagement enregistré, retrait public effectif,
nouvel usage refusé et Reconcile (histoire/ref distinctes). Un test nommé PASS,
recette à deux étapes et vérification d’intégrité exit0 ; sources et images
liées avant/après. Voir le protocole ARTICLE-V6-CONCURRENT-WITHDRAWAL dans les
plans actions-confiance. Cette observation bornée du retrait coopératif ne
démontre ni H2 globale, ni bail imposé au point physique Git, ni PV2 globale,
ni LLM réel ou installation. Les incidents historiques restent dans le plan.
