# M22 — cachets de contexte et repères réutilisables

17 octobre 2026. Demande : icônes démonstratives pour mémoire/protection/identité,
descriptions des états, réemploi possible par la session identité ; direction
visuelle précisée comme un cachet de cire au bas d'un parchemin. Codex seul,
APEX borné et `replan` sur prérequis manquant.

Choix : modifier tout de suite les écrans et ressources Go embarquées pendant
IA3b, ou livrer d'abord le composant partagé et son catalogue isolé. La seconde
voie est retenue pour préserver les compilations et fichiers actifs de la session
identité. Les sources sont sous `frontend/signals/` ; le build explicite produit
un répertoire neuf hors checkout. Aucun asset `web/` n'est modifié.

| Lot | Propriétaire | Critère |
| --- | --- | --- |
| M22.1 | Codex mémoire | Vocabulaire de formes et de sens : 29 repères, FR/EN, type/version/identité/protection distincts ; fait conceptuel marqué non persistant. |
| M22.2 | Codex mémoire | API locale `SwarmSignals`, badges accessibles et cachet irrégulier ; inconnus par défaut, aucun octroi ou signature par affichage. |
| M22.3 | Codex mémoire | Catalogue interactif, changement d'états, FR/EN/thèmes, cinq largeurs, contraste, texte non fiable, clavier/focus et diagnostics. |
| M22.4 | Codex mémoire | Documentation partagée FR/EN, exemple pour la session identité, captures inspectées et RETEX. Intégration aux écrans réels explicitement restante. |

Périmètre : composants, build/recette isolés et documentation. Exclut identité
réelle, autorisation moteur, migration, activation Admin, appels modèles,
lancement Swarm natif, déploiement et changement de serveur. L'ajout visuel ne
ferme pas M20. Les preuves et incidents sont dans `evidence/m22-20261017/`.

M22.1–M22.4 sont réalisés et vérifiés dans ce périmètre isolé : cinq groupes de
contrôle passent sur les 29 repères et vingt configurations de rendu. Les
contrastes minimaux sont 5,75:1 et 8,02:1. La régression des clés JavaScript
héritées est reproduite puis corrigée ; le résultat courant et les captures
sont dans `browser-verified/`. [Rapport et RETEX](RESULTATS-MEMOIRE-18.md).
