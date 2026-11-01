# Reprise BLP-04 — provenance du prompt, 31 octobre 2026

## Besoin, décision et portée

Reprise du plan BLP après l’inventaire BF-05 → BF-07 → BF-16.
Responsable : Codex, développement direct autorisé, sans délégation ni appel payant.
Observation : `PreparationSend` ne transporte pas de principal BLP authentifié ;
un label ou un identifiant ajouté par le client ne prouverait aucune attribution.
Deux voies comparées : accepter des labels dans le DTO (écartée, autorité inventée),
ou conserver d’abord les octets du producteur et leurs origines (retenue, additive,
sans activer une protection incomplète). Les attributions persistées et le principal
courant restent nécessaires avant l’admission de transport.

Réalisé : assembleur avec couverture des octets, registre privé v3 obligatoire,
validation à la lecture native avant dispatch/worker/rejeu, refus sans mutation
pour une incohérence. Couverture : contexte projet agrégé, instructions moteur,
contrat de plan et neuf champs JSON. Aucune étiquette de confidentialité n’est
attribuée ; les sous-fichiers d’une source agrégée ne sont pas individuellement
résolus. BLP-03 migration, BLP-04 attribution et héritage, BLP-05 destination et
médiation et BLP-06 déclassification restent ouverts. Les acquis Biba restent requis.

## Preuves et reprise vérifiées au 1er novembre 2026

- `verification-blp/20261031-prompt-provenance-01/` : neuf tests initialement
  passants, avant découverte du défaut de correspondance des noms d’origine.
- `verification-blp/20261031-prompt-provenance-02-counterexample/` : **FAIL conservé**,
  permutation de deux noms acceptée malgré des octets/empreintes inchangés.
  Correction : contrôler aussi la grammaire d’assemblage et les clés JSON réelles.
- `verification-blp/20261031-prompt-provenance-04/` : **11 tests PASS** après
  correction : assembleur (4), enveloppes/consommateur natif (5), reprise
  historique (2). L’intégrité de cette preuve a été revérifiée le 1er novembre.
- `verification-blp/20261031-provenance-public-01/` : CLI 29 assertions PASS ;
  parcours HTTP incomplet, arrêté par une garde de fixture exigeant encore `/tmp`.
  Ce premier échec reste conservé. Le helper Python et le sidecar Rust exigent
  désormais le dossier configuré canonique explicite, un enfant direct de fixture
  et son marqueur ; `/tmp` et les placements hors périmètre restent refusés.
- `verification-blp/20261031-provenance-public-02/` : **PASS**, garde du sidecar
  (1 test), construction séparée du sidecar, CLI (**29 assertions**) et HTTP
  (**34 assertions**). Le stdin effectivement reçu par le fournisseur inerte
  correspond à l’empreinte conservée. L’intégrité de la recette a été revérifiée.
- `verification-blp/20261031-flow-inventory-02/` : inventaire réancré et cinq tests
  du vérificateur PASS ; intégrité revérifiée. Cela ne prouve pas la médiation BLP.
- `verification-blp/20261031-provenance-installed-01/` : **installation 284 vérifiée**,
  image chargée `874feeb290283e464711021f072e92e8973e6ac75be07b2d7994733eaaba954e`,
  endpoint protégé, notes compilées et boîte Version FR/EN (**14 assertions**,
  lien de commit, indication des modifications, Escape/focus, aucune erreur de
  page ni POST). La sonde terminée après interruption avait bien produit PASS.

L’observation initiale `running:false` provenait du bac à sable incapable de
joindre l’écoute locale, et ne prouvait pas l’arrêt du serveur : le contrôle
hors bac à sable a retrouvé la version 283 avant son remplacement par 284.
Le premier build avait compilé mais échoué à sa sonde d’écoute ; la qualification
locale autorisée a ensuite réussi. Aucun échec initial n’est assimilé à un PASS.

Au 1er novembre, une autre livraison a installé **290**, image
`2f104936daae347cbe46d21410a49c1485e135da9a2cfba15011e6d4eab39e2b`,
commit `18bb6bb6b5ee987e12a1f51c5ccda70fcd9db508`, modifications non commitées.
Cette reprise conserve le service courant et ne réinstalle pas 284. Les résultats
ci-dessus qualifient leur candidat historique, **pas la version 290**.
Voir `verification-blp/20261101-provenance-resume-01/` pour l’observation actuelle
et la comparaison des sources. Cinq des six fichiers sélectionnés sont identiques
aux preuves ; `root_store.rs` a évolué. Le workspace et ses dépendances ont aussi
évolué : aucune conformité globale actuelle n’est déduite de cette comparaison.

## Revue des contrats liée aux octets examinés

Empreintes encore identiques aux preuves (auto-revue, sans acceptation indépendante) :

| Source | SHA256 |
|---|---|
| `preparation_prompt.rs` | `86d08b88a025ba481bd7665aaab92d17257086de41180eb3ff68ad4163cd88c4` |
| `preparation_call.rs` | `615fcf28f61903a3baeeb8363b0966cda7bac96a1e4ccbffb36a36f24050d2d1` |
| `rust_preparation_cli_calls.py` | `7d0ffdd8d1e983370cd7e5263af5f481d1ecfb9c21898ca7d9ac2b95d1dfd575` |
| `rust_project_memory_source.py` | `e5f3530c67dc22bf93a1ac28685f983e215f58aaad71097a6936346044fa5d82` |
| `memory_source_fixture.rs` | `5dfb1b3d211724eab108b3736a5648ad7fd21d305942f88983ba1064d09c9e7f` |

Politiques : schémas 1/3, noms d’origines et neuf champs sont des invariants de
représentation ; aucune durée, tentative, budget ou plafond opérateur nouveau.
Les limites existantes de contexte continuent à porter sur les octets réels du
prompt. Le texte d’instructions préexistant est conservé à l’identique.

Fonctions non triviales : `assemble`, `Manifest::verify`, `append`, `hash`,
`native_metadata_valid`, `decode_present_provenance`, producteur `preparation_send`
et consommateur `record`, helpers de test et `private_shape`. Contrats documentés
avec absence d’authentification et d’autorité ; la provenance ne confine pas un
écrivain du stockage. Les DTO publics n’exportent pas le registre.

Erreurs : échec de schéma/encodage à l’assemblage avant sauvegarde ; échec de
provenance à la lecture native avant passage au worker et avant mutation de son
état/budget. Les erreurs SQL/codec/finalisation existantes restent conservées.
Ni champ absent dans v3 ni null ne donnent une réussite vide ou un repli historique.
Un ancien format reste explicitement sans provenance, sans garantie BLP ajoutée.

Prochaine condition observable : classification persistée de toutes les entrées
et principal authentifié, puis témoin d’envoi autorisé et refus sans octet reçu,
révocation, changement de source et héritage de la réponse. Ce registre ne clôt
aucun de ces critères. Aucune acceptation indépendante n’est affirmée.

Apprentissage vérifié : des empreintes valides ne suffisent pas à lier un nom
sémantique aux octets ; conserver le contre-exemple de permutation et vérifier
la clé/cadrage effectifs. Pour les recettes héritées, transmettre le dossier de
fixture configuré jusqu’au consommateur natif, sans supprimer sa garde. Les
recettes permanentes et leurs sources transmettent ces contrats aux reprises.

Cet incrément de reprise ne modifie que le suivi FR/EN et les pointeurs du plan ;
aucun code, réglage, ressource embarquée ou comportement produit ne change.
Aucun nouveau build, redémarrage ou changement de notes de version n’est requis.
