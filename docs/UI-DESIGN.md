# Identité et ergonomie de Swarm

Direction adoptée le 10 octobre 2026 pour la refonte du cockpit, de la préparation,
de l’accès et des sessions d’agents. Cette référence guide les prochaines évolutions.
La demande : une interface reconnaissable, moins administrative, une navigation
regroupée et moins de lecture obligatoire, dans les deux thèmes.

## Intention

Swarm est un atelier de coordination : mission, agents, dépendances et décisions.
Son logo (lampe, engrenage et connexions) fournit l’identité. Le graphe est une
surface de travail centrale. Ses connexions arrondies suivent les routes du moteur
de placement, en écho aux courbes du logo. Une interface reconnaissable vient aussi de sa
composition : rail marine, titres de mission à empattements, repères numérotés,
lignes de connexion et détails à déplier. Éviter les tableaux de bord interchangeables
avec quatre cartes colorées, grands dégradés et panneaux imbriqués.

## Sources et architecture

- Logo canonique : `web/swarm-logo.png`, conservé sans transformation.
- Base historique des jetons : `web/wattson_themes.css`.
- Système partagé actuel : `web/swarm-design.css`, chargé après les styles des vues.
  Il réaffecte les jetons sémantiques existants pour conserver les composants.
- Comportements de présentation : `web/swarm-shell.js`.
- Identifiants historiques de thèmes : `etat` = clair, `sombre` = sombre.
  Les conserver pour les préférences, liens et tests existants.
- Les préférences de présentation sont locales ; elles n’altèrent jamais une mission.

Le choix d’une feuille commune évite quatre identités divergentes et une réécriture
des composants métier. Les anciens styles par vue restent responsables de leurs
comportements spécifiques. Ne pas ajouter une deuxième couche de surcharge : faire
évoluer le système commun et retirer les anciennes règles devenues inutiles à mesure
que leur périmètre est vérifié.

## Palette et typographie

Palette UI dérivée du logo (valeurs de travail, pas prétention à une extraction exacte) :

| Usage | Sombre | Clair |
| --- | --- | --- |
| Fond | `#081b29` | `#f5f3ec` |
| Surface | `#102b3b` | `#fffefa` |
| Texte | `#e5edf0` | `#243d4a` |
| Titre | `#f6f3e9` | `#062b49` |
| Connexions / liens | `#93d4df` | `#205e70` |
| Action principale | Doré `#f8c650`, encre marine | Marine `#062b49`, encre blanche |
| Accent lisible | Doré `#f8c650` | Ocre `#926300` |

Le rail reste marine dans les deux thèmes, avec ses propres paires de texte/fond.
Le doré signale la sélection et les moments clés. Le cyan accompagne les connexions.
Les états utilisent les paires succès, attention, alerte et information ; ne pas
remplacer le rouge d’erreur par la couleur de marque. Toujours doubler la couleur
par un libellé, une forme ou un motif. Aucun texte doré clair sur fond blanc.

Interface : pile système locale (`Segoe UI`, `system-ui`). Titres de mission :
Georgia locale, 28–42 px. Identifiants, repères et compteurs : monospace.
Aucune dépendance à une police ou un service distant. Texte courant confortable,
petits libellés réservés à l’information secondaire. Rayons : contrôles 5 px,
surfaces 5–8 px. Ombres surtout pour les dialogues et inspecteurs.

## Navigation

| Groupe | Destinations |
| --- | --- |
| 01 Missions | Gérer les missions, préparer un projet |
| 02 Préparation | Dialogue IA |
| 03 Conduite | Graphe, tâches, agents et hiérarchie, décisions |
| 04 Suivi | Journaux, budgets et coûts IA, reprise et OODA |
| 05 Configuration | Programmes, IA et connexions, administration |

- La mission sélectionnée reste au-dessus de la navigation.
- Les groupes sont des accordéons HTML natifs ; plusieurs peuvent rester ouverts.
- Le groupe actif s’ouvre automatiquement, y compris sur un lien direct.
- Les choix d’ouverture sont mémorisés sur l’appareil.
- Sur ordinateur, « Replier le menu » masque le rail entier et rend sa largeur
  au contenu. « Ouvrir le menu » reste visible dans le contenu, même au défilement.
  Le choix est mémorisé (`swarm-nav:collapsed`), sans changer le zoom, la sélection
  ou le plan. Le menu mobile conserve son ouverture indépendante.
- Les deux modes gardent toutes les destinations accessibles. Le mode simplifié
  réduit les informations, sans faire disparaître une fonctionnalité du menu.
- Le compteur de décisions provient de la même donnée que l’indicateur du cockpit.
  Un accès direct reste visible quand des décisions attendent, même avec le groupe fermé.
  Les retours du planificateur ouvrent leur propre détail ; les alertes ouvrent la vue Décisions.
- Connexion visible ; apparence, langue, aide et version dans un accordéon d’outils.
- Sur mobile, le menu est un panneau dans le flux ; Échap le ferme, une destination
  sélectionnée le ferme et place le focus dans le contenu.

## Ordre de lecture et accordéons

La liste complète « Résultats et prochaines actions » est construite à l’ouverture
de son accordéon. À l’actualisation, une liste ouverte est reconstruite avec les
données courantes, ses sous-accordéons et son focus conservés. Une liste fermée
diffère cette construction ; les interventions prioritaires restent visibles.
Les lectures initiales de session et des missions sont parallèles ; aucune
commande n’est envoyée avant la réception du jeton de session.

