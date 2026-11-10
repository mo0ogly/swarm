# PA-03 — raccord des services d’effet : décision de reprise

[English](../../../en/plans/actions-confiance/paea/PA03-SERVICE-INTEGRATION.md)

État au 27 octobre 2026 : analyse de sources, implémentation et qualification à faire.
Propriétaire : Codex direct. PA-02 reste partiel ; aucun invariant global acquis.
Cette décision ne modifie ni le moteur, ni une politique, ni un service exécuté.

## Frontière réellement présente

`AuthenticatedControl::with_trust_action` conserve le compte personnel courant,
la réservation métier et la permission exacte pendant le callback. `Use` garde
séparément l’action et l’erreur d’autorité ; `UseError::AfterUse` peut encore
contenir ce résultat après une erreur personnelle. Toute sortie du futur service
doit traverser ces deux couches sans transformer une erreur tardive en absence
d’effet. Les tests existants de `authenticated_startup` couvrent les droits exacts,
la révocation et l’annulation ; ils ne prouvent pas un service PA-03 installé.

`RootStore::trust_enrollment_request` produit une liaison au stockage ouvert avec
un nonce : son root_digest n’est pas le hash libre d’un chemin. Son témoin doit
rester vivant jusqu’à la clôture explicite. `trust_as_enrolled` compare cette
liaison et les octets exacts, réserve le plan SQL et valide son empreinte courante.
`trust::authentication` compare aussi la révision Trust et l’expiration du droit.
Un nouveau consommateur ne peut donc accepter un simple principal désérialisé ou
remplacer cette liaison par un digest fourni par la proposition.

`trust::operations::effect_transition` écrit actuellement des entrées simulées
dans l’état Trust : local-ledger-simulation et local-access-simulation. Ce ne sont
pas le grand livre SQLite séparé et le dépôt Git exigés par PA-03. Les reçus y sont
indexés par clé de transport ; cette observation ne démontre pas l’unicité métier
PAEA. `protection::canonicalize` fournit déjà l’intention projet+facture, ou
projet+dépôt+référence+ancien OID, indépendamment de la nouvelle clé de transport.
Les contrats PAEA restent des données, sans capacité d’engagement.

## Choix architectural et séquence

Deux options ont été examinées : étendre directement les connecteurs simulés de
Trust, ou raccorder des services distincts au contrôle retenu. La première couple
l’effet et sa preuve dans un seul état et ne satisfait pas la séparation demandée.
Choix : services distincts, réemploi du contrôle courant et du stockage ouvert ;
aucun remplacement immédiat du contrat Trust historique.

1. Définir le contrat de liaison entre action PAEA canonique, projet/plan courants,
   témoin de stockage et service attribué. Vérifier explicitement le namespace de
   permission : une opération Trust `commit` n’autorise pas implicitement tout
   effet PAEA. Aucune nouvelle autorité ou traduction permissive par défaut.
2. Construire le service de règlement fictif à partir d’un stockage privé détenu,
   avec dette résolue par source de confiance, comparaison des termes courants et
   unicité durable de l’intention métier. Garder l’ancien effet en cas de requête
   nouvelle, version modifiée ou refus d’autorité.
3. Construire le service Git attribué : objets immuables, référence directe,
   ancien OID attendu, intention durable avant publication conditionnelle puis
   réconciliation de la frontière Git/stockage. État inconnu conserve le blocage
   d’un nouvel engagement susceptible de dupliquer l’effet.
4. Qualifier d’abord en fixtures isolées : témoin valide avant chaque faute,
   révocation avant première consommation, transport différent, concurrence,
   panne après effet, réouverture, refus sans mutation et reprise. Retenir les
   erreurs indépendantes d’effet, d’autorité, de publication et de clôture.
5. Raccorder au transport protégé et au profil effectivement séparé seulement
   après satisfaction des préconditions PA-02/04. Installer par le chemin local
   pris en charge et vérifier le véritable binaire et les parcours FR/EN.

Les étapes 1–4 ne lèvent pas l’absence observée de newuidmap/newgidmap ni les
approbations humaines/indépendantes manquantes. L’étape 5 reste conditionnelle.
La décision d’un nouveau format de permission doit précéder tout effet ; elle
n’est pas remplacée par cette note. Aucun nouveau délai, budget, retry ou plafond
opérateur n’est décidé. Les limites techniques devront être justifiées au code.

## Preuve et limites de cette reprise

L’inventaire adjacent `PA03-SERVICE-INTEGRATION.sources.json` lie les observations
aux octets lus, pas à un futur build. Aucune compilation ni recette exécutée pour
cette modification documentaire ; aucune installation ou acceptation annoncée.
Les changements concurrents du moteur et leurs processus sont préservés. Le disque
observé ne dispose que d’environ 2,6 Go libres ; aucun cache partagé ni preuve n’a
été supprimé. Avant compilation, réexaminer la capacité et le profil effectif.
Le RETEX global reste ouvert jusqu’aux exigences initiales vérifiées.

Suite : l’incrément65 réalise la permission exacte PAEA distincte et le noyau commun retenu. Le premier inventaire ci-dessus reste historique ; aucun sink séparé n’est encore raccordé. Voir [le contrat actuel](../../../architecture/PAEA-EFFECT-AUTHORITY.md).

66: Separate settlement reference kernel is now implemented and locally verified; enabled separation fails closed pending attributed admission. No protected daemon or public settlement transport. See [current contract](../../../architecture/PAEA-SETTLEMENT.md). Historical observations above remain dated.
