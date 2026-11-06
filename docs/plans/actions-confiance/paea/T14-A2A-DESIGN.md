# T14/I14 — conception des échanges agent-à-agent (A2A)

[Plan PAEA](../PAEA-APEX.md) · [English version](../../../en/plans/actions-confiance/paea/T14-A2A-DESIGN.md)

**Statut : conception, non implémentée, non vérifiée.** Ce document traite la
ligne « conception manquante » de T14 (invariant I14) du tableau de bord. Il ne
crée aucune permission, n'active aucune protection et ne clôture aucun
invariant : I14 reste NON ÉTABLI en vérification locale tant que le code
décrit ici n'existe pas et qu'aucune preuve de recette ne le porte, et
l'acceptation reste du périmètre E3 (T18).

## Besoin et ancrage contrat

Registre PAEA :

- I02 Identité/délégation — « Enrôlement/mandat courant → lancement/pair ;
  sous-ensemble des droits délégables » (T03/T14).
- I14 A2A — « Pair/mandat/audience → réception liée et pas d'intermédiaire
  privilégié » (T14/T18).
- Exigence abrégée : « message/délégation A2A authentifiés, liés à mandat réduit
  et provenance conservée ; canal mTLS ou signature selon le modèle ».
- PA-06 Composition — « Instructions/données séparées ; provenance, intégrité/
  confidentialité et versions à lecture/résumé/transmission/compaction ; droits
  descendants bornés et pairs authentifiés ». Verdict attendu : « Pas de
  promotion d'instruction ni de droit par mémoire, A2A, retrait ou reprise dans
  les scénarios retenus ».
- Architecture (PAEA-APEX, diagramme cible) : « Le canal local authentifie le
  pair mais ne déduit aucun mandat de sa seule UID » ; le contrôle et ses clés
  sont hors du domaine agent, sous compte distinct.
- T18 (E3) — multi-tour, renommage d'outil, contexte source→cible, perte de
  labels, résumé/compaction ; oracle : « aucune promotion d'instruction et
  aucun effet hors mandat ».

Prérequis déclarés et état réel :

| Prérequis | État au 02-11-2026 | Mise à jour au 05-11-2026 | Portée |
|---|---|---|---|
| Identité réelle (sorties IA3, enrôlement) | ouverte, humaine | inchangé — ouvert, humain | I02 d'abord ; A2A authentifié réel attend sa sortie |
| `delegation` (schéma `max_depth`) | admission fermée : `max_depth != 0` → `trust-active-extension-not-ported` | **porté** (commit `75c35fa278`, recette `trust-delegation-rust`) : profondeur 0 refuse par défaut, profondeur > 0 plafonnée à la création ; la réception par pair (audience D3) reste à construire | I02 borne les droits délégables |
| Mandats délégués | `parent_mandate_id` non vide → `trust-delegated-mandate-not-ported` (`trust/operations.rs:83`) | **portés** : `parent_mandate_id` vérifié contre la politique (profondeur, révoqué en cascade) ; le refus historique `trust-delegated-mandate-not-ported` n'existe plus dans les sources | I14 les fait circuler entre pairs |
| `data_context` | `trust-advanced-admission-not-ported` (`trust/operations.rs`) | **porté** (2026-11-05, lot provenance, `trust/data_policy.rs`) : admission typée `data_policy`/`data_context` avec refus `authorization_required` et validation au format ; le refus historique n'existe plus dans les sources | provenance conservée des messages, hors A2A réseau |
| Pairs authentifiés (canal local) | UID authentifiée mais aucun mandat déduit de la seule UID (PAEA-APEX, diagramme) | inchangé — l'identification n'autorise rien sans mandat courant | le transport de message D1 n'accorde rien |

La colonne « état au 02-11-2026 » est conservée telle quelle : elle décrit ce
que la conception avait sous les yeux au 2 novembre. Les codes cités
`trust-delegated-mandate-not-ported` et `trust-advanced-admission-not-ported`
n'existent plus dans `rust/crates/swarm-engine/src/` ; le seul garde actif
restant est le garde générique des extensions non portées
(`trust-active-extension-not-ported`, `trust/state.rs`), qui ne concerne plus
ni `delegation` ni `data_policy`.

## Décisions de conception

