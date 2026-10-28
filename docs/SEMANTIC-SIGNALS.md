# Cachets de contexte et repères partagés

Composant de présentation réutilisable, inspiré d'un cachet de cire au bas d'un
parchemin. **Le composant, le catalogue isolé et le raccord à la page mémoire
sont livrés dans les sources.** Aucun cachet n'émet de droit, ne signe un
document et ne certifie une identité. [English](en/SEMANTIC-SIGNALS.md).

## Forme et vocabulaire

Une empreinte centrale indique le type de mémoire ; un bord irrégulier, un
double cerclage et une ombre discrète évoquent la cire. À côté, trois libellés
séparent le type, la protection de l'usage et l'état de l'identité. Propriétaire,
auteur et référence sont affichés en texte lorsqu'ils sont fournis. Sans données,
le cachet montre **Droits non établis** et **Identité non attestée**.

Les couleurs viennent uniquement des jetons partagés Swarm. L'or du cachet est
un repère de présentation ; les états ont leurs formes et leurs mots, sans
autorité déduite de la couleur ou de l'ornement.

| Famille | Repères et sens |
| --- | --- |
| Mémoire, 5 | Type inconnu ; expérience passée ; procédure ; cadrage question/réponse ; résumé extractif. |
| Concept, 1 | Faits : repère de mémoire sémantique, **pas un nouveau type persistant**. Les types du catalogue mémoire actuel restent `experience` et `procedure`. |
| Version, 5 | Inconnue ; active dans le catalogue ; historique ; retirée du réemploi ; indisponible dans le contexte. Active ne signifie pas autorisée. |
| Protection, 5 | Droits non établis ; décision attendue ; usage autorisé pour son contenu/destinataire/durée ; usage refusé ; autorisation révoquée. |
| Identité, 5 | Non attestée ; à vérifier ; attestée ; suspendue ; révoquée. Une identité attestée n'octroie aucun droit implicite. |
| Intervenant, 3 | Personne, agent, service : catégories, sans preuve d'authentification automatique. Un service n'est pas une approbation humaine. |
| Usage, 3 | Consulter, résumer, transmettre : droits indépendants. |
| Attribution, 2 | Propriétaire : responsabilité/gestion ; auteur : origine de l'intervention. |

Les 29 repères ont un libellé et une description FR/EN dans le catalogue.
Retrait et révocation conservent les auteurs, textes et réceptions historiques.
Le refus doit conserver un motif visible dans le parcours qui utilise le cachet.

## Sources et construction

Sources canoniques :

- [signals.js](../frontend/signals/signals.js) : API sans dépendance, SVG locaux
  construits par le DOM, dictionnaire bilingue et états explicites.
- [signals.css](../frontend/signals/signals.css) : badges et cachet, uniquement
  avec les paires sémantiques du thème.
- [catalogue.html](../frontend/signals/catalogue.html), `catalogue.js` et
  `catalogue.css` : démonstration interactive, légende et descriptions.

Créer une sortie **neuve hors du checkout**, dont le parent existe :

```sh
node frontend/build-signals.mjs /tmp/mon-catalogue-cachets
```

Le build copie les cinq fichiers, les deux feuilles de jetons Swarm et le logo
local ; `build-manifest.json` contient leurs empreintes. Il refuse une sortie
existante ou résolue dans le checkout, y compris via un lien symbolique. Cela
évite de modifier les sources ou ressources embarquées pendant le chantier
identité actif. Il ne reconstruit pas le binaire et ne déploie pas de page.
Le catalogue est `catalogue.html` dans cette sortie ; il fonctionne sans API.

## Réutilisation par la session identité et les autres vues

Charger `signals.css` après les jetons et `signals.js` comme script local avant
le consommateur. Le chemin de génération des deux copies embarquées est :

```sh
node frontend/embed-signals.mjs
node frontend/embed-signals.mjs --check
```

Le premier copie les deux assets canoniques vers `web/` ; le second refuse une
copie absente ou différente sans la modifier. Ne pas modifier ces copies à la
main. Leur génération précède la compilation Go (`go build -o /tmp/swarm-candidat
./cmd/swarm`) et doit être coordonnée avec toute compilation consommant les
mêmes ressources. Le catalogue n'est pas embarqué. La session identité peut
charger ces mêmes URL locales sans dupliquer les icônes ; son raccord reste à
réaliser et à vérifier dans son propre périmètre.

