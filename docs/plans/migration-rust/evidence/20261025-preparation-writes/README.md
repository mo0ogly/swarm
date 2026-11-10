# Préparations Rust — preuves du 25 octobre 2026

Parcours CLI/API create/save et Admin, uniquement Roots isolées.
Binaire final : `bde7da26821e20958766a9aeba122350757e1cba68c9668f66ebe31d14166072`.
Profil par défaut : 432 entrées archivées, 111 artefacts observés, quatre crates
principales sans feature de fixture ni profil test. Ce binaire sans métadonnées
de livraison injectées n’est pas une version installée.

- [Index](index.json) : portée et empreintes.
- [Écritures finales](writes-recipe8-result.json) : 47 assertions, 8 clés,
  2 tests SDK (deux défauts SQL et corruption du reçu courant).
- [Reprise v41](upgrade-recipe8-result.json) : 6 assertions.
- Chaque parcours Admin : 45 contrôles navigateur FR/EN, deux thèmes et largeurs.
- [Revue des quatre axes](scoped-source-review.json) et
  [instruction de la revue indépendante GLM](independent-review-assessment.json).

Les originaux sont conservés sous `private/`, ignoré par Git et à permissions
restreintes : sources exactes, commandes, sorties, recettes, Roots factices,
captures et réponses GLM, dont les échecs. L’inventaire privé permet le contrôle
local de leurs empreintes. Ces fixtures contiennent des jetons de session :
ne pas publier les données brutes. Leur archivage local n’est pas une sauvegarde
Git partagée. Les recettes maintenues sont versionnées dans `tests/` et
`tools/verification/recipes/`, hors cache.

Deux builds communs invalidés et la revue GLM504 restent des échecs conservés.
La lecture58 conserve son candidat précédent distinct. Dev77, activé ailleurs,
a été observé et son binaire testé sur fixtures ; il précède le correctif final D1.
Aucune mutation de données live, installation depuis ce chat, acceptation globale
ou qualification Biba/SCALING n’est déduite de ces résultats.

## Livraison effectuée après la phase isolée

[Dev78 installé et vérifié](delivery-result.json) :49assertions CLI/API,
8clés,2tests SDK,6de reprise ;14contrôles du dialogue installé et11HTTP/identité.
Le profil source et les captures réels sont conservés sous `private/delivery-5xptu6ez`.
Le binaire isolé et sa revue ci-dessus restent des preuves historiques distinctes.
La revue liée aux octets de cette livraison est dans `delivery-source-review.json`.
