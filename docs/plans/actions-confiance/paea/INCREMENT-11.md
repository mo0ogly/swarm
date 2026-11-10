# PAEA — incrément 11 : accès UX aux vérifications

[English](../../../en/plans/actions-confiance/paea/INCREMENT-11.md) ·
[Plan adopté](../PAEA-APEX.md) · [Contrat](../../../architecture/PAEA-CONTRACT.md)

## Besoin et décision

Origine : après « ces tests, comment les mettre en œuvre ? c’est un menu dans
l’UX ? », l’utilisateur précise **« c’est à ajouter »**. Propriétaire : Codex
racine, développement direct, sans délégation. Consigne antérieure conservée :
**aucune compilation ni aucun redémarrage** ; aucun commit/push/effet réel.

Attendu : accès par menu et vérification action/reçu utilisable dans l’application.
Observé : PA-08 prévoyait l’UX mais aucun écran/route PAEA n’était raccordé.
Deux options : attendre toutes les interfaces protégées PA-02…07, ou exposer
maintenant le contrôle de données disponible. Choix : premier parcours borné,
sans droit ni effet, en gardant les dépendances de l’approbation protégée.
Cela n’équivaut pas à lancer les suites de tests du dépôt depuis le navigateur.

## Changement écrit

- Menu du cockpit natif et lien depuis Actions de confiance : **Vérifications PAEA**.
- `/paea.html` : deux fichiers JSON UTF-8, service attendu, état précédent explicite,
  vérification, refus/reprise, état déclaré et rapport v2 téléchargeable.
- `/api/v1/protection/receipt-report` : admission native Host/session/origine/CSRF,
  enveloppe fermée, limite technique commune, même `check_receipt_documents` que
  la CLI ; aucune ouverture Root par requête, politique, mutation ou effet.
- Entrées transportées comme chaînes brutes, sans arrondi des nombres par JS.
  UTF-8 invalide refusé localement, BOM conservé, hashes de réponse rapprochés
  des octets sélectionnés. Les JSON restent validés par le noyau commun.
- Résultat antérieur invalidé sur modification/nouvelle demande/refus ; double
  soumission et saisies bloquées pendant la requête. `UNKNOWN` visible sans
  permission de réengager ; authenticité/autorité/effet toujours non établis.
- Assets locaux FR/EN, thèmes et accessibilité ; catalogue généré par l’outil
  existant. Dépendance locale `swarm-web → swarm-protection-contract`, aucune
  nouvelle dépendance externe, feature ou politique d’exécution.

Les sources des règles, contrats FR/EN, plan/file, notes `local_changes`, schéma
d’enveloppe et recettes sont actualisés dans cet incrément. Les notes décrivent
un raccord en sources, sans numéro de version installée inventé.

## Preuves et incidents conservés

Preuves : `evidence/20261025-ux11-xu84gb27/` dans ce répertoire.

| Critère | Preuve | Résultat et limite |
|---|---|---|
| Absence initiale du parcours | `browser-before.stderr`, `plan.json`, sources avant | Reproducteur échoue sur `web/paea.html` absent ; originaux conservés. |
| Premier composant | `browser-01/result.json` | 170 assertions, 35 requêtes ; couverture initiale sans contrôle de contraste. |
| Contraste clair et traduction | `browser-02.stderr`, captures de 01 | Échec du contraste reproduit ; lien anglais encore français observé. Corrigés avec paire `action-fond/action-encre` et catalogue. |
| Composant final | `browser-03/result.json`, `recipe-01/` | **219 assertions PASS, 40 requêtes**, 20 combinaisons FR/EN × thèmes × largeurs ; API explicitement simulée. |
| Preuve conservée | `recipe.py verify --evidence …/recipe-01` | **VERIFIED, recorded PASS** ; inventaire déclaré seulement, pas graphe compilateur. |
| Parcours natif et assets protégés | `swarm-web/tests/paea.rs` | **2 tests écrits, 0 exécuté**, sans compilation autorisée. |
| Installation, protection réelle, E2/E3 | Plan PA-02…13 | **NON TESTÉ / NON INSTALLÉ**, pas de clôture du programme. |

