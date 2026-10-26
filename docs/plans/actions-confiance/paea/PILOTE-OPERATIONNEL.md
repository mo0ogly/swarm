# PAEA — dossier des deux pilotes opérationnels

[English](../../../en/plans/actions-confiance/paea/OPERATIONAL-PILOT.md) ·
[Plan](../PAEA-APEX.md) · [Guide intégré](GUIDE-INTEGRATION.md)

## Portée et responsabilités

Ce dossier prépare PA-00/01/02/03/09 à partir du guide fourni. Les services
exécutables de `tools/paea/pilot_lab.py` sont des **fixtures de laboratoire** :
effets réels dans leurs fichiers isolés, autorité simulée, même UID. Ni service
PAEA protégé, ni paiement réel, ni déploiement dans l’instance locale.
`pilot_oracle.py` est un processus et un chemin de lecture séparés ; son auteur
et ses droits ne sont pas indépendants. Aucun E2/E3 ou invariant global démontré.

| Décision / rôle | Laboratoire actuel | Exigence pour le produit |
|---|---|---|
| Intégration et recette | Codex racine, seul | Garder un propriétaire du besoin et les preuves attribuées. |
| Responsable métier / risque | **Non attribué** ; aucune classe C0…C4 décidée | Identité habilitée, conséquences, périmètre et risque accepté enregistrés. |
| Approbateur humain | **Absent** ; table de grants simulés | Identité réelle, droits, contenu exact et politique courante ; revue IA distincte. |
| Contrôle d’identité / clés | **Absent** ; noms des vecteurs de test | Sorties IA3/PA-02, compte distinct, ouverture de projet liée et clés hors agent. |
| Responsable du service d’effet | Rôles PA-03 registre métier / dépôt géré | Compte/service effectif, droits, point d’engagement et procédure d’incident. |
| Évaluateur indépendant | **Absent** | Désignation et droits E3, seconde implémentation et B2 préenregistré. |

Ces manques sont des préconditions de réalisation, pas des décisions à remplir
automatiquement depuis l’exemple F-12/4200 EUR. L’implémentation protégée et
l’activation protégée restent conditionnées par leurs dépendances. La restriction
de compilation a été levée et dev80 installé à l’incrément 15. Le laboratoire
reste indépendant ; sa séparation de processus ne remplace pas ces dépendances.

## Fiches d’effet

| Champ | Règlement fictif | Promotion Git |
|---|---|---|
| Conséquence mesurée | Ajout d’une écriture canonique à un grand livre SQLite isolé. | Déplacement de `refs/heads/pilot` de l’ancien commit vers le candidat dans un dépôt bare isolé. |
| Entrée exacte | Vecteur `payment-control` existant : `invoice-17`, `v7`, `amount_minor:"420000"`, `EUR`, `account-17`. | `pilot-repository`, ref directe, OID ancien/candidat créés comme vrais objets Git dans chaque fixture. |
| Intention v1 | Projet + objet de dette `invoice_id`, hors clé de tentative/version/montant. | Projet + dépôt + ref directe + ancien OID, hors candidat et clé de tentative. |
| Source de vérité requise | Registre protégé résolvant fournisseur/facture/compte, versions et objet de dette unique au projet. **Absent de la fixture.** | Dépôt physique attribué, candidat/revue et droits de promotion liés au projet. **Absents de la fixture.** |
| Point d’engagement de fixture | Transaction SQLite qui vérifie epoch/grant et insère l’effet unique. | Transaction Git qui CAS la ref et crée un marqueur d’intention ; lock SQLite partagé avec la révocation de fixture. |
| Oracle brut | Lecture SQL en `mode=ro`, `query_only`, de la table `effects`, avec contenu canonique exact. | Lecture des refs réelles et du blob canonique pointé par chaque marqueur, sans journal SQL ni reçu. |
| Risques hors preuve | Aliases métier, usurpation, manipulation des fichiers, budget cumulé, doubles accès, coupure électrique. | Accès direct au dépôt, suppression de marqueurs, rollback, usurpation, vrai candidat/revue, coupure électrique. |

Les montants, identités, epoch initial 1, commits déterministes, code de crash 86
et domaines sont des **données/protocoles de test**, jamais des politiques
opérateur. SQLite `timeout=0` fait remonter immédiatement une contention de
fixture ; aucun watchdog, retry, plafond produit ou durée d’autorisation ajouté.
Les futures politiques réelles restent persistées, validées et modifiables via
Admin/API/CLI selon le contrat du projet.

## Carte d’accès et engagement

```mermaid
flowchart LR
  T[Recette isolée] --> C[CLI native figée par SHA256 : données hors ligne]
  T --> A[Grant et epoch simulés : SQLite]
  C --> S[Processus de service de fixture]
  A --> S
  S --> P[Écriture SQLite OU transaction de refs Git]
  O[Processus oracle séparé] -->|lecture seule| P
  T --> O
```