`project-memory.html` charge les deux assets. Chaque fiche termine par un
cachet dont le type vient de `version.kind`, et une référence travail/leçon/version.
Le badge près de la sélection distingue `active`, `historical`, `unavailable` ;
`version.action === 'withdraw'` donne « Retirée ». Un type ou état inconnu reste
inconnu. Les cases demeurent sélectionnables uniquement pour `status === 'active'`.
Les droits et l'identité restent inconnus ; le propriétaire et l'auteur sont
« Non renseigné ». `version.actor` conserve sa place dans l'historique, sans
devenir une attribution certifiée.

```js
// Dans un fichier JS local de la vue, après chargement du composant.
const cachet = SwarmSignals.seal({
  memory: 'procedure',
  lang: 'fr'
}); // Identité et droits restent inconnus tant que non fournis.
container.append(cachet);

const lecture = SwarmSignals.badge('usage-read', {lang: 'fr'});
const inconnu = SwarmSignals.state('identity', undefined, {lang: 'fr'});
```

`seal` accepte aussi `protection`, `identity`, `owner`, `author`, `reference`.
Les trois dernières valeurs sont du texte affiché, **jamais une source d'autorité**.
`memory` accepte les suffixes `experience`, `procedure`, `dialogue`, `summary`,
`knowledge` ; un type non reconnu devient inconnu. Les états de protection sont
`unknown`, `pending`, `allowed`, `denied`, `revoked` ; ceux d'identité sont
`unknown`, `pending`, `verified`, `suspended`, `revoked`.

`badge(id, {lang, iconOnly})` et `state(family, value, options)` rendent un DOM neuf.
`family` vaut `identity`, `protection` ou `version`. Une valeur absente reste
inconnue ; un état/identifiant ou une langue non pris en charge produit un
`RangeError`, sans remplacer l'erreur par un succès visuel. `lang` vaut `fr`
(défaut) ou `en` ; un document DOM peut être passé explicitement via `document`.

Le consommateur doit relier les états aux données vérifiées du moteur. Ne pas
déduire `verified` d'un nom, d'un fournisseur, d'un ID ou du rôle Admin ; ne pas
déduire `allowed` de l'authentification, d'un hash, de l'existence d'une version
active ou d'un accord historique. Le
[contrat mémoire cible](plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md)
reste distinct : son autorisation réelle n'est pas fournie par ce composant.

## Accessibilité et vérification

Les badges ont un texte visible ; leur SVG est décoratif, hors du focus clavier.
`iconOnly` donne au conteneur `role="img"` et un nom accessible localisé. Le
cachet conserve ses libellés ; ses champs utilisent `textContent`, sans HTML
de provenance. Les descriptions sont visibles dans la légende et disponibles
en titre sur le badge. Aucun CDN, police distante, image générée, script inline,
écouteur HTML ou accès réseau applicatif.

```sh
node tests/semantic_signals_ui.cjs /tmp/ma-verification-cachets
```

La recette utilise Chrome local (`CHROME_BIN`, sinon `/usr/bin/google-chrome`),
un build temporaire et un serveur statique de test, tous les processus de test
étant fermés en fin de recette. Elle vérifie les états, le retour à inconnu,
les textes malveillants, FR/EN, les deux thèmes, 1440/1024/760/390/320 px, le
contraste des badges, le clavier, le focus et les erreurs/réseau. Les résultats
et captures sont conservés. Ce test isolé ne prouve pas l'application des droits.

La recette de page `tests/project_memory_ui.cjs BINAIRE BUNDLE SORTIE_NEUVE`
vérifie les ressources embarquées du binaire fourni. Son quatrième argument
optionnel `DOSSIER_ASSETS` sert les cinq assets mémoire candidats dans le
navigateur, au-dessus de l'API réelle du binaire conservé. Ce mode frontend est
explicitement inscrit dans `environment.json` et ne prouve pas une compilation
Go actuelle. Les variantes de transport synthétiques restent distinguées des
admissions/remplacements/retraits réels. Elle conserve empreintes, transitions,
contexte reçu par un fournisseur local scripté et diagnostics. Voir le
[rapport M23](plans/deepagents-integration/RESULTATS-MEMOIRE-19.md) pour le mode
effectivement exécuté et la limite du serveur utilisateur non redémarré.
