# Choix proposé pour le premier incrément mémoire

État courant : le cadrage étendu est dans [CADRAGE-MEMOIRE.md](CADRAGE-MEMOIRE.md).
La proposition UI ci-dessous est conservée comme étape historique et composante
possible. Le parcours recommandé porte désormais sur apprentissage sourcé,
rappel entre missions et correction ; aucune réalisation n’est déclarée adoptée.

## Recommandation issue de l’audit

Enrichir la sélection et la traçabilité de la mémoire native Swarm en réutilisant
le guide de terrain et les profils projet par rôle. Cette proposition traite
les écarts MEM-11 et MEM-12 de `AUDIT-MEMOIRE.md`. Elle n’est pas encore une
fonctionnalité réalisée ni une décision produit adoptée.

Le code et les sondes montrent que les leçons peuvent déjà être publiées avec
preuve puis transmises dans un profil. Deep Agents confirme l’intérêt d’une
mémoire documentaire injectée au démarrage, mais son adaptation demande aussi
propriété des conversations, validation des sources, fraîcheur et concurrence.

## Options comparées

| Option | Apport établi ou attendu | Conséquences | Proposition |
| --- | --- | --- | --- |
| A Utiliser les profils actuels par CLI | Le corps d’une leçon peut déjà être sélectionné par rôle et reçu par le fournisseur local de test. | Utilisable techniquement ; choix de sources personnalisé peu guidé dans l’UI ; chemin implicite d’index toujours distinct. | Réemployer comme socle et recette de référence. |
| B Ajouter une sélection mémoire guidée aux contrats natifs | Rendre visibles les leçons et consignes disponibles, les sources sélectionnées et leur transmission effective. | Extension UI/API à spécifier ; réutilisation des profils, empreintes et contrôles existants ; pas de migration de stockage nécessaire pour ce premier lot. | Premier incrément recommandé. |
| C Ajouter un exécutant Deep Agents | Offre un environnement d’exécution avec outils de mémoire et backends. | Nouveau runtime et dépendances Python ; intégration des droits, threads, budgets, reprises et preuves ; bénéfice non établi pour combler les deux écarts observés. | Différer le choix jusqu’à un besoin d’exécutant démontré. |
| D Mémoriser automatiquement les échanges bruts | Peut augmenter la quantité de contexte réutilisable. | Qualité, bruit, contenu transitoire, secrets et promotion des hypothèses à mesurer ; les tests déterministes actuels ne démontrent pas ce bénéfice. | Expérience séparée après sélection et provenance fiables. |

## Premier incrément B à spécifier avant réalisation

L’opérateur doit pouvoir examiner les sources mémoire de son projet, sélectionner
celles utiles aux rôles, prévisualiser ce qui sera transmis et enregistrer son
choix. Une tentative conserve l’instantané et les empreintes effectivement
utilisés. Les notes publiées par RETEX gardent leur provenance ; un fichier de
consignes choisi manuellement n’acquiert pas pour autant une preuve de validation.

Réutiliser les objets existants `ProjectProfile`, `ProjectInstruction` et
`ProjectContext`, ainsi que `swarm.project.json`, les contrôles de chemin et de
fraîcheur et les reçus du contexte. Le CLI `project-profile apply` sait déjà écrire
un profil personnalisé. Les routes HTTP actuelles `GET /api/v1/project-profile`,
`GET /api/v1/project-profile/catalog` et `POST /api/v1/project-profile/select`
ne constituent pas une API d’édition complète : l’évolution correspondante doit
être définie et validée côté moteur, avant son appel depuis l’UI.

Une mémoire explicitement sélectionnée et devenue invalide doit produire un
refus avant transmission, en conservant le contexte et la cause. Pour le guide
facultatif chargé implicitement, rendre toute omission visible et attribuée.
Ces deux situations doivent rester distinguables. Définir la compatibilité des
anciens profils avant de remplacer le chemin implicite.

La sélection n’accorde aucun outil supplémentaire aux planificateurs ou au
vérificateur. La publication d’une leçon conserve ses conditions de preuve. Une
configuration native fournisseur ne doit pas contourner le contrat mémoire.

## Critères observables proposés