Le placement du graphe possède un cache optionnel, limité à une géométrie par
onglet (`sessionStorage`, `swarm-pilot-layout:v1`). Sa signature inclut la version
dagre, ses paramètres, les dimensions des nœuds et la forme affichée. Dépendances,
orientation, filtres, détail ou responsabilités différents provoquent un recalcul.
Les états, libellés, coûts et actions sont toujours lus dans le snapshot courant.
Une entrée invalide, un stockage refusé ou plein revient au placement normal.
Changer le format du cache impose d’incrémenter sa version. Les preuves doivent
distinguer le premier affichage sans géométrie et les rechargements du même onglet.

1. Mission et objectif.
2. Situation courante, action principale et problèmes qui demandent une intervention.
3. Recherche / filtre / vue et commandes immédiates du graphe.
4. Graphe ou liste.
5. Explications et outils secondaires à la demande.

Les règles de lancement, d’autorisation et de revue restent celles du moteur.
Ne jamais replier une erreur de formulaire, un refus de lancement ou la cause
principale d’un blocage. Les réglages avancés et longues preuves peuvent être
repliés si leur sommaire explique ce qu’ils contiennent.

Accordéons ajoutés ou harmonisés : groupes du menu, outils de l’interface,
consignes du projet, IA/contexte de la conversation, affichage/navigation du
graphe, équipe/limites/état détaillé. Les accordéons existants (activité, résultats,
assistant, réglages de conduite) partagent ce vocabulaire. La synthèse de mission
et ses actions précèdent désormais le graphe. Les résultats conservent leur accès
explicite et leur état d’ouverture au rafraîchissement.

Les préférences `swarm-nav:*` et `swarm-reading:*` n’enregistrent ni documents,
ni secrets, ni décisions métier. Les détails de mission sont distingués par mission.
Les résumés futurs peuvent afficher un compteur réel ; ne jamais inventer un état
rassurant pour raccourcir le texte.

## Accessibilité, langues et états

- Utiliser `details`/`summary`, boutons et liens natifs. Pas de faux boutons en `div`.
- Focus visible, lien d’évitement, navigation au clavier et restitution du focus.
- Cibles tactiles d’au moins 44 px ; saisie à 16 px sur écran tactile.
- Respecter `prefers-reduced-motion`. Pas d’animation continue du graphe.
- Vérifier à 1440, 1024, 760, 390 et 320 px. Pas de débordement global ; le graphe
  et les tableaux peuvent garder leur défilement local.
- FR/EN via `locales/en.json` et `npm run i18n:build` ; ne pas traduire les titres
  de missions, réponses IA ou documents utilisateur.
- Chargement : état explicite, contrôles inactifs seulement si nécessaire.
- Vide : expliquer le prochain geste sans simuler une activité.
- Erreur : laisser le message et la saisie visibles ; offrir la reprise existante.
- Succès : refléter les données confirmées du moteur, pas une animation optimiste.

## Vérification et limites

Le rapport de cette refonte est `docs/reports/ui-redesign-20261010.md`.
La recette `tests/design_system_ui.cjs` couvre l’accès aux destinations, les
accordéons, les deux langues/thèmes, le responsive et les erreurs navigateur.
Compléter par les recettes existantes de préparation, connexions et navigation produit.
La recette de conception utilise des missions temporaires isolées et ne lance aucun
agent ; les recettes de sessions utilisent des fournisseurs locaux de test.
Le bouton d’ajout d’une connexion attend la fin de son chargement : aucun clic
sans effet pendant la lecture des données. Le logo et la feuille de style de
l’écran de connexion sont publics ; les scripts métier, données et commandes
restent soumis à la session. L’arrêt depuis une session d’agent utilise la tâche
attribuée par le snapshot courant et attend la confirmation du superviseur ;
il ne valide pas le résultat de la tâche. La page de préparation désactivée charge elle aussi
le système graphique commun.

Une compilation ne remplace pas la vérification du binaire effectivement servi.
Une vérification locale de l’auteur n’est pas une revue indépendante.

Le contrôle Go d’intégrité utilise `docs/ui-redesign-source-manifest.json` après
les instantanés historiques. Après une évolution des sources et des preuves UI,
`python3 tools/refresh_ui_source_manifest.py` recalcule cet instantané, puis
`go test ./internal/engine -run TestGraphDeliveryD03QualificationManifest -count=1`
vérifie son intégrité. Cette opération ne renouvelle aucune acceptation historique
et ne remplace jamais l’exécution des recettes comportementales.

## Captures et formation embarquée

Les vidéos et les images du lecteur Casa Pizza sont des fichiers enregistrés :
la CSS du cockpit ne les transforme pas. Après une évolution graphique, recapturer
les étapes concernées avec `tools/verification/training_ui_capture.cjs` dans son
atelier temporaire, vérifier les images, puis reconstruire les modules concernés.
`build_graph_overview.py` assemble la présentation française ; `build_media.py`
accepte `--modules` et `--swarm-recorded` pour conserver les autres enregistrements.
Exécuter la présentation avant le générateur de modules pour inclure ses empreintes
dans `manifest.json`, puis `build_player.py`, `build_kits.py` et reconstruire le binaire.

Dater chaque module selon sa véritable capture. Le lecteur indique les dates
distinctes Swarm/client/restaurant ; les captures historiques des PDF et DOCX
restent datées. `tests/training_design_ui.cjs` vérifie le lecteur embarqué et les
deux kits extraits : FR/EN, vidéos, décodage, déplacement dans la vidéo, absence
de lecture automatique, thèmes et largeur mobile.
