# Diagnostic de reprise fondé sur l’état du dialogue

16 octobre 2026, reprise « next », Codex seul, APEX et `replan`.
Exigences MEM-U08, MEM-U12, MEM-U13 ; cas MEM-C15 et MEM-C16.

## Constat et choix

M11 conserve les gardes et usages, mais `mission recovery` et l’API
`recovery-preview` ne les consultent pas. `recoveryPreviewFor` décrit toujours
une nouvelle tentative. Le responsable doit pouvoir distinguer une frontière
confirmée, un échange encore actif, un état ancien absent et un effet incertain.

Une restauration directe demanderait aussi de rouvrir les réservations,
réacquérir l’espace, vérifier la session chez le fournisseur et enregistrer une
décision fraîche. Les opérations actuelles de `task.restart` et de reprise
corrective autorisent une nouvelle production ; elles ne restaurent pas la même
conversation. Réutiliser ces noms pour restaurer implicitement serait incorrect.

Choix : enrichir le diagnostic public existant, reçu par la CLI et le panneau
« Avant une relance ». Il utilise le même lecteur validé de checkpoint que M11,
sépare contrôles réussis, refus et inconnus, compare les paramètres de lancement,
les plafonds administratifs et la mémoire sélectionnée, et indique le traitement
recommandé. Les permissions effectives et la disponibilité de session restent
non vérifiées : aucun précontrôle ni fournisseur n’est exécuté par une lecture.
Le responsable conserve la décision ; le diagnostic n’enregistre aucune
autorisation, ne modifie pas la consommation et ne permet pas de reprise.

La sélection d’une tentative utilise son identité exacte ou la dernière de la
tâche en stockage, sans dépendre des 200 agents récents de l’affichage. Une
tentative plus ancienne ne devient pas la dernière parce qu’elle est sélectionnée.

| Lot | Responsable / périmètre | Preuve attendue |
| --- | --- | --- |
| M12.1 | Codex, producteurs et contre-exemple | Diagnostic actuel sans distinction de frontière ; état M11 et sources préservés. |
| M12.2 | Codex, qualification commune | Frontière confirmée, effet manquant, état absent/altéré, processus actif, ancienne tentative, contrat actuel modifié ; aucun droit déduit d’un checkpoint. |
| M12.3 | Codex, CLI/API/panneau existants | Même diagnostic, sélection exacte au-delà de la liste récente, aucun effet d’écriture ni invocation ; affichage FR/EN accessible et texte hostile rendu comme texte. |
| M12.4 | Codex, vérification et RETEX | Tests ciblés, concurrence, compilation/analyse statique, recette navigateur isolée, docs FR/EN et résultats. |

Le serveur déjà ouvert est conservé. Aucun appel de modèle, processus utilisateur,
migration ou mission native n’est modifié. La restauration publique de conversation
reste explicitement non disponible ; elle devra appliquer une décision fraîche
sous les réservations et preuves requises, pas seulement afficher un avis favorable.

## Résultat

M12.1 à M12.4 sont réalisés et vérifiés localement : contre-exemple exécuté,
diagnostic commun, CLI/API et panneau FR/EN, onze tests Go distincts dont cinq
avec `-race`, analyse statique, compilation et vraie recette navigateur isolée.
Le RETEX et les incidents conservés sont dans
[RESULTATS-MEMOIRE-08.md](RESULTATS-MEMOIRE-08.md). La restauration reste non
disponible ; aucun avis du diagnostic n’accorde de droit d’exécution.