D1 — **Un message A2A n'est pas une autorité.** Le seul transport d'un message
entre deux instances n'accorde rien : toute capacité reste détenue par le
contrôle (démon d'autorité) et consommée au point d'engagement existant. Un
message sans mandat courant valide est du contexte (I03), jamais un droit.

D2 — **Mandat réduit strictement inclus.** Un mandat pair reprend la chaîne
existante `mandate-create` (admin I3 porteur, TTL, `policy_revision`, actions
énumérées, plafond monétaire, connecteur) : le sous-ensemble délégable est
contrôlé par la politique `delegation.max_depth` (0 = interdiction actuelle
conservée par défaut). Chaque mandat porte son parent et sa profondeur ;
`max_depth = 0` refuse toute délégation, profondeur supérieure est plafonnée par
la politique au moment de la création — pas au moment de l'usage. La vérification
`active()` au point d'engagement reste la seule référence : révision périmée,
révocation ou expiration refusent comme aujourd'hui.

D3 — **Audience explicite.** Chaque mandat A2A désigne son audience : le sujet
pair nommé (identité enrôlée ou principal local), jamais « toute instance ».
La réception vérifie l'audience à l'arrivée : un message délivré à un pair
autre que son audience est refusé à l'admission de lecture (code typé), pas
seulement à l'usage.

D4 — **Provenance conservée, pas requalifiée.** Le message transporte ses
labels d'origine (classement, intention, digest du contenu) ; un intermédiaire
ne réécrit pas les labels, il ajoute sa trace d'acheminement. Une donnée non
classée ou sans provenance lisible reste non privilégiée à la réception (I03/I13
appliquent leurs portes existantes). Aucun résumé ou compaction ne produit un
contenu qui porte plus de droits que sa source.

D5 — **Pas d'intermédiaire privilégié.** Le canal n'élève personne : un relais
A2A qui n'est pas l'audience du mandat ne peut ni lire un contenu qui ne lui est
pas destiné ni engager un effet. L'oracle de réception est « la réception est
liée » : le reçu d'engagement cite le message reçu (digest) et le mandat courant
vérifié à l'engagement, de sorte que l'observateur (PA-09) relie message →
mandat → action → effet sans croire le récit d'un agent.

D6 — **Canal selon le modèle.** Local (même machine, démon d'autorité) :
authentification mutuelle par le démon existant ; la signature du message est
faite par l'expéditeur et vérifiée à la réception, la clef de vérification venant
du contrôle, jamais du message. Distante : TLS mutuel (mTLS) avec certificats
émis par le contrôle ; la forme exacte (mTLS, signature, les deux) est une
décision enregistrée par profil de déploiement, pas un choix d'exécution.

D7 — **Refus typés et UNKNOWN explicites.** Tout échec de la chaîne
authentifier → mandat courant → audience → profondeur → provenance produit un
refus typé observable (`a2a-*`) sans effet et sans mutation de grand livre ;
l'expéditeur voit un UNKNOWN de livraison, pas un succès inventé. La reprise
passe par les opérations publiques (nouveau mandat, nouvelle audience), jamais
par un contournement.

## Chaîne d'admission à implémenter (ordre contractuel)

1. **Authentification du pair** — une seule source d'identité (local ou enrôlé,
   comme `trust/authentication.rs`) ; aucune déduction de mandat de l'UID seule.
2. **Lecture du mandat** — `active()` au révision/clock courant ; mandat délégué
   (parent + profondeur) contrôlé contre `delegation.max_depth`.
3. **Audience** — le sujet du mandat vaut pour ce pair et ce message ; sinon
   refus typé avant toute ouverture de contexte.
4. **Provenance** — labels et digest du message vérifiés (les octets reçus
   correspondent au digest déclaré) ; contenu non conforme refusé, jamais
   réécrit.
5. **Contexte borné** — le message entre dans la conversation comme donnée
   étiquetée (instructions/données séparées, PA-06) ; aucune promotion
   d'instruction depuis un message.
6. **Engagement** — tout effet passe par l'opération existante (`request`,
   `settlement`, …) avec le mandat courant re-vérifié au dernier usage (I05) ;
   le reçu cite le digest du message.

## Hors périmètre de cette conception

- ~~Portage du bloc `delegation` (activation réelle de `max_depth`) — lot
  contractuel distinct, admission de configuration d'abord.~~ Réalisé
  (commit `75c35fa278`, 2026-11-04, recette `trust-delegation-rust`) ; la
  profondeur 0 reste l'interdiction par défaut, conformément à D2.
- ~~Mandats délégués en chaîne (`parent_mandate_id`) au-delà du premier niveau —
  borné par I02 et la politique, non exercé ici.~~ Le premier niveau est porté
  et borné par la politique ; les chaînes profondes restent bornées par
  `max_depth` et non exercées en A2A.
- T18 et toute campagne adaptative — périmètre E3, équipe non auteure.
- Identité réelle des pairs — enrôlement humain (IA3), prérequis I02, toujours
  ouvert.
- A2A réseau (réception par audience, canal D6, reçus liés D5) — reste entier :
  c'est l'objet de ce document, bloqué sur l'enrôlement humain IA3 et
  l'intégration G1 réelle (tronçon dev.322 posé, enrôlement humain non fait).

## Critères de vérification locale (quand le code existe)

- Un mandat pair à profondeur 1 avec audience A : A engage un effet borné, B
  (autre pair) est refusé à la réception (audience), un mandat expiré ou révoqué
  refuse à l'engagement avec le même code qu'aujourd'hui.
- Un message sans mandat courant valide est conservé comme contexte étiqueté et
  n'autorise aucun effet ; l'oracle lit le grand livre : aucun effet.
- Un relais non-audience ne lit pas le contenu (confidentialité BLP existante)
  et ne peut pas engager en son nom.
- Révocation du mandat parent entre réception et engagement : l'effet est refusé
  (dernier usage, I05/I17).
- Chaque refus est typé, observable, sans mutation ; la reprise reste publique.
