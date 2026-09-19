# Monaco embarqué pour Préparer

Versions épinglées : Monaco 0.56.0, esbuild 0.28.2. Sources d’intégration :
[guide ESM officiel](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)
et les déclarations `editor.api.d.ts` de la version installée.

Depuis ce dossier :

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run build
```

Le build remplace uniquement `../web/lib/monaco/`. Il produit des modules ESM,
CSS, polices, workers et licences locaux. Le binaire Go les embarque au build ;
aucun npm ni accès Internet n’est nécessaire sur le poste qui utilise la page.
`bundle-meta.json` conserve l’inventaire. Le bundle généré est du code tiers :
il n’est pas présenté comme conforme au scanner de couleurs du code applicatif.

## CSP

Deux fabriques de feuilles de style Monaco reçoivent le nonce propre à la réponse
HTML de `/prepare.html`. Le build applique une transformation exacte et refuse
une dérive de ces deux points. Les sources dans node_modules restent intactes.
La modification est limitée à l’ajout de `style.nonce` ; elle est traçable dans
build.mjs. Le thème applicatif est produit depuis les tokens Wattson calculés.

Monaco génère également des attributs style de positionnement. La page de
préparation ajoute donc `style-src-attr 'unsafe-inline'`. Les feuilles style
restent soumises à `self`/nonce, les scripts à `self` ; pas d’unsafe-eval, de CDN
ou de script inline. L’exception ne s’applique ni au cockpit ni aux API. Le texte
utilisateur est rendu par les API d’éditeur, textContent ou textarea ; aucun HTML
utilisateur n’est inséré. Cette portée est vérifiée par web_prephase_test.go.

## Portée actuelle

Édition Markdown/JSON, recherche Monaco, sauvegarde/historique via l’API partagée,
secours textarea sur mobile ou erreur de chargement. Le mode textarea classique
Monaco est choisi (`editContext: false`) plutôt que son EditContext expérimental.
L’indentation, la fermeture automatique et le formatage à la saisie/collage sont
désactivés pour préserver le texte documentaire fourni.
Le dialogue IA est raccordé côté serveur et dans prephase-dialogue.js ; ce bundle
fournit seulement l’éditeur. L’édition visuelle du plan et sa conversion en missions
restent des lots suivants.

Les scripts applicatifs résident dans web/prephase*.js et prephase.css. L’utilitaire
security.js du projet est copié sans modification dans web/lib/preparation-security.js.
Une mise à jour de cette dépendance locale exige de relancer le build.


Le terminal natif utilise xterm.js 6.0.0 et addon-fit 0.11.0. `npm run build`
reconstruit l’éditeur et le terminal ; `npm run build:terminal` reconstruit seulement
`web/lib/terminal`. Trois créations de styles upstream sont patchées pour le nonce
CSP (compilation refusée si dérive). Les couleurs de la feuille upstream sont
remplacées par des jetons ; la palette ANSI et le contraste sont définis à partir
des jetons dans `terminal.js`. Les licences MIT sont livrées avec le bundle.
