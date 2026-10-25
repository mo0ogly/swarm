# PAEA — incrément 14 : effets de laboratoire et oracle

[English](../../../en/plans/actions-confiance/paea/INCREMENT-14.md) ·
[Plan](../PAEA-APEX.md) · [Dossier opérationnel](PILOTE-OPERATIONNEL.md)

## Reprise et décision

« go next » reprend la priorité de l’incrément 13 : effet/autorité/oracle et trois
pannes sur les **deux** domaines. APEX/replan appliqués ; Codex racine seul,
checkout direct, sans compilation, redémarrage, installation, provider, base
Root, effet réel externe, délégation, commit ou push. Les six documents partagés
ont été comparés à leurs préimages avant édition ; pas de dérive concurrente.
Le binaire effectivement exercé reste la copie figée dev77 de l’incrément 12,
SHA256 `7e7563dd5e1bdb848fec42f6ae2e619c31b048889221191517553fe45e56f6d3`.
Les preuves de profil de 12 restent historiques ; aucun nouveau build déduit.

Deux voies comparées : écrire maintenant des services natifs non qualifiables,
avant leurs sorties d’autorité PA-02 ; ou préparer des effets exécutables isolés
et leurs observations. Seconde voie retenue. **Ce laboratoire ne réalise pas
PA-02/03 protégés.** Le dossier renseigne les conséquences, entrées, intentions,
sources requises, point d’engagement, oracles, risques et carte d’accès. Les
responsables métier/risque, approbateur humain et évaluateur restent non attribués,
sans inventer de classe C3 ni de mandat.

Diagnostic de méthode conservé : sortie `git status` trop volumineuse, puis
recherches bornées aux chemins possédés ; crate `swarm-trust` supposée absente,
véritable propriétaire trouvé sous `swarm-engine/src/trust`. Aucune édition ou
exécution dépendante sur le chemin supposé.

## Réalisation et observations

`tools/paea/pilot_lab.py` fournit deux sinks de fixture : écriture SQLite unique
par intention et transaction Git CAS branche + marqueur d’intention. Un grant
simulé et l’epoch courant sont vérifiés dans l’ordre du lock SQLite partagé.
Le modèle fournit donc des ordres déterministes, pas une preuve de course
distribuée ni de révocation immédiate. La CLI publique existante produit le
contenu canonique ; aucune canonicalisation Python de remplacement.

`pilot_oracle.py` lit les données d’effet réelles dans un processus séparé :
SQL en lecture seule, ou refs et blobs Git. Il ne lit ni reçu, ni approbation,
ni journal de décision. **Chemin d’observation séparé, pas évaluation indépendante** :
même auteur, même UID et fichiers toujours accessibles aux participants.

| Contrôle exécuté | Résultat et limite |
|---|---|
| T04, deux domaines | Témoin frais, substitution refusée sans mutation ; JSON équivalent réordonné accepté ensuite. |
| T06/T21, deux domaines | Vraie sortie de processus 86 après effet avant réponse ; client UNKNOWN, effet unique constaté puis rapproché. Pour Git, journal SQL encore vide après crash. Nouvelle clé d’essai sans duplication ; contexte modifié refusé. |
| T05/T17, deux domaines | Première consommation d’un grant préémis refusée après révocation ; séquence/epochs persistés contrôlés. Ordre inverse conserve l’effet ; nouvel accord de fixture permet la reprise. |
| Oracle indisponible, deux domaines | Échec/UNKNOWN, aucune base vide créée ; restauration retrouve l’effet initial. |
| Git obsolète/symbolique | Refus, snapshot identique, puis reprise valide. |
| Sortie `/dev/full`, deux domaines | Diagnostic UNKNOWN, effets conservés et rapprochés. CPython retourne 120 à la finalisation de son buffer après le refus d’écriture ; jamais un succès ni une absence d’effet. |
| PAEA-GUIDE-03 | Deux IDs bruts hypothétiquement aliases d’une dette donnent **deux écritures** sans registre. Contre-exemple conservé ; unicité métier globale non établie. |

Recette finale `paea-pilot-lab.json`, **7 tests nommés PASS**, durée **17,125 s**,
aucun watchdog. **132 processus : 31 CLI de canonicalisation, 39 services de
fixture, 62 oracles** ; 26 racines isolées conservées. Sorties : 116 succès,
10 refus attendus code2, deux crashs injectés86, deux erreurs de sortie120,
deux observations indisponibles1. Ces nombres décrivent le run final, pas le
cumul des relectures. Ces nombres ne sont ni essais E3 indépendants,
ni garanties d’atomicité sous coupure électrique. Recette **PASS**, intégrité
**VERIFIED**. Les tests natifs HTTP restent non exécutés ici.

