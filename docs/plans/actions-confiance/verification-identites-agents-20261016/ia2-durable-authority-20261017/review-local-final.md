# Auto-revue finale IA2.3b — composant interne

17 octobre 2026, Codex dans le contexte producteur. **Revue locale non indépendante.**
La base de comparaison est celle de `baseline.json` et de `preimages/`, pas le
HEAD Git : le checkout contenait déjà d'autres changements.

Décision : **composant interne vérifié dans le périmètre des essais**. Cette
décision ne vaut ni acceptation de l'identité globale ni activation en production.

Les quatre changements de sources préexistantes ont été relus par rapport aux
préimages. Les six nouveaux fichiers ont été examinés, en particulier le permis
exact, la lecture versionnée, les refus sans mutation et la reprise après COMMIT.
Le refus pour travail absent précède maintenant l'intention durable ; le lecteur
vérifie la version dans sa transaction. Les contre-exemples originaux sont
conservés, et leurs gardes passent dans le candidat final.

Les 181 noms de tests passent, dont 33 pour le registre ; les 52 noms sous
`-race` sont un sous-ensemble rejoué. Les 1 052 entrées inventoriées sont restées
inchangées pendant ces vérifications. La sélection élargie couvre les dépendances
du contrat commun d'écriture et de la migration, sans revendiquer une suite globale.

Ordre examiné : publication Trust exclusif puis écrivain SQLite ; lecture Trust
partagé puis transaction SQLite libérée avant le verrou runtime. Le contexte
annulé pendant l'attente est testé. Le scan AST affiné n'est pas une preuve de
tous les callbacks : le futur appelant ne doit pas entrer dans la publication
avec une transaction SQLite déjà tenue. Un temps d'évaluation fourni pour un
test ne constitue pas une horloge extérieure authentifiée.

Les six diagrammes FR/EN ont été inspectés visuellement. Le texte rogné de la
note du nouveau schéma français a déclenché une correction de mise en page,
un nouveau rendu FR/EN puis une nouvelle inspection ; `diagrams-layout-fix.json`
conserve ces quatre commandes. Aucune source Go n'a changé pour cette correction.

Le vérificateur d'index a été corrigé après un comptage manuel faux (16 archives
attendues, 17 présentes). Il exige maintenant l'ensemble exact des chemins et
la concordance des octets. L'original rouge est conservé ; aucune garde ni
empreinte n'a été supprimée pour obtenir le passage.

Réserves maintenues : permis/checkpoint émis par fixtures, absence de témoin
externe courant et protégé, producteur du vrai plan adopté absent, accord humain
et consommation du lancement non raccordés, aucun lanceur public ni effet métier.
Les types Go privés ne protègent pas d'un contrôleur hôte compromis. Les essais
ne certifient pas une image SQLite hostile arbitraire, une panne électrique ou
tous les systèmes de fichiers. Les primitives prenant un cadre fourni restent
soumises à la fraîcheur de leur appelant.

F01–F07, IA3, IA4, IA5 et RETEX global restent ouverts. Ces limites sont
transmises dans le plan, le contrat FR/EN et le dossier scientifique ; leur
documentation ne prouve pas leur résolution. Aucun critère public IDA n'est
requalifié par cette auto-revue.
