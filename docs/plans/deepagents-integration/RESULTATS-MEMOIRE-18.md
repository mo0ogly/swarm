# M22 — cachets de contexte, bibliothèque et démonstration

17 octobre 2026. Codex seul, APEX borné ; `replan` sur prérequis et module de test.
**Composant et démonstration vérifiés localement ; intégration applicative restante.**

## Résultat

La demande de repères pour mémoire, protection et identité est réalisée sous la
forme d'un cachet en bas de fiche : bord irrégulier, relief discret et empreinte
centrale du type de mémoire. Les libellés adjacents séparent les dimensions.
Les états inconnus restent inconnus ; noms, références et rôles ne sont jamais
interprétés comme preuve ou autorisation par le rendu.

Le composant comprend 29 repères FR/EN : cinq types mémoire, un concept factuel
explicitement non persistant, cinq états de version, cinq protections, cinq états
d'identité, trois intervenants, trois usages et deux attributions. Chaque repère
a sa description, visible dans la légende. Les formes, mots et paires sémantiques
du thème se complètent ; aucun CDN, police distante, nouveau nuancier ou image
bitmap n'a été ajouté.

Sources : `frontend/signals/` et `frontend/build-signals.mjs`. Les deux assets
réutilisables sont `signals.js` / `signals.css`, via `SwarmSignals`. Le catalogue
permet de composer un cachet, changer langue/thème/états et revenir aux inconnus.
Documentation et exemple destinés au chantier identité :
[FR](../../SEMANTIC-SIGNALS.md), [EN](../../en/SEMANTIC-SIGNALS.md).

## Vérifications et preuves

Paquet : [`evidence/m22-20261017/`](evidence/m22-20261017/).
Préimages et périmètre dans `baseline.json`. La construction produit un
répertoire neuf hors checkout ; aucun fichier `web/`, source Go, configuration,
mission ou serveur existant n'est modifié par ce lot.

| Critère | Effet observé | Preuve |
| --- | --- | --- |
| M22.1 vocabulaire | 29 libellés/descriptions FR/EN et catégories distinctes ; données absentes affichées inconnues. | `browser-verified/result.json`, source liée par empreinte. |
| M22.2 composant sûr | Rendu DOM neuf, textes non fiables non interprétés en HTML ; ID/état/langue invalides refusés ; pas de vert par présence d'un nom ou ID ; SVG décoratifs et noms accessibles en mode icône seule. | Cinq groupes de contrôle de la recette, dont sécurité du rendu ; aucun appel API. |
| M22.3 fonctionnement | Tous les états identité/protection sont sélectionnés et répercutés ; reset garde le focus, retour aux inconnus ; accordéon clavier et focus visibles. | Recette et captures `browser-verified/`. |
| M22.3 lisibilité | FR/EN × État/sombre × 1440/1024/760/390/320 px : 20 configurations sans débordement horizontal ; tous les badges ont un contraste au moins 4,5:1. | Quatre reçus `contrast-*.json`, huit captures desktop/mobile. |
| M22.4 partage | API/conditions d'intégration, sources canoniques, règles sémantiques, limites et recette documentées FR/EN. | Documentation, liens, diff et bilan final. |

Commande : `node tests/semantic_signals_ui.cjs SORTIE_NOUVELLE`.
La recette finale est liée à dix entrées inchangées, dont le test lui-même,
les composants, les feuilles de thème et le logo. Les cinq groupes passent ;
console et requêtes échouées sont vides, aucun accès API, fournisseur ou modèle.
Les contrastes minimaux observés sont 5,75:1 en clair et 8,02:1 en sombre.
Elle ferme son navigateur et son serveur statique. Le dossier généré est conservé
avec son manifeste. Le refus d'écraser un reçu précédent est aussi vérifié.

Les captures du cachet, de la légende protection et du mobile sombre sont
inspectées visuellement. L'aperçu centré pour l'utilisateur est
[`seal-paper.png`](evidence/m22-20261017/browser-verified/seal-paper.png).
Le catalogue constitue une démonstration graphique ; sélectionner « autorisé »
ne crée pas un véritable accord.

## Incidents et RETEX

- Le chargement Node initial supposait CommonJS alors que `frontend/package.json`
  déclare ESM. Diagnostic conservé dans `module-preflight-incident.json` ; le
  navigateur charge le script classique généré et les assertions Node utilisent
  ce même fichier généré, sans changer le contrat global du frontend.
- Le premier navigateur attendu par le cache Puppeteer était absent
  (`browser-first/failure.json`). Lecture des recettes existantes et inventaire
  des exécutables : Chrome local est disponible. La recette utilise leur même
  convention `CHROME_BIN` / `/usr/bin/google-chrome`, sans téléchargement.
- Une recherche initiale trop large a inclus un bundle tiers minifié. Les
  recherches suivantes excluent les répertoires `lib/` et bornent les colonnes ;
  cette sortie tronquée ne constitue pas une preuve d'inventaire.
- La revue a reproduit six refus incorrects pour des noms hérités de JavaScript
  (`constructor`, `__proto__`, `toString`) utilisés comme identifiant/langue.
  Le dictionnaire est désormais sans prototype et la langue est contrôlée par
  propriété propre. `browser-key-regression/failure.json` garde l'échec ;
  `source-before-key-fix.js.txt` est repris octet pour octet du build fautif et
  vérifié contre son empreinte d'entrée. La recette finale vérifie dix entrées
  invalides, dont ces six variantes, et les refuse explicitement. Ce défaut de
  composant n'est pas présenté comme un contournement de l'identité du moteur.
- La recette finale ajoute la conservation des preuves (sortie neuve exigée)
  et une capture centrée du cachet. Le premier résultat navigateur PASS est
  conservé ; la relance finale vérifie cette modification de recette et lie
  les captures aux entrées exactes. Après correction des clés héritées,
  `browser-verified/` est le résultat courant. Les exécutions ne sont pas additionnées
  comme des scénarios différents.

Apprentissage réutilisable : une grammaire visuelle d'identité doit représenter
l'inconnu et le refus aussi clairement que le succès. Ne pas confondre catégorie,
état d'une version, authenticité et permission. Cette règle est dans le composant,
le guide et sa recette, pas seulement dans ce rapport.

## Reste à raccorder

La session identité peut réutiliser les deux assets et l'API depuis les sources
partagées ; aucun message inter-chat ni accusé de réception n'est revendiqué.
Les assets ne sont pas encore livrés dans les pages Go embarquées : ce choix
préserve les sources et compilations de la session IA3b active. Leur adoption
exige le chemin frontend convenu, la reconstruction concernée et une vérification
du parcours réel. Le catalogue isolé n'est pas une qualification de ces pages.

M20 reste ouvert : propriétaire, auteur attesté et droits effectifs dépendent
du contrat d'identité/autorité et du consommateur mémoire décrits dans
`CONTRAT-USAGES-MEMOIRE.md`. Aucun cachet ne répare ce contrat à lui seul ; aucune
signature, permission, preuve d'authenticité, acceptation ou diffusion n'est créée.