**Carte du laboratoire, pas TCB protégé.** Tous les chemins restent accessibles
au même utilisateur. La séparation des modules/processus ne confine personne.
Aucun secret, réseau, agent natif ou base Root n’est utilisé. Git ignore les
variables `GIT_*` héritées et la configuration globale/système ; hooks désactivés.
Le produit devra fermer les accès FS/FD/socket/réseau et lier les comptes,
binaires, manifests, projets et droits effectifs sous IA3/PA-02 et Rust/AC-02.

Le marqueur Git `refs/paea-lab/intents/<intent_id>` pointe vers un **blob du
contenu canonique complet**, pas seulement vers le commit candidat. Branche et
marqueur sont créés/modifiés dans une même transaction `update-ref --stdin`.
Ainsi une sortie après cet effet mais avant le journal SQL laisse une observation
exacte exploitable. Un nouvel essai avec autre contexte/candidat ne peut être
attribué au premier. Cette propriété reste limitée aux participants coopérants
de la fixture : aucune résistance à un agent écrivant directement ces fichiers.

## Les trois recettes initiales

Avant **chaque** injection, un témoin frais établit un effet valide avec les mêmes
préconditions ; les snapshots et sorties brutes sont conservés. Les opérations
de service/observation passent par de nouveaux processus, sans sleep ni retry.

| Recette | Injection et assertions observables | Reprise |
|---|---|---|
| T04 | Accord de fixture sur A ; changement du montant/candidat après cet accord. Refus avant effet ; snapshot du sink inchangé. | JSON réordonné/indenté mais équivalent accepté ; contenu exact présent à l’oracle. |
| T06/T21 | Sortie effective du processus **86 après publication** avant toute réponse. Pour Git, journal SQL encore vide. Client UNKNOWN ; oracle constate une seule écriture ou branche+marqueur exact. | Rapprochement connu ; nouvelle clé d’essai inchangée ne duplique rien. Autre contexte avec même intention/candidat refusé, sans fausse attribution. |
| T05/T17 | Grant préémis inutilisé ; révocation persistée puis première consommation : refus, aucun effet. Ordre inverse également joué. | Nouveau grant à l’epoch courant permet le témoin de reprise. Effet engagé avant révocation conservé, rapprochement encore connu. |

Ce sont des ordres déterministes sous lock, **pas une preuve de concurrence
distribuée/révocation immédiate**. L’ordre de fixture provient de séquences SQL,
jamais de deux horloges clientes. T05/T17 de production doit encore exercer la
course réelle au point d’engagement et les conditions de propagation.

Cas complémentaires : oracle indisponible → erreur et UNKNOWN, sans création
de base vide ; restauration → même effet ; ref Git obsolète/symbolique → refus
sans mutation puis reprise ; `/dev/full` après publication → échec de sortie,
effet toujours observé et rapproché. Un code d’erreur ne prouve pas l’absence
d’effet. Le service ne réémet pas automatiquement après une issue inconnue.

**Contre-exemple conservé PAEA-GUIDE-03.** Deux identifiants bruts représentant
hypothétiquement la même dette produisent deux intentions puis deux écritures
dans ce laboratoire sans registre métier. La recette attend ce résultat pour
réfuter la prétention d’unicité globale par hash seul ; ce n’est pas une réussite
de sécurité. Résolution : objet de dette unique fourni par un registre protégé,
avec tests fournisseur/référence/alias/correction avant PA-03 ; sinon schéma et
migration explicites. Le contrat v1 n’est pas changé implicitement.

## Exécution, preuves et accès UX

Recette maintenue : `tools/verification/recipes/paea-pilot-lab.json`.
Elle exécute huit tests nommés, dont les trois familles sur les deux domaines.
Variables requises : binaire existant exact, son SHA256 et **nouveau** dossier
`SWARM_PAEA_PILOT_EVIDENCE`. Aucun build caché.

```sh
export SWARM_PAEA_TEST_BINARY=/chemin/absolu/du/binaire-deja-qualifie
export SWARM_PAEA_TEST_BINARY_SHA256=empreinte-exacte
export SWARM_PAEA_PILOT_EVIDENCE=/chemin/neuf/observations
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/paea-pilot-lab.json --output /chemin/neuf/recette
python3 tools/verification/recipe.py verify --evidence /chemin/neuf/recette
```

Les parents des dossiers de preuve doivent exister. Les chemins et empreintes
réellement exercés sont dans le suivi d’incrément, avec argv, octets CLI, sorties,
états avant/après et racines des sinks. Une intégrité VERIFIED conserve la preuve ;
elle ne transforme pas le contre-exemple attendu en conformité.

Le menu **Vérifications PAEA → Campagnes de laboratoire** expose désormais la
commande explicite et un lecteur local de rapport. Il ne lance pas les campagnes
depuis le navigateur et ne donne aucun accord humain. Les preuves natives et
leurs limites sont dans [l’incrément23](INCREMENT-23.md).

