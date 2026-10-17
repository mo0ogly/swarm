# R01/R13 — couverture locale après introspection des services

Extension du 16 octobre : la [création et la légitimité des agents](identites-agents-replan.md)
ajoutent les scénarios IDA-01–18 à R01/R13. **NON TESTÉ pour ce nouveau
périmètre** : identité de l'instance, délégation bornée par le plan, validation
humaine hors plan, revalidation avant départ et protection du plan de contrôle.
Les PASS historiques ci-dessous restent limités à leurs consommateurs et
ne constituent pas une qualification de cette extension.

15 octobre, propriétaire Codex. Évaluation du contrat produit existant, distincte
 d’une revue indépendante, d’une recette fournisseur réel et de la diffusion.

| Critère | Source actuelle | Preuve | État |
| --- | --- | --- | --- |
| Identité authentifiée et contexte commun | trust_identity_context.go, trust_actions.go | identity-context-verification.json ; régression identités246 | PASS local |
| Personnes OIDC Code/PKCE, issuer/aud/nonce/signature/dates | trust_oidc.go | régression246 ; recette humaine26 | PASS local |
| Services private_key_jwt + access token distinct | trust_workload.go | régression246 ; recette service14 avec effet exact | PASS local |
| Cycle de vie et révocation locale | trust_identity.go, résolution commune | régression246, suspension/révision/logout/expiration | PASS local |
| Mapping versionné, plafond local et rôles effectifs | trust_group_mapping.go |32 cas finaux ;26 humain/14 service | PASS local |
| Rotation de clés humaines et services | trust_oidc_test.go, trust_workload_test.go | cas key-rotation dans246 nommés | PASS local |
| Délégations, séparation, step-up exact | trust_delegation.go, trust_step_up.go |246 ;32 mapping dont step-up dérivé ;10/20 navigateur | PASS local |
| Révocation fournisseur humaine signée | trust_oidc_logout.go |246 et18 navigateur, connexions en vol comprises | PASS local |
| Détection d’un access token de service révoqué seulement chez son émetteur | trust_introspection*.go, trust_workload.go, résolution commune et routes publiques | introspection-verification.json :41 cas dédiés et18 contrôles publics FR/EN ; révocation en vol et après redémarrage | PASS émetteur isolé |
| Intégration des consommateurs T4/T5/T6 | contexte commun produit, T4 courtier le consomme | octrois du helper dans les régressions ; processus et preuves étendues à intégrer dans leurs lots | PARTIEL, hors clôture R13 locale |

Le reçu courant est [introspection-verification.json](verification-confinement-20261015/introspection-verification.json) :570 cas Go distincts (145 principaux,425 sous-cas),
issus des groupes252/319, helper vide exclu ; pas une exécution unique570. Les41
cas d’introspection sont inclus.172 contrôles navigateur et sept scénarios publics
passent sur le binaire final. Aucun diagnostic inattendu ; refus volontaires attribués.
Les originaux des échecs restent conservés. Les preuves historiques de mapping
restent attribuées à leurs entrées, sans remplacer ce nouveau parcours observé.

R13 est vérifié localement avec un émetteur isolé. Il ne s’agit ni d’une activation
chez un fournisseur réel, ni d’une acceptation indépendante ou globale. La fenêtre
administrée borne la décision après réponse, sans garantie d’instantanéité entre
systèmes. Redémarrage et révision exigent nouvelle clé publique enregistrée et login.

## Révocation du fournisseur de services — décision de progression

Fait : suspension locale, révision, logout, échéance de session et exp signé sont
contrôlés. Une révocation distante non transmise n’est pas détectée avant ces
bornes. Ce constat ne constitue ni panne de mapping ni révocation instantanée.