| ID | Déclencheur et résultat attendu | Contrôles |
| --- | --- | --- |
| MEM-B01 | Une source est sélectionnée pour un rôle ; elle apparaît dans le contexte reçu par ce rôle. | Comparer sélection, aperçu, métadonnées et entrée réelle d’un fournisseur local ; rôle non sélectionné exclu. |
| MEM-B02 | Une source sélectionnée change entre aperçu, enregistrement et départ ; un état périmé est refusé sans envoi. | Changement de contenu, suppression, chemin interdit et rejeu. |
| MEM-B03 | Le guide facultatif n’est pas chargé ; la raison est visible et conservée. | Absent, illisible et trop grand ; aucun succès vide présenté comme mémoire transmise. |
| MEM-B04 | Le choix est enregistré puis relu après redémarrage. | Persistance du profil et concordance API/CLI/UI ; les tentatives historiques gardent leurs empreintes originales. |
| MEM-B05 | Deux opérateurs modifient une sélection depuis le même état. | Une modification gagne ; l’autre est refusée sur sa précondition périmée ; aucune écriture silencieusement perdue. |
| MEM-B06 | Une leçon RETEX est sélectionnée. | Préserver preuve, chemin et attribution ; le contenu reçu est celui de la révision examinée. |
| MEM-B07 | La sélection est utilisée dans les parcours concernés. | Contrats communs par rôle, UI FR/EN, thèmes, clavier, mobile et erreurs ; aucune nouvelle politique cachée. |

## Décision à consigner et travaux indépendants

Choisir le premier incrément et son périmètre produit avant M3. La recommandation
est B, avec A comme mécanisme réutilisé. Le choix n’a pas été déduit d’une
fonctionnalité annoncée : il répond aux chaînes de code et aux effets contrôlés.
La spécification détaillée peut ensuite fixer les états, le schéma d’édition,
les préconditions et la compatibilité. Aucune route ou table nouvelle n’est
présentée ici comme déjà existante.

La qualité d’apprentissage automatique reste un lot distinct : construire un
jeu de cas de préférences durables, informations transitoires, connaissances
fausses, corrections et contradictions, puis mesurer leurs effets avec les
modèles et budgets réellement autorisés. Les évaluations LLM amont fournissent
un point de départ à examiner ; leurs résultats ne sont pas supposés acquis.


## Réexamen M2 après la référence conceptuelle fournie par l’utilisateur

La page [Memory overview](https://docs.langchain.com/oss/python/concepts/memory)
distingue mémoire de conversation et mémoire partagée entre conversations,
puis faits, expériences et procédures. Elle compare profil et collection,
écriture pendant le travail et traitement séparé. Mémoire de faits et recherche
sémantique sont deux notions distinctes.

Cette référence élargit le cadrage ; les preuves M1 restent valables pour les
chemins effectivement testés. La sélection UI proposée ci-dessus traite un
accès aux connaissances existantes. Elle n’a pas été choisie comme priorité
par l’utilisateur et ne détermine pas à elle seule le besoin mémoire global.
La question de priorité précédemment présentée ne constitue pas une adoption.

Avant de figer le premier incrément, rapprocher les usages attendus du code :

| Usage à cadrer dans Swarm | Acquis observés en M1 | Analyse encore nécessaire |
| --- | --- | --- |
| Conserver le travail courant | Historique, tentatives et instantanés de contexte | Déterminer ce qui permet réellement de reprendre une conversation fournisseur et ce qui reste une pièce à relire. |
| Retenir faits et décisions utiles | Leçons sourcées et décisions persistées | Suivre extraction, correction, contradictions et réutilisation entre missions ; ne pas supposer ces opérations automatisées. |
| Réutiliser une expérience | Rapports, incidents, décisions et RETEX | Vérifier la sélection d’un précédent pertinent et son effet sur une nouvelle tâche ; archive seule insuffisante. |
| Améliorer une manière de travailler | Skills, profils et révisions de consignes | Qualifier la transmission effective et les conditions de généralisation entre tâches ou missions. |

Pour chaque usage retenu, préciser information conservée, origine, portée,
révision, moment d’écriture, récupération et observable de qualité. Examiner
le code concret derrière les modèles de mémoire référencés avant toute
transposition. Les liens historiques `memory-service` et `memory-agent` ont
été ouverts ; le premier redirige vers `langchain-ai/memory-template`, indiqué
comme archivé. Leur code n’a pas été audité dans M1 et leurs exemples ne sont
pas des composants Swarm disponibles.

Décision de méthode : conserver B comme proposition d’accès documentaire,
poursuivre le cadrage mémoire fondé sur le code avant de choisir l’incrément.
Aucun nouveau type de stockage, schéma ou agent de consolidation n’est adopté.
