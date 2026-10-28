# Autorité exacte des services d’effet PAEA

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
