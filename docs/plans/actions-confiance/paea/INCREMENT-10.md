# PAEA — incrément 10 : rapport autonome lié aux entrées exactes

[English](../../../en/plans/actions-confiance/paea/INCREMENT-10.md) · [Plan](../PAEA-APEX.md)

## Besoin et décision

Reprise « next », APEX/replan, Codex seul. L’interdiction de compilation et de
redémarrage reste active. Aucun agent, fournisseur, Root réel ou service lancé.
PAEA-OBS-01 est repris dans PA-01/09 : deux reçus différant seulement par leur
digest d’observation produisent le même rapport v1, comme le prouve l’original
de l’incrément 09. Cette limite était déclarée, sans fausse authenticité ; le
transcript complet fournissait déjà la mitigation dans les dossiers de recette.

Trois options relues : modifier v1 sur place romprait son schéma strict ; garder
seulement le dossier externe ne lie pas le rapport autonome ; ajouter un rapport
distinct explicite permet de conserver les consommateurs existants. Cette
dernière option est retenue. `receipt-report` émet un schéma de sortie v2 ; les
entrées et `check-receipt` restent en v1. Aucun alias d’autorité ou de service.

`check_receipt_documents` et `ReceiptReport` sont ajoutés au module Rust existant
des reçus. Décodage strict, canonicalisation et même contrôle de liaison/cycle,
puis `DocumentIdentity` existant et SHA-256 des deux tranches d’octets réellement
consommées. Le CLI réutilise ses lecteurs et son writer ; aide, schéma, recette
et contrat FR/EN sont actualisés. Pas de nouvelle dépendance ni de code produit
Python. Les trois marqueurs d’autorité/authenticité/effet restent non établis.

**Sources écrites, non compilées, non exécutées, non installées.** Le témoin
public avant changement conserve contrôle v1 → nouvelle commande absente →
reprise v1 sur le binaire dev75 déjà figé, sans build. Cela établit le manque
avant modification, pas le succès du nouveau code. Les sept tests nouveaux
sont seulement sélectionnés : quatre bibliothèque, trois CLI Linux. La recette
complète et Rustdoc restent différés.

## Contrat, contrôles et erreurs

Le rapport contient version2, scope, identité du document, SHA du reçu, états
précédent/proposé et les trois marqueurs non autorisants. Les clés de transport,
la mise en forme et le digest d’observation ont leur identité brute ; une action
canonique ou intention équivalente peut donc garder son digest tout en changeant
`document_sha256`/`receipt_sha256`. Aucun hash n’est une signature, horloge ou
observation indépendante. L’origine et l’actualité des entrées restent à établir
par le propriétaire protégé. La lecture CLI reste séquentielle, sans instantané.

| Axe | Revue et limite |
|---|---|
| Politiques | Littéral2/version et commande distincte : protocole de sortie, pas choix opérateur. SHA-256 et valeurs `not-established` : contrat existant. Les longueurs, tableaux de cas et digests des tests sont des fixtures. Aucun budget, retry, délai, plafond ou politique effective ajouté. |
| Fonctions | Nouveau type/fonction publics documentés ; `execute`/helper CLI et aide raccordés. Treize fonctions de test documentées ; préconditions, erreurs, IO, non-autorité et publication partielle relues. Rustdoc/typage non vérifiés sous la consigne. |
| Erreurs | Décodeurs stricts puis checker existants, avant construction du rapport ; codes/cause JSON conservés. Lectures/fermetures et erreurs secondaires par le lecteur commun ; sérialisation avant sortie, write/flush commun avec cause originale. Aucun refus converti en rapport vide ou état enregistré. |
| Documentation | Opération, versions d’entrée/sortie, schéma, compatibilité, limites et statut non exécuté en FR/EN. Notes locales qualifiées « en sources ». Aucune preuve de 09 réécrite ou réutilisée comme PASS du nouveau code. |

Les tests préparés couvrent deux domaines, formats bruts, changements d’observation,
alias transport, mauvaise forme/binding/cycle et reprise. Le parcours CLI prévoit
FR/EN, format v1 inchangé, refus runtime/stdin et panne `/dev/full` ; ce dernier
ne remplace pas une injection de flush Rust. Aucune API/UI ni service protégé
n’est raccordé par cette tranche.

## Preuves et reprise

[Preuves propres](evidence/20261025-receipt-report-b9gi7a2v/) : plan/choix, originaux,
témoin public avant changement, commandes, diff et revue liés aux octets.
[Revue finale](evidence/20261025-receipt-report-b9gi7a2v/source-review.json).
Le formatage/analyse syntaxique et les assertions de schéma statiques ne sont
ni compilation, ni exécution des sept tests, ni graphe de build, ni revue
indépendante. Le statut précis est dans le dossier de preuves.

Le contrôle final a refusé deux empreintes périmées après des écritures concurrentes.
Les sources liées au contrôle statique ont été reconstituées depuis originaux/diff
et vérifiées par leur SHA : seule la constante d’aide générique USAGE du CLI a
changé, ainsi qu’une note FR/EN d’un autre chantier. Ni parser, dispatch PAEA ni
contrôle de reçu modifié par cette dérive. Originaux/refus/diffs conservés, ajouts
étrangers préservés ; syntaxe et métadonnées réexaminées sur leurs nouveaux octets.
Le statut ne vaut aucune validation du chantier concurrent.

PAEA-OBS-01 : amélioration réalisée en sources, effet encore **NOT TESTED**.
Owner Codex : après levée de la consigne, compiler les targets affectés, vérifier
les sept tests et le maintien des anciennes sorties, Rustdoc et recette sur même
candidat, puis chemin de livraison supporté. La preuve originale de 09 reste
valide sur son binaire et ses sources archivées ; les sources nouvelles ne sont
pas celles de ce build. PA-02/03 protégés, E2/E3, installation et RETEX restent
ouverts ; aucune nouvelle autorité acquise par des empreintes.
