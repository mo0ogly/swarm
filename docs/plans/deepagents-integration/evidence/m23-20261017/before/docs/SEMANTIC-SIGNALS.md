# Cachets de contexte et repères partagés

Composant de présentation réutilisable, inspiré d'un cachet de cire au bas d'un
parchemin. **Le composant et le catalogue isolé sont livrés ; les écrans métier
existants ne les chargent pas encore.** Aucun cachet n'émet de droit, ne signe un
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
le consommateur. Pour intégrer aux vues embarquées, ajouter ces deux fichiers
par le chemin de livraison frontend convenu, puis reconstruire et vérifier les
écrans concernés. Ne pas recopier les pictogrammes dans chaque vue, ni modifier
un fichier généré à la main. La démo n'a pas besoin d'être livrée avec l'application.

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
et captures sont conservés. Il reste à vérifier chaque écran réel lors de son
intégration ; ce test isolé ne prouve pas l'application des droits du moteur.
