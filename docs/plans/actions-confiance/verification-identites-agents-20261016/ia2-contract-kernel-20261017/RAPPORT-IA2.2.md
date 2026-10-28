# IA2.2 — noyau de contrat exécutable, 17 octobre 2026

Codex seul, développement direct, branche `codex/clear-product-method-names`,
HEAD `4e58f7cbf8200a58f69f1f4d03d657c3313fb642` avec changements locaux conservés.
Méthodes : `replan`, APEX, puis auto-revue `code-reviewer`. Aucun avis indépendant,
commit, déploiement ou appel de modèle réel. Racines et émetteurs OIDC/service isolés.

**Noyau interne écrit et vérifié ; admission applicative non livrée.** F01–F07
restent ouverts et la matrice IA1 conserve 4 FAIL, 4 PARTIAL et 10 NOT TESTED sur
18 scénarios. Les résultats de composant ne prouvent pas le raccord au moteur.

## Reprise et décision

Les sources centrales de la preuve IA2 précédente étaient inchangées à la
reprise. Pas de nouvel audit identique pour produire de nouveaux chiffres.
Deux options comparées : acteur/autorité déclarés dans `Launch` et réemploi
implicite des rôles métier, ou contrat privé alimenté par Trust et une source
d'autorité distincte. La seconde est retenue. Sans bootstrap et canal protégés,
activer un contrôle de lancement aurait laissé une échappatoire ou bloqué les
parcours sans chemin permis qualifié. Le raccord demeure dans IA2.3/IA3.

Fichiers propres : `agent_identity_contract.go`, `agent_identity_authority.go`
et leurs deux tests dans `internal/engine/`,
[contrat FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md),
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md), plan, contrat IA2 et cette zone de
preuves. Sources mémoire, lancement, stockage et Trust préexistantes inchangées
par cette tranche. Aucun nouveau champ public, endpoint ou paramètre Admin.

## Contrat réalisé et limites

Types privés, JSON strict, principal complet issu de Trust, contexte du projet,
portée liée au plan, octrois et permissions exactes, délégation atténuée,
validité temporelle, révocation des droits dérivés, détection des cycles,
rejeu réauthentifié et transitions conservant l'histoire. Le consommateur interne
résout l'identité et évalue sous le même verrou Trust ; un logout OIDC invalide
le rejeu même sans changement de révision de configuration.

Le cadre d'autorité n'est pas peuplable par JSON et exige une source protégée,
**non implémentée**. Les tests en fournissent une fixture. Les références
d'instance sont simulées : leur comparaison n'est pas une attestation OS.
La racine canonique du Store lie le contexte local, sans être un identifiant
durable entre hôtes. Le digest de contenu devra venir de la vraie proposition
préparée. Une décision n'est ni réservation, ni accord humain, ni effet observé.

## Défauts introduits, reproduits et corrigés

| ID | Défaut et cause | Correction et preuve |
| --- | --- | --- |
| IA2-K01 | Un contexte d'appelant déjà résolu restait réutilisable à un instant ultérieur. | Instant vérifié et résolution fraîche sous verrou avant décision/rejeu. [Échec avant](cached-caller-before.jsonl), [commande et empreintes](cached-caller-before-run.json). Le test correspondant et le parcours OIDC décision → logout → rejeu passent ensuite. |
| IA2-K02 | Deux Stores isolés aux mêmes noms/jetons acceptaient le cadre et reçu de l'autre. La portée ne liait pas le projet ouvert. | Racine canonique liée à l'appelant/cadre et digest du contexte dans le reçu. [Échec avant](foreign-project-before.jsonl), [commande et empreintes](foreign-project-before-run.json). Refus entre projets vérifié après correction, même si le second dispose séparément d'un accord de fixture. |

Les sources avant correction ont été reconstituées depuis les éditions puis
vérifiées **octet pour octet contre les empreintes enregistrées avant les tests
rouges** : [index](source-reconstruction.json). Elles sont en `.go.txt` dans
`source-before-cached-caller/` et `source-before-foreign-project/`, pour ne pas
créer de faux packages Go. Cette reconstruction n'est pas une nouvelle exécution.

