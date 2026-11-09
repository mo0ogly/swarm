# Maquettes de conception Swarm

Maquettes produites le 8 novembre 2026 avec Claude Design, réunies avec les
retouches UX des copies locales `design-swarm` et `design-admin`. Ce sont des
**propositions visuelles avec données d’exemple**, sans connexion au moteur,
à l’API, aux fournisseurs IA ni aux écrans servis dans `web/`.

| Dossier | Canvas Claude Design d’origine (privé) | Planches |
| --- | --- | --- |
| `swarm/` | https://claude.ai/artifact/P4U3fe9gRnrWAx7Kh1177h | `Main.dc.html` : cockpit Graphe ; `Couloirs.dc.html` : cockpit par rôles et phases ; `Preparation.dc.html` : dialogue et brief ; `Securite.dc.html` : Administration › Sécurité ; `Audit.dc.html` : Suivi › Audit et preuves |
| `admin/` | https://claude.ai/artifact/8QxgcH2oP1xv7JR9UH9cAM | `Main.dc.html` : référence Administration ; `Proposition.dc.html` : proposition des limites d’exécution |

## Ouvrir les maquettes localement

Ouvrir [`swarm/Main.dc.html`](swarm/Main.dc.html) dans Chrome. La barre
« atelier UX » relie les sept planches et permet de changer de thème. Aucun
serveur n’est nécessaire. Garder l’ensemble de `docs/design/` : les planches
utilisent `preview.js`, `preview.css` et la copie locale du logo canonique
`web/swarm-logo.png`. Les références de blobs privés sont remplacées par ce logo.

Les balises `<x-dc>`, `<sc-for>`, `<sc-if>` et les classes `DCLogic` restent dans
les fichiers `.dc.html` ; chaque `canvas.json` conserve l’inventaire du canvas.
`preview.js` est un adaptateur limité à ces planches. Il évalue leurs expressions
et leur code local de confiance ; ne pas lui donner un document non fiable.
L’import retour des retouches dans Claude Design n’a pas été vérifié. Le canvas
distant et cette copie versionnée ne sont pas automatiquement synchronisés.

## Parcours proposés

- **Conduite** : Graphe et Couloirs reliés par un sélecteur. Les badges utilisent
  un texte, une forme et une couleur. La recherche du graphe atténue les tâches
  sans correspondance. La grille des interventions s’adapte aux petites largeurs.
  La mémorisation de la vue dans le produit reste une proposition non implémentée.
- **Préparation** : conversation et brief côte à côte sur ordinateur, panneaux
  empilés sur mobile. Les réponses de démonstration actualisent le brief ; « Je
  précise » ne suffit pas à compléter le périmètre. Aucun message n’est envoyé.
- **Limites d’exécution** : héritage explicite, brouillons de démonstration par
  portée, motif, aperçu des changements puis simulation d’enregistrement.
  Aucun réglage réel ni historique moteur n’est modifié. Les nombres positifs
  sûrs constituent seulement la validation de cet exemple, pas une politique du
  produit. Les valeurs héritées sont illustratives, sans résolution moteur.
- **Sécurité** : proposition de regroupement dans Configuration › Administration
  › Sécurité : intégrité Biba, confidentialité Bell-LaPadula, conflits de rôles,
  identités et clés de preuve. Les niveaux I1/I2, classifications, compartiments,
  sujets et paires de rôles sont des exemples. Le bouton d’aperçu illustre un état
  de maquette ; il ne valide aucune politique et ne prouve aucun droit.
- **Audit et preuves** : proposition dans Suivi, avec transactions illustratives,
  étapes de preuve et exemple de verdict public. Le bouton de vérification ne
  vérifie aucune signature : il affiche un **exemple de verdict**. Export, ancre,
  archives et compteurs restent illustratifs. Les gestes demander/approuver/
  engager restent une piste de placement dans Conduite › Décisions.

## Sources et portée des affirmations

Direction visuelle : [`../UI-DESIGN.md`](../UI-DESIGN.md). Les fichiers
[`../../web/index.html`](../../web/index.html) et
[`../../web/admin.js`](../../web/admin.js) ont servi à la composition initiale.
Les champs et avertissements de sécurité se rapprochent de
[`../../web/trust-native.html`](../../web/trust-native.html) : formulaire Biba,
champ Admin I3, confidentialité, matrice de rôles, clés et audit de transaction.
[`../../web/trust-native.js`](../../web/trust-native.js), fonction `api` et
écouteur `settings.close`, montrent la sélection du jeton par opération et
l’effacement du champ Admin à la fermeture. Cela établit les sources UI lues,
pas une qualification du contrôle moteur ni la validité d’un exemple affiché.

Les pages `trust-native.html`, `security-proofs.html`, `trust-actions.html` et
`paea*.html` sont des références pour la réorganisation proposée. Cette PR ne
modifie pas leur navigation. L’avertissement de `trust-native.html` sur le
contrôle transversal BLP doit rester pris en compte lors d’une intégration :
une politique enregistrée ou une maquette ne prouve pas ce contrôle.

## Vérifier et reprendre

Depuis la racine du dépôt, après installation des dépendances Node existantes :

```sh
node docs/design/verify-preview.cjs CHEMIN_PREUVES_NEUF
```

Le script utilise `CHROME_BIN` ou `/usr/bin/google-chrome`, comme les recettes
existantes. Il contrôle les sept planches et produit des assertions nommées,
`checks.json` et des captures. Voir [`VERIFICATION.md`](VERIFICATION.md) pour
les résultats, la provenance et les limites. Les sources de maquettes sont les
seuls fichiers concernés : aucun changement produit, installation, lancement
d’agent, approbation indépendante ou fusion de PR n’est établi par ces contrôles.
