# Matrice de recette de l édition et de l automatisation

Statut : contrôles à réaliser. Aucun PASS produit dans ce document. Figer commandes exactes, options, candidat et environnement lors de l'adoption de chaque lot. Toutes les données de test vivent dans un root isolé.

## Scénarios liés aux exigences

| Test | Exigences | Scénario et assertion discriminante | Lot |
| --- | --- | --- | --- |
| T01 | R01 | Créer brouillon ; vérifier rôles/critères et zéro départ après lecture/édition | B03 |
| T02 | R02 | Ajouter puis retirer dépendance ; vérifier graphe durable et conditions de départ modifiées | B01 |
| T03 | R02 | Cycle, inconnu, doublon ; état avant/après identique sur rejet | B01 |
| T04 | R03 | Deux clients depuis même révision ; un seul applique, autre récupère conflit | B01 |
| T05 | R04 | Zoom, déplacement, repli et refresh ; candidat et preuves inchangés | B03/B04 |
| T06 | R05 | Replier groupe ; afficher compte et liens masqués ; développer restaure les liens | B03 |
| T07 | R06 | Cliquer tâche ; journal/identité de tentative correspondent à sélection | B04 |
| T08 | R07 | Undo/redo brouillon ; aucune écriture de tentative et aucun rollback d'effet | B02/B03 |
| T09 | R08 | Même proposition web et CLI ; comparer gardes et effet final, pas IDs générés | B02 |
| T10 | R09 | Réponse perdue puis même clé ; un seul départ ; autre contenu même clé refusé | C01 |
| T11 | R10 | Espace occupé et dépendance non validée ; demande attend, compteur tentative stable | C01 |
| T12 | R11 | Redémarrage, DST et pause ; prochaine occurrence correcte et aucun rattrapage implicite | C02 |
| T13 | R12 | Signature mauvaise, expiration, secret révoqué, replay et contenu arbitraire ; zéro départ | C03 |
| T14 | R13 | Erreur environnement versus code ; action de reprise différente et cause conservée | C02 |
| T15 | R14 | Code changé pendant contrôle long ; reçu conservé périmé, acceptation refusée | B04/D03 |
| T16 | R14 | Changement administratif sans impact prouvé ; rattachement évite répétition injustifiée | B04 |
| T17 | R15 | Parcours clavier, focus, chargement/erreur/dialogue dans quatre variantes | B03/C04 |
| T18 | R16 | Mission ancienne et migration copie ; lecture compatible, rollback ou refus sûr | D02 |
| T19 | R17 | Processus terminé sans preuve ; jamais afficher mission acceptée/clôturée | D01 |
| T20 | R18 | Coût absent ; inconnu présent, pas total monétaire inventé ni confusion outils/tokens | B04 |
| T21 | R09/R11 | Deux conducteurs, crash après réservation ; aucune deuxième action non réconciliée | C01 |
| T22 | R12/R15 | Journaux/web/CLI ; aucun secret de test dans exports et captures | C03/D02 |
| T23 | R02/R06 | Liens responsabilité/retour ne deviennent jamais des prérequis satisfaits | B01/B03 |
| T24 | R05/R15 | Graphe 50/200/500 cartes de fixture ; flèches et focus sélection récupérables | A03/B03 |

Les tailles T24 sont des charges d'essai proposées et non une capacité produit promise. Mesurer sur machine/configuration identifiées, inclure le temps d'ouverture et interaction ; figer un budget de performance avant adoption, après baseline.

## Régressions historiques à conserver

- Flèches présentes après orientation, simplification, refresh et filtrage.
- Rôle vérificateur distinct de l'état accepté ; pas de succès vert sur une simple activité.
- Sous-planificateurs reliés au bon périmètre et retours routés réellement.
- Double clic ouvre sans départ implicite ; boutons lancer/reprendre/cancel apparaissent seulement si admissibles.
- Contrôle navigateur couvre l'effet du clic et l'état final, pas seulement le texte du bouton.
- Clé i18n modifiée : français/anglais contrôlés dans aides et erreurs, pas seulement navigation.
- Dossier de revue contient documents réellement nécessaires et références résolubles.
- Défaut de recette de supervision n'est pas imputé automatiquement au producteur.

## Vérifications complémentaires obligatoires

- **T25 — Pause et annulation** (R10/R11/R13, C02/C04) : arrêter les prochaines demandes, vérifier le sort explicite des occurrences déjà en attente ; annuler une attente puis tenter la même opération après prise en charge. Le refus n’efface aucun effet commencé et n’arrête pas implicitement l’agent.
- **T26 — Autorisation modifiée** (R03/R09/R12, B01/C03) : retirer une permission après prévisualisation, puis après réception mais avant prise en charge. Chaque action revalide la portée actuelle ; aucun jeton ancien ne force l’exécution.
- **T27 — Mission clôturée** (R09/R17, C01/C02) : soumettre une occurrence à une mission déjà clôturée ; obtenir le motif public sans clone, nouvelle tentative ou réouverture silencieuse.

## Niveaux de preuve

1. Tests purs : graphes, transitions et projections sans fournisseur.
2. Tests intégration : stockage/public operations/concurrence dans root temporaire.
3. Navigateur : DOM et rendu en FR/EN et sombre/etat avec erreurs réseau/console collectées.
4. CLI : véritable processus et codes de sortie, pas appel direct à la fonction Go.
5. Fournisseur réel : exécution bornée sur environnement identifié, si autorisée ; aucune simulation présentée comme autonomie réelle.
6. Binaire installé : provenance version et parcours ciblé après installation ; build seul insuffisant.

## Ordre des contrôles

Chaque exécutant lance les contrôles ciblés de son lot. Ne pas lancer une suite complète à chaque tâche. Configurez les contrôles réels avant la revue indépendante ; un diff sans espace final et check.py ne prouvent pas la fonction.

En D03 : suite Go pertinente et vet ; race si synchronisation changée ; frontend/i18n ; navigateur ciblé ; contrats d'agents ; diff des fichiers du lot. Exécuter la suite finale complète une fois par candidat stabilisé. Une nouvelle exécution globale exige cause nouvelle vérifiée et analyse d'impact.

Les exemples `go test ./...`, `go vet ./...` et les scripts npm existants sont à adapter aux capacités/installations effectivement inventoriées. Les recettes nouvelles reçoivent de vrais scripts versionnés avant activation dans la politique de validation. Un timer d'attente ne devient pas un test de concurrence.

## Fiche de preuve par contrôle

Exigence et scénario ; candidat Git plus diff ; empreinte des entrées ; commande/options ; environnement ; début/fin ; exit code ; résultat PASS/FAIL/PARTIAL/NOT TESTED ; trace ; limites ; identité du reviewer et contexte distinct ; action suivante si non passé. Les échecs historiques restent consultables.

## Critère final

R01–R18 couverts par preuves applicables, absence de défaut bloquant, revue indépendante réelle sur le même candidat, clôture par opération publique et version installée vérifiée. Écarts fournisseur/fixture et contraintes de licence ne doivent pas être masqués par un score global.