Des patches documentaires ont été refusés pour contexte de ligne ou préfixe
manquant ; relecture du texte/format puis correction, sans reprise inchangée.
Une recherche visant un glob encore absent a été remplacée par l'inventaire
réel. Ces erreurs de méthode ne sont pas des tests de protection.

## Vérifications et candidat

| Contrôle | Résultat | Originaux |
| --- | --- | --- |
| Nouveau noyau | 16 tests racine + 84 sous-tests : **100 PASS / 100 noms exécutés** | [JSONL final](tests-final.jsonl), [résumé](tests-final-summary.json) |
| Contrôles existants liés | 5 tests racine + 9 sous-tests : **14 PASS / 14 noms exécutés** | Même JSONL : lancement atomique/exclusif, refus de portée/prérequis, contexte/suspension Trust et logout OIDC |
| Total | **114 PASS / 114 noms**, 21 tests racine, aucun FAIL | [Commande, dates, exécuteur](tests-final-run.json) |
| Candidat | **1 042 fichiers locaux**, aucun changement pendant les contrôles | [Inventaire](inputs-final.json), `source-final/*.go.txt` |
| Statique / compilation | `go vet ./internal/engine`, `go build ./cmd/swarm` : sortie zéro | [Sorties finales](static-build-final.json), [binaire non déployé](binary-final.json) |

Les 75 puis 113 PASS précédents sont des étapes historiques sur d'autres versions,
pas la qualification du candidat final. Aucun test global ni fournisseur réel
n'est annoncé. Les contrôles existants sont sélectionnés pour le raccord Trust
et la conservation des contrats de lancement. L'inventaire final inclut les
ressources embarquées réelles ; les nouvelles pages d'architecture ne font pas
partie des motifs `go:embed`. La relecture reste locale, non indépendante.

## Couverture globale des critères IA2

| Critère | État et limite |
| --- | --- |
| C01 | PARTIAL : refus et expiration dans le consommateur interne ; aucun garde avant lancement fournisseur. |
| C02 | PARTIAL : principal, projet, détenteur et référence d'instance ; pas d'ancrage OS ni résistance au jeton volé. |
| C03 | NOT TESTED : aucune création ou consommation atomique réelle. |
| C04 | PARTIAL : atténuation et filiation ; pas de parcours de demande d'enfant/humain. |
| C05 | PARTIAL : pas de promotion par rôle/champ ; accord humain exact et renforcement non réalisés. |
| C06 | PARTIAL : droits dérivés retirés dans le graphe ; aucun arrêt, course au départ ou effet réconcilié. |
| C07 | PARTIAL : rejeu frais et exact ; pas de consommation unique concurrente ni reprise de départ. |
| C08 | NOT TESTED ici : frontière fournisseur non intégrée ; Bubblewrap antérieur reste séparé. |
| C09 | PARTIAL : import sans autorité, ancien état incompatible avec cadre courant et histoire conservée ; ancre réelle et récupération après compromission absentes. |
| C10 | NOT TESTED : opérations mémoire nommées mais consommateurs non raccordés. |
| C11 | NOT TESTED : bootstrap, administration et source d'autorité protégée absents. |

Ces résultats locaux ne requalifient pas IDA-01–18.

## Apprentissage et suite

Résoudre l'identité avant tout résultat de rejeu et lier les preuves d'autorité
au contexte effectivement ouvert sont deux règles vérifiées ici. Contrats FR/EN
et plan transmettent ces leçons à la suite ; cela ne modifie pas automatiquement
les contextes natifs et ne prouve pas une boucle Swarm autoréparatrice.

Propriétaire : Codex. IA2.3 doit fournir cadre protégé, ancrage d'instance et
protocole Trust JSON / SQLite : ordre des verrous, contrôle courant, réservation,
départ/révocation et réconciliation. Le fournisseur devra travailler sans accès
au registre/clé ni sortie contournant le contrôle ; le réseau coupé du prototype
ne suffit pas pour un fournisseur distant. Aucun raccourci par rôle `admin`,
bearer partagé, désactivation ou empreinte calculée depuis l'entrée à vérifier.

IA3 raccordera toutes les activations et la décision humaine. IA4 dépend du
chantier mémoire ; IA5 conserve qualification complète et RETEX. Cette tranche
ne clôt ni IA2 entier ni le chantier identité.