Trois dossiers distincts : **conception** = ce document ; **réalisation** = sources,
binaire figé, profil retenu et état exact du laboratoire ; **qualification** =
recette/observations/contre-exemple/revue locale, avec critères protégés ouverts.
Les résultats réels et les limites de reprise sont dans [l’incrément 14](INCREMENT-14.md).

## Suite 16 — résolution explicite avant effet de fixture

La recette compte désormais huit tests. Le huitième appelle la CLI publique
`resolve-payment`, puis le sink et son oracle séparé : deux aliases convergent
vers un seul effet ; une correction conserve l’intention et refuse sa réattribution
à l’ancien contenu. Le contre-exemple sans registre reste conservé. Snapshot
coopérant, même UID : aucune autorité protégée ni continuité inter-révisions.

## Suite 17 — accès opérateur au résolveur

**Vérifications PAEA → Résoudre une référence de facture** donne accès au contrôle
de fichiers registre/demande et au rapport brut. Les exemples versionnés sont
`tests/fixtures/paea/payment-registry-v1.json` et `payment-request-v1.json`.
Ce menu n’exécute pas les campagnes de laboratoire, ne stocke pas de registre
et ne donne pas d’accord humain. [Parcours et preuves](INCREMENT-17.md).


### Campagnes de laboratoire — incrément23

Le menu Vérifications PAEA → Campagnes de laboratoire expose les scénarios et
la commande de reproduction. Depuis le dépôt :

```sh
python3 tools/paea/run_pilot.py --binary /CHEMIN/BINAIRE --sha256 SHA256 --output /DOSSIER/NEUF
```

Python/Git et un binaire existant exact sont requis ; parent existant, dossier neuf
hors stockage vivant et sources. Aucun build implicite, provider, retry ou watchdog.
La commande réutilise la recette maintenue, vérifie son intégrité et exporte
`paea-lab-report.json` seulement après huit tests PASS. En échec, preuves conservées,
pas de rapport de réussite ; corriger la cause et reprendre dans un dossier neuf.

La page lit ce fichier explicitement choisi, sans upload ni requête métier, et
permet son export exact. Elle vérifie sa structure fermée et les huit noms de test,
pas son authenticité ni ses originaux. Les empreintes relient binaire, résultat de
recette, inventaire déclaré et index des appels/oracles ; elles ne constituent pas
une signature. Vérifier aussi `python3 tools/verification/recipe.py verify --evidence
/DOSSIER/NEUF/recipe` sur les originaux conservés. Le test des aliases bruts passe
en constatant deux effets : la prétention d’unicité est réfutée. Autorité simulée,
même UID, oracles de processus distinct mais auteur commun ;0/17invariants
protégés, E2/E3 et exécution de campagnes depuis le navigateur restent ouverts.
[Résultats et limites de la tranche](INCREMENT-23.md).


### Concurrence locale — incrément 24

Le contrat de rapport à huit cas reste compatible : le cas T05/T17 existant
ajoute quatre scénarios de contention (deux domaines × détenteur commit/révocation).
Helper maintenu `tests/helpers/paea_pilot_contention.py` : barrière IPC après la
lecture réelle sous transaction, processus pair indépendant, erreur de verrouillage
conservée, absence d'effet concurrent puis reprise explicite. Aucun retry ajouté.
Le curseur est entièrement consommé avant la pause ; la contre-épreuve ne peut
donc pas conserver involontairement un verrou de lecture après commit prématuré.
Chaque contention a son témoin sans pair utilisant le même worker/barrière.
Recette ciblée `paea-pilot-contention.json`, recette complète `paea-pilot-lab.json` ;
les huit IDs de rapport sont inchangés, leurs empreintes distinguent les runs.
`SWARM_PAEA_CONTENTION_FAULT=early-release` est uniquement une substitution de
vérification qui doit faire échouer le test ; ne pas l'utiliser pour une campagne
normale. Les verrous de fixture ne protègent pas contre un auteur du même UID.
[Résultats, revue et limites](INCREMENT-24.md).


### Originaux liés — incrément25

`run_pilot.py` produit aussi `paea-lab-evidence.json` et annonce son SHA256 à
conserver séparément. Les appels v2 lient chaque entrée/flux original ; une sortie
d'oracle obligatoire absente refuse l'export avant publication. Rapport v1 inchangé.

```sh
python3 tools/paea/verify_pilot.py --campaign /DOSSIER/CAMPAGNE --inventory-sha256 EMPREINTE_CONSERVEE
```

Vérifie rapport, recette et membership/hashes exacts des sous-arbres recipe et
observations ; liens, FIFO, chemins traversants et mutations refusés. Ne rejoue
aucun argv. Originaux intacts, copie temporaire privée de recette retirée. Un pin
ne prouve ni auteur ni vérité de l'oracle ; tout réécrire et fournir un nouveau
pin reste possible au même UID. Historiques sans inventaire non réécrits ; aucune
garantie protégée ajoutée. [Contrat, preuves et limites](INCREMENT-25.md).