Comparer borne locale actuelle, introspection authentifiée et notification signée
spécifique fournisseur. La borne locale reste la protection courante, explicitement
limitée. Retenir l’introspection standard comme évolution après définition des
secrets et destinations contrôlés de T4. [RFC7662](https://www.rfc-editor.org/rfc/rfc7662.html#section-2.2)
permet de demander l’état actif d’un token ; la requête transporte le token et
requiert une autorisation. Le token ne doit donc pas être ajouté en clair à l’état
métier ou aux preuves pour simuler cette capacité.

Propriétaire Codex. Condition concrète : contrat de secret référencé/éphémère et
destination contrôlée, puis fixture d’introspection authentifiée : actif → demande
permise ; révoqué → refus sans effet ; indisponible/malformé → refus fermé ; aucun
secret persistant dans les requêtes/proofs et aucune attente réseau sous le verrou
métier. Timeout/fraîcheur éventuels persistés et éditables Admin, sans plafond caché.
Aucune activation externe ni nouvelle permission ne découle de cette conception.

## Prochaine tranche autorisée

Les jointures T4 `data-policy-v1` et `secret-grant-v1` sont désormais détaillées
dans `actions-confiance-contract.json`. La première admission des données est
implémentée et vérifiée dans le parcours simulé ; le courtier de secrets reste
une cible non implémentée. Voir [la reprise T4](data-policy-replan.md) et son
[reçu](verification-donnees-20261015/data-policy-verification.json).

Poursuivre le courtier référencé/éphémère et le confinement : les connexions IA
existantes ne prouvent pas une attribution de secret par action. Définir les
assertions de consommation unique, identité/digest/révision, destination, refus
sans secret conservé, redémarrage et concurrence avant modification. Puis revenir
à l'introspection authentifiée des services, hors verrou métier. Conserver les
autres lots et leurs limites jusqu'au RETEX ; R13 n'est pas complet.

## Reprise du 15 octobre 22:02 UTC — introspection des services

Le courtier éphémère est livré et vérifié dans sa portée de helper fixe (reçu
`verification-confinement-20261015/secret-broker-verification.json`). La section
précédente qui le disait absent décrit la décision antérieure, désormais dépassée.
Il ne devient pas un connecteur réseau d’introspection.

R13 : le jeton de service vérifié est actuellement jeté. Deux options comparées :
notification signée spécifique fournisseur (dépend d’un protocole tiers) ou
introspection RFC7662 authentifiée (contrat standard, requête à chaque usage).
Retenir la seconde, sans cache de réponse : jeton uniquement en mémoire serveur,
clé privée éphémère propre à l’introspection, clé publique à enregistrer chez
l’émetteur isolé, endpoint exact administré sur la même origine que l’émetteur.
Aucun fournisseur externe activé. Le redémarrage exige une nouvelle inscription
de cette clé et une nouvelle connexion ; aucun repli sur les sessions persistées.

Plan Codex : (1) garde avant modification et politique stricte persistée/Admin ;
(2) transport sans proxy/redirection, assertion private_key_jwt dédiée, réponse
liée aux issuer/sub/audience/client/exp validés ; (3) preuve interne par requête,
réseau hors verrou puis revalidation sous verrou pour contexte/opérations/octrois ;
(4) recettes CLI/API/UI isolées, refus sans mutation, indisponibilité, révocation,
concurrence/reconfiguration/redémarrage, absence du brut en état/export et RETEX.
La fenêtre entre réponse et décision est une politique visible, pas un cache ni
une garantie de révocation instantanée entre deux systèmes. Timeout, fenêtre,
capacité et taille sont persistés avec unités et zéro explicites.

Le premier contrôle nommé `IntrospectionPolicyContract` attend la politique via
le vrai validateur de configuration. Aucune acceptation R13 annoncée avant le
parcours public actif → effet exact → révocation → refus sans nouvel effet.

Incident lié R13-fence : le contrôle `introspection-fence-before-race.out`
reproduit une preuve positive encore utilisable après réception de `active:false`.
Cause : preuve isolée par requête sans invalidation commune. Correction choisie :
témoin atomique de révocation lié à la matière ; retrait, échéance et changement
de révision invalident aussi les preuves en vol. L’état métier reste inchangé lors
du refus distant. Propriétaire Codex ; fermeture conditionnée au rejeu de la garde.
Les recherches sur un nom de fichier supposé ont aussi échoué sans mutation ; les
lectures suivantes utilisent l’inventaire `rg --files` et les symboles réels.

Replan des témoins : première recette navigateur refusée par l’émetteur de fixture
qui exigeait une audience JWT sous forme de liste ; la bibliothèque produit une
chaîne valide pour une seule audience. Le moteur a correctement refusé la réponse
d’erreur de cette fixture. Normalisation chaîne/liste dans le témoin uniquement.
Le test de timeout attendait sans borne la fermeture du contexte de sa requête
sans lire son corps ; `httptest.Server.Close` restait actif. Groupe propriétaire
identifié puis arrêté, reçu d’interruption conservé ; ce run n’est pas PASS. Le
témoin a maintenant une sortie indépendante à deux secondes, supérieure au délai
d’introspection configuré à une seconde. Les originaux restent conservés.

Réexamen d’hypothèse, 22:22UTC : `io.ReadAll(response.Body)` semblait non borné
à la lecture locale du login. Le contrôle public `--token-response-bound` refuse
une réponse OAuth avec une enveloppe de plus de1Mio, sans session publiée. La
lecture du producteur `trustOIDCTransport.RoundTrip` établit que `trustOIDCBody`
borne déjà ce flux à1Mio. Aucun défaut moteur démontré, aucun correctif ajouté.
La recette conserve cette garde, avec quatre échanges (deux connexions positives
et deux réponses surdimensionnées refusées) et18 contrôles FR/EN au total. Le
nom historique de son reçu `introspection-envelope-before-services-delivery.json`
désigne une hypothèse testée, pas un échec ni une correction.

Incident R13-legacy-lock : le groupe `introspection-final-regression-t3` termine
FAIL (à conserver), car les deux variantes de `IdentityContextClockAfterLock`
répondent avant libération du verrou. Cause établie : prélecture ajoutée via
`withTrustReadAvailable`, qui introduisait LOCK_NB dans ces parcours historiques.
Options : changer le contrat d’attente ou préserver sa sérialisation. Retenir la
seconde : snapshot par `withTrustLock(false)` ; réseau toujours après libération,
puis horloge et autorité relues sous le verrou de la décision. Aucune suppression
de garde ni adaptation du témoin pour faire passer le moteur. Codex propriétaire ;
rejouer le test d’horloge puis le groupe T3 et les parcours publics sur le nouveau
binaire. Les252 cas intérieurs T2/T4 non modifiés gardent leur reçu initial, avec
leurs sources, distincts de cette nouvelle exécution et sans prétendre un run global.
