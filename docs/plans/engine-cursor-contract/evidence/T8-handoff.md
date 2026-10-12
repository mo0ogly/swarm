# T8 — RETEX et quick wins de la livraison directe

**PARTIAL — recette locale terminée pour les derniers correctifs, recette réelle
et revue indépendante T7 encore attendues.** Ce bilan ne clôt pas la mission
historique et ne fabrique aucune acceptation native.

## Résultat observé

La reprise directe a corrigé les cycles de jugement, la planification, les remises
et politiques décrites dans T1–T7. Le dernier lot ajoute l'atelier du graphe et
retire la limite fixe de chargement de l'éditeur. Les rapports, empreintes et
limites restent dans leurs reçus d'origine ; les tests ciblés ne sont pas comptés
comme des tâches acceptées.

[Reçu du graphe, de l'éditeur et de la console](direct-20261012/graph-studio-verification-receipt.json).
[Recette de reprise directe](direct-20261012/verification-receipt.json).

## Quick wins réalisés et vérifiés

| Défaut observé | Correction générale | Preuve |
| --- | --- | --- |
| Échap ne reçoit pas le geste de déplacement | La poignée et le port prennent le focus avant capture du pointeur | Annulation au pointeur dans les quatre variantes navigateur |
| Mini-carte interceptant le port d'arrivée | La mini-carte informative laisse passer les gestes | Diagnostic original `pointer_hit: graph-minimap`, puis port atteint et brouillon enregistré |
| Ancien brouillon prévisualisable après un refus d'enregistrement | État non enregistré et aperçu invalidé ; confirmation indisponible | Garde de refus/récupération `graph_studio_test.cjs` |
| Monaco calcule le retour à la ligne d'une console cachée | Messages conservés ; rendu reporté jusqu'à visibilité du conteneur | Original `reading top`, cinq rechargements sans erreur, recette console visible |
| Chargement éditeur interrompu à 15 secondes fixes | Délai courant Admin partagé, 0 sans expiration, temporisateurs nettoyés | Chargement retardé : 250 ms → texte éditable/sauvegardé ; 0 → Monaco disponible |
| Les diagnostics masquent une erreur derrière `JSHandle@error` | Conserver message et pile de l'erreur navigateur | Cause Monaco récupérée et gardée dans les preuves |

## Leçons de méthode

Les tests au clavier seuls n'auraient pas détecté l'interception du port par la
mini-carte. La vérification du chemin complet inclut donc aussi le point réellement
atteint, le brouillon public, l'aperçu, la confirmation et l'état relu. Une recette
sur un seul état visible n'aurait pas révélé le défaut de console cachée : les
messages doivent rester disponibles lors des changements de vue.

Deux autres échecs concernaient mon harnais : compilation de la bibliothèque à
la place du CLI, et tentative de focus Puppeteer réservée aux éléments HTML sur
un nœud SVG. Ils ont été corrigés avant les recettes et ne sont pas attribués au
moteur. Une dépendance vide peut être `null` dans la réponse : le harnais respecte
le producteur au lieu d'imposer un tableau absent. Les originaux restent en cache.

## Mesures et limites

Pour 50 cartes, cinq rechargements donnent un p95 de 739 ms ; les cinq interactions
clavier donnent 31,9 ms. Il s'agit d'un environnement local, sans comparaison de
coût fournisseur ni preuve à 200/500 cartes. Le dernier lot appelle zéro modèle
réel. Le coût global et les tokens des exécutions historiques ne sont pas déduits
de ces mesures. Les interventions sont celles de Codex dans cette conversation,
pas une récupération autonome démontrée de plusieurs superviseurs.

La clôture de T8 dépend encore de T7 : modèle réel et revue indépendante du même
candidat dans les droits autorisés, correction des constats éventuels, puis bilan
cumulatif final. Propriétaire de cette décision manquante : utilisateur. Le serveur
historique reste inchangé ; sa mise à jour exige une reprise compatible avec les
autres travaux qui partagent ce processus. Le développement livré et l'acceptation
native restent distincts.

## Complément vérifié après le bilan local initial

La recette configurable du graphe avait dérivé du format de sa recette source et
conservait des constantes dans le verdict final. Le correctif restaure le parcours
et utilise la politique figée pour tous les verdicts ; les variantes strictes,
plus souples et égales sont vérifiées sur le code réellement généré. La mesure
à 500 cartes passe sans relever la politique : p95 1 342 ms / clavier 31 ms,
cinq observations, démarrage à froid séparé 1 691 ms. La restriction précédente
« 200/500 non requalifiés » reste historique pour le premier reçu ; 200 cartes
n'est toujours pas requalifié, 500 possède désormais son reçu propre.

Les cinq plafonds de réglage sont reproduits comme refus avant correction puis
remplacés par les contraintes de représentation. L'application et l'aperçu doivent
également refuser une configuration trop volumineuse avant toute persistance ;
un garde de régression porte ce cas. L'historique et les anciennes mesures ne
sont pas modifiés pour faire apparaître un succès rétroactif.

La recette Admin multi-profils a également révélé un désaccord de producteur et
consommateur : le bouton d'ajout fournissait `render`, tandis que la ligne lisait
`render_p95_ms`. Le moteur n'était pas appelé car la validation HTML détectait
le champ vide. Correction du nom au producteur et garde de recette sur le champ,
puis sauvegarde/relecture des onze profils. Le retour de focus doit suivre la
relecture asynchrone, afin de ne pas se perdre lorsque le bouton est désactivé.

Le [reçu de ce complément](direct-20261012/performance-policy-verification-receipt.json) contient les empreintes, les onze tests et sous-cas Go et les quatre recettes Admin réellement terminées. Le build et `go vet ./internal/engine` passent. Aucune suite globale de 884 tests n’est rejouée.

## Prévenir les reprises sans progrès

L'audit de clôture conservait une séquence de travaux déjà réalisés et la page
d'architecture demandait encore une décision T1 au futur. Ce guidage pouvait
faire repartir la prochaine activation sur des étapes terminées. La décision,
sa comparaison et ses sources sont maintenant explicites en FR/EN ; l'audit
courant dirige la reprise vers T7 et sa décision manquante. Les preuves passées
restent inchangées. La leçon rejoint la règle existante de replan : une reprise
part de l'état réel, apporte une preuve ou une précondition nouvelle, et distingue
un processus actif d'une autorisation encore attendue.
