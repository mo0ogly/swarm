# Audit de l’échec et de la durée de la suite Go — 10 octobre 2026

Audit demandé après la [requalification des phases du plan](20261010-phases-plan-integration.md).
La mission Cursor continue avec T2 en copie Git isolée. Cet audit utilise des
fixtures temporaires et un fournisseur déterministe ; aucune base de mission
active, preuve historique, politique ou limite d’agent n’est modifiée.

## Rectification du diagnostic

La commande `go test ./... -timeout 5m` a arrêté **le paquet entier**
`internal/engine` après 300,075 s. Au moment de l’arrêt,
`TestManagedFragmentHistoricalPublicRecovery` ne tournait que depuis 33 s, et
son sous-cas `exit` depuis 6 s. Dire qu’un test était bloqué pendant cinq minutes
était donc incorrect. Une trace prise à l’expiration identifie le travail en
cours ; elle ne prouve pas sa cause ni un blocage infini.

## 1. Qualification du candidat périmée : défaut établi

`graphDeliveryD03Load` sélectionne `docs/ui-redesign-source-manifest.json`
par une succession de chemins codés dans le test. Ce dernier instantané contient
1 081 entrées, avec le statut explicite
`UI_REDESIGN_SOURCE_SNAPSHOT_NOT_INDEPENDENT_ACCEPTANCE` et la base
`d4539e0041000a38acb162a17f053e318fb3db5d`.

La comparaison avec le code courant trouve 30 empreintes différentes. Sur un
clone isolé du commit sauvegardé `1169df70bec73d50fd23f640245b8ffd1317b9de`,
23 entrées divergent déjà, sans les modifications récentes du moteur. Le test
échoue aussi dans ce clone, en 0,01 s. Les sept divergences supplémentaires
concernent les fichiers Go corrigés ensuite ; cela invalide leur ancienne
qualification, sans prouver une régression fonctionnelle.

Première divergence, `PREPARATION-UX.md` :

- Empreinte attendue : `f0ce90e1a0ae2559bcef55aedea21225c19ffe07f475c9d72ee1dafcabfc0092`.
- Empreinte actuelle et du HEAD committé : `5d6e1016aff21ef22ff9b5d4f27d10c75a21e6f617801ca02cac3dcb1947d06f`.
- Ce fichier n’est pas modifié dans la copie de travail.

L’empreinte attendue correspond bien au fichier du commit `d4539e0` : le test
attend l’ancienne version et la copie actuelle contient une version ultérieure.

La garde d’intégrité refuse correctement le candidat actuel. Le défaut de
maintenance est de conserver comme vérification du code courant un instantané
qui ne correspond déjà plus au commit sauvegardé. Rafraîchir ses empreintes
automatiquement ferait disparaître le signal sans produire une nouvelle recette.

**Correction à préparer :** conserver les anciennes qualifications avec leur
candidat reproductible ; séparer leurs contrôles d’intégrité des régressions
fonctionnelles courantes. Une nouvelle qualification doit enregistrer son propre
candidat, ses contrôles et ses limites. Remplacer la succession de manifestes
par une sélection explicite et traçable, avec les cas négatifs de candidat absent,
preuve altérée et nouvelle entrée non qualifiée. Cet audit ne renouvelle aucune
acceptation historique et n’applique pas cette correction.

## 2. Délai de la suite : blocage infini non démontré

Exécution isolée, sans enlever de contrôle :

```sh
go test ./internal/engine \
  -run '^(TestGraphDeliveryD03QualificationManifest|TestManagedFragmentHistoricalPublicRecovery|TestManagedFragmentHistoricalCapacityPreservesKnownInputs)$' \
  -json -count=1 -timeout 3m
```

| Scénario | Verdict | Durée |
| --- | --- | --- |
| Manifeste de qualification | FAIL | 0,01 s |
| Reprise historique, `pass` | PASS | 22,77 s |
| Reprise historique, `fail` | PASS | 19,61 s |
| Reprise historique, `exit` | PASS | 21,03 s |
| Reprise historique, ensemble des trois cas | PASS | 63,41 s |
| Capacité conservant les entrées connues | PASS | 0,88 s |

Le code de sortie 1 de cette commande provient du manifeste. Il ne transforme pas
les deux scénarios réussis en échecs. Les sous-cas et leur parent se recouvrent ;
leurs durées ne doivent pas être additionnées.

L’inventaire découvre 1 099 tests principaux. La CI lance déjà
`tests/supervision_go_suite.py`, qui compile une fois et vérifie une couverture
disjointe et complète : 19 groupes, dont six isolés, et parallélisme quatre.
Son délai configuré est de 240 s **par groupe**, pas pour toute la suite.
La répartition est calculée depuis l’inventaire réel. Les estimations de durée
ne sont pas des résultats de test ; 1 002 noms n’ont pas de mesure dans le planning
actuel et prennent le coût de répartition par défaut.