La recette finale couvre contraste principal, traduction de navigation, clavier,
1440/1024/760/390/320 px, absence de débordement, vide/chargement, mauvaise
réponse/refus/hash/autorité, reprise, UTF-8 invalide, conservation brute BOM/entier
au-delà de la précision JS, invalidation/export, session indisponible. Pas de
watchdog global, pas de fournisseur ni réponse métier native exercée.

Deux lectures de diagnostic du premier navigateur ont échoué : erreur de syntaxe
du script de diagnostic, puis connexion DevTools refusée après terminaison de la
recette. Le résultat final de 01 est PASS ; aucune cause de blocage de l’écran
n’a été établie. Une tentative d’édition de documentation a aussi refusé une
ancre textuelle inexacte, sans changement partiel constaté ; source relue puis
édition corrigée. Ces erreurs de méthode n’invalident pas les originaux conservés
et ne sont pas des preuves de défaillance produit.

## Revue politiques, fonctions, erreurs et documentation

`source-review.json` lie l’inventaire et la revue aux empreintes courantes ;
`owned.diff` et `before/` distinguent notre incrément du checkout déjà modifié.
Revue par le même auteur ; aucune revue indépendante ou acceptation native.

- Politiques : versions 1/2, dix états/cinq issues, SHA256 64 hex, session CSRF
  64 hex et MIME/chemins sont invariants de contrat, pas des choix opérateur.
  Booleans `busy`/`disabled` protègent un geste local ; aucun retry, timeout,
  quota ou fallback d’autorité ajouté. La borne effective reste celle du transport.
  Dimensions, couleurs et seuil 4,5:1 viennent du contrat UX ; les entrées du
  serveur double et ses défauts restent exclusivement des données de test.
- Fonctions : contrat du handler, enveloppe, erreurs `code/status/source`, assets,
  menu, helpers UI et tests revus dans leurs commentaires et consommateurs.
  Syntaxes Rust/JS et Rustfmt contrôlés ; **aucun typage Rust, Rustdoc compilé
  ou test Rust**, conformément à la consigne. Les commentaires ne prouvent pas
  le fonctionnement du raccord.
- Erreurs : accès avant décodage, enveloppe/JSON/liaison/cycle vers erreur typée,
  code public sûr, état/refus UI visible, conservation des saisies, suppression
  de tout rapport antérieur et reprise explicite. Causes originales JSON/PAEA
  conservées dans le service. Livraison/flush natifs restent non exercés ; ce
  parcours pur n’a aucun effet métier à remettre ou réconcilier.
- Documentation : guides et exemples FR/EN distinguent composant, API écrite,
  binaire servi, approbation et effet ; liens locaux contrôlés. Reconstruction
  du catalogue de traductions vérifiée, sans reconstruire l’exécutable.

## Améliorations et reprise

**PAEA-UX-01, réalisé et vérifié au niveau composant :** utiliser directement la
paire sémantique d’action pour le bouton. Origine : contraste défaillant de 02 ;
valeur : lecture de l’action principale en thème clair ; propriétaire UX ;
non-régression : contraste calculé sur les couleurs effectivement rendues.

**PAEA-UX-02, ouvert :** l’ancienne `.trust-primary` utilise aussi accent/nav au
lieu de la paire d’action. Candidat découvert en réexaminant les surfaces voisines,
pas une mesure du parcours Trust complet. Propriétaire UX Actions de confiance ;
critère : témoin et contraste rendu des boutons Trust, correction générale par
jetons communs et reprise de sa recette propre. Différé pour conserver la portée
PAEA ; aucun état « conforme » n’est donné à Trust par ce test de composant.

Prochaine frontière : compiler/typer et exécuter les deux tests HTTP, puis le vrai
parcours navigateur sur un candidat de profil normal, vérifier octets embarqués
protégés, notes/version et livraison locale après levée explicite de la consigne.
PA-08 reste partiel : producteurs protégés, accords exacts, deux UX séparables,
configuré/appliqué/observé et vigilance doivent encore être établis. L’écran de
données ne remplace pas PA-02…07, les 17 invariants ni les moyens tiers E3/RETEX.