## Contre-épreuve et améliorations

**PAEA-LAB-01, liaison exacte après crash.** Revue du premier dessin : un marqueur
ne contenant que le candidat Git perdrait le contexte canonique si le processus
quitte avant son journal SQL. Réalisation retenue : marqueur vers un blob de tout
le contenu canonique, lié atomiquement à la branche. Une copie isolée de quatre
fichiers modifie seulement le garde de rejeu pour comparer le candidat en ignorant
le contexte. Le témoin valide passe ; l’essai du contexte modifié retourne alors
0 au lieu du refus2, et le test échoue exactement sur **`AssertionError: 0 != 2`**
dans le sous-cas Git. `mutation.diff`, sources et résultats originaux conservés.
Les sources retenues n’ont pas été modifiées pendant ce test. La revue suivante
a ajouté la cause explicite `laboratory-intent-conflict` aux issues UNKNOWN de
rapprochement, avec assertion exacte sur les deux domaines. `recipe-01` conserve
le premier PASS (21,126 s) ; `recipe-02` vérifie les nouveaux octets (17,125 s).
La contre-épreuve reste applicable au garde de rejeu inchangé ; sa portée
n’inclut pas cette évolution de diagnostic.

**PAEA-GUIDE-03, registre métier nécessaire.** La recette produit désormais le
contre-exemple réel de laboratoire, distinct du simple risque documenté en 13.
Propriétaire : intégration métier PA-02/03. Reprise : registre protégé résolvant
une dette unique au projet, avec collisions fournisseur/référence, alias et
corrections ; si impossible, schéma versionné et migration explicite. Le service
de fixture accepte volontairement les IDs bruts pour conserver cette limite.
Il ne doit pas être promu comme service protégé avant résolution. Ni modification
implicite du hash v1, ni exception par facture/mission.

## Revue, rétention et transmission

Dossier de preuve : `evidence/20261025-pilot14-zswhjf5p/`. `plan.json`,
`summary-02.json`, `recipe-02/`, `observations-02/`, le premier run conservé,
`mutation-source/`,
`mutation-result.json`, revue liée aux hashes, contrôles statiques et diff possédé
conservent l’autorité, les octets, fonctions, politiques et erreurs examinées.
La recette retient les sources déclarées ; elle ne remplace pas un graphe de
compilation ou une revue indépendante.

Revue explicite des trois axes :

- Politiques : montants/identités/OIDs/epoch1/exit86 sont fixtures ou protocole de
  panne ; grammaire d’intention et CAS sont invariants de test. `timeout=0` est
  le refus de contention de SQLite de laboratoire. Aucun réglage d’exécution
  produit, retry, budget ou plafond caché introduit ; aucun parcours Admin réel
  revendiqué. Git neutralise configuration/hook/routage hérités dans la fixture.
- Documentation : **27 fonctions/méthodes** des trois fichiers Python ont un
  contrat relu ; dossier FR/EN, plan, file et guide transmettent usages/limites.
  Aucun binaire ou contexte embarqué déjà démarré n’est modifié par ces fichiers.
- Erreurs : refus d’accord/epoch/intent/CAS/symref avant nouvel effet ; erreurs IO,
  SQL, Git et écriture peuvent conserver une issue UNKNOWN. L’observateur ne
  transforme pas un refus/échec de lecture en résultat vide. Les échecs injectés
  et la contre-épreuve restent visibles, avec reprise sur précondition changée.

Une édition du plan a refusé un ancrage tronqué avant toute modification ; ligne
complète relue puis patch corrigé. `execution-inputs.json` conserve ce diagnostic
ainsi que Python/SQLite/Git, chemins/empreintes des exécutables et environnements.

Pas de nouvelle note de version produit : les modifications sont outils de
laboratoire et documentation, aucune entrée de build de l’interface/service
modifiée ni livraison effectuée. Aucun test navigateur relancé pour ces fichiers
hors interface ; les preuves de composant de 13 restent distinctes.

## Retour au besoin

Préparation opérationnelle vérifiée **dans les fixtures seulement**. **0/17
invariants démontrés de bout en bout**. Le menu PAEA conserve le contrôle de
correspondance ; les campagnes ci-dessus ne sont pas déclenchables dans l’UX.
Suite autorisée : réemploi des tests/contre-exemples pour spécifier le producteur
de dette et la liaison d’autorité PA-02, puis services PA-03 ; intégration UX des
campagnes après contrat public réel. Conserver les prérequis IA3/Rust/AC-02.
Build/qualification native/installation après levée explicite de la consigne ;
E2/E3, rôles habilités et RETEX final restent ouverts.