Le passage complet d’audit est `go test ./... -json -count=1 -timeout 15m`.
Ce délai sert à observer la suite au-delà de la coupure précédente ; il ne change
aucun réglage d’exécution du moteur. Le paquet termine en **504,624 s**, soit
environ **8 min 25 s**, sans expiration. Les 1 099 tests principaux ont tous un
état terminal : **1 090 PASS, 8 SKIP, 1 FAIL**. Le seul échec est
`TestGraphDeliveryD03QualificationManifest`. Le code de sortie reste 1 ; la
suite n’est pas verte.

Les huit tests ignorés sont `TestOperatorPreview`,
`TestCorrectiveRecoveryBrowserRecipe`, `TestEngineContractTruthBrowser`,
`TestHistoricalRequalificationBrowser`, `TestManagedRecoveryBrowserRecipe`,
`TestMissingReportRecoveryBrowserRecipe`, `TestRecoveryHealthBrowserRecipe`
et `TestLiveProviderStreamReplay`. Ces recettes ne sont pas comptées comme
réussies et aucune recette navigateur ou qualité fournisseur réel n’est déduite
du passage Go.

Dans ce passage complet, la reprise historique termine en 46,31 s ; le scénario
de publication fragmentée en 38,59 s et le protocole fournisseur fragmenté en
20,25 s. La coupure précédente de 300 s était donc insuffisante pour cette
exécution séquentielle. Aucun blocage infini n’est reproduit dans cet audit ;
cela ne garantit pas l’absence de blocage dans d’autres conditions.

## 3. Coût des vérifications de preuve : problème mesuré

Profil du seul sous-cas historique `pass`, avec sortie du binaire dans `/tmp` :

```sh
go test ./internal/engine -run '^TestManagedFragmentHistoricalPublicRecovery/pass$' \
  -count=1 -timeout 3m \
  -cpuprofile /tmp/swarm-go-audit-fragment.cpu \
  -memprofile /tmp/swarm-go-audit-fragment.mem \
  -o /tmp/swarm-go-audit-fragment.test
```

Le test passe en 15,739 s. Le profil relève 15,89 s de CPU échantillonné et environ
4,95 Gio d’allocations **cumulées**, ce qui ne mesure ni le pic de mémoire ni la
mémoire occupée à un instant. Les différences de durée avec la commande précédente
reflètent des exécutions distinctes, sous charge ; une seule mesure ne suffit pas
à établir une performance représentative.

`managedReviewFilesIntactSeen` représente 42,54 % du CPU cumulé du profil et
55,15 % des allocations cumulées. `readFragmentJournalAnchor` est inclus dans
ce chemin : 33,10 % du CPU et 44,48 % des allocations. Ces pourcentages se
recouvrent. Le profil montre surtout sérialisation/décodage JSON, empreintes,
copies et collecte mémoire, pas un modèle réel attendant une réponse.

Le code revalide les preuves au rafraîchissement, à la surveillance de révocation
et à la publication. Le fournisseur appelle sa garde lors d’un contrôle nominal
toutes les 200 ms ; la revue fragmentée y relit et revalide les preuves. En outre,
`saveManagedReview` les vérifie dans la mutation, après réservation du rédacteur
SQLite. Ce placement peut augmenter la contention ; cet audit n’a pas mesuré
sa latence ni démontré une expiration de production causée par ce mécanisme.

**Correction à étudier :** éviter de reconstruire et reparcourir les mêmes
structures validées à chaque garde. Une réutilisation doit être bornée à
l’activation, liée aux identités et empreintes exactes et précédée des contrôles
de fraîcheur nécessaires. Conserver les refus sur altération, changement de
candidat/contrat et révocation. Mesurer allocations, CPU et latence de mutation
sur des tailles croissantes avant de choisir une optimisation. Le profil ne
justifie ni suppression de garde ni simple relèvement des délais.

## Provenance et limites

Hôte Linux amd64, Go `1.24.3`, douze processeurs logiques accessibles. HEAD
`1169df70bec73d50fd23f640245b8ffd1317b9de`, copie modifiée. L’empreinte agrégée
des 521 fichiers `internal/engine/*.go`, tests inclus, est
`d57301e0f960fa1ce7de56f1b3e229b5c19a974e79b1f34e64b9d93e469d0182`.
Elle ne couvre pas toutes les autres entrées documentaires de ces tests.

Traces d’audit dans `/tmp/swarm-go-audit-*` : événements JSON complets et isolés,
reproduction sur HEAD, comparaison des empreintes, inventaire, profils CPU et
allocations. Les fixtures ne consomment pas d’appel IA réel. T2 utilise séparément
le fournisseur réel ; son exécution n’est pas une preuve issue de cet audit.

Les [observations structurées](20261010-audit-suite-go.json) conservent les
commandes, identités, états terminaux, durées et empreintes des traces. La suite
répartie de CI n’a pas été réexécutée : sa couverture a été vérifiée statiquement,
et le passage séquentiel complet a fourni les verdicts ci-dessus.
