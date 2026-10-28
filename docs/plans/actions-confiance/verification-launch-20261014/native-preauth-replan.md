# Replan — reprise avant authentification

Le premier fragment R21 constate que withTrustLock terminait une publication
initiale attribuée avant findTrustIdentity. Les six opérations métier pouvaient
ainsi publier clé et état avant leur refus. La pause publique459 conserve les
90 appels, le candidat b7179c5 et les originaux de R21 sans verdict final.

Le contre-exemple échoue avant correction. Le verrou métier ne reprend plus de
publication ; lectures et aperçus restent sans mutation. La reprise opérateur
exige le même event_id et le digest exact de la configuration initiale validée,
avant toute publication. Une autre configuration est refusée sans effet.
Les 126 cas Go race (33 principaux,93 sous-cas) passent, y compris les trois
étapes d’interruption, jetons valides/invalides et six opérations. La recette
publique observe refus sans mutation, configuration différente refusée et reprise
exacte des mêmes octets de l’état et de la clé. Les sept cas publics passent.

L’arbre correctif est 08ede6843a08ffec564a4c009da7161478f60f0b. Les 32 originaux
et rapport totalisent398126 octets : quota409600 appliqué publiquement en460,
sans suppression de preuve ; plafond94 pour quatre appels frais et90 consommés
conservés. Les contrôles natifs et la revue nouvelle restent nécessaires.
Propriétaire : Codex. Le programme reste à un lot accepté sur huit et non qualifié
pour diffusion. Reprendre les autres lots et le RETEX après acceptation réelle.
