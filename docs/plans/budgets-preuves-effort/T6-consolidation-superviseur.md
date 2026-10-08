# T6 — consolidation manuelle, validation non obtenue

Les quatre corrections ont des recettes isolées réussies et T1–T5 sont acceptées par le moteur. La mission reste 5/6 : les deux producteurs T6 ont été interrompus et aucun avis indépendant T6 ne valide cette consolidation.

## Provenance et preuves

Mission `w-01567e073c1ed2f3d4c71c9e`, état public r135 le 4 octobre 2026. Consolidation réalisée par le superviseur Codex, distincte du brouillon du producteur conservé dans `docs/plan-01567e073c-T6.md`.

| Exigence | Preuve | Résultat et limite |
|---|---|---|
| Recette des quatre QW | `T6-recipe-current-2.log`, commande `python3 tests/qw_all_acceptance.py`, exit 0, 6735 octets | PASS isolé : opérations CLI/UI réelles, FR/EN et deux thèmes. QW6 utilise les fonctions de rendu avec données moteur isolées ; QW7 utilise le cockpit complet avec télémétrie synthétique ; fournisseur de QW8 simulé. Pas de preuve d'autonomie de bout en bout. |
| Mesures et RETEX | `RETEX-supervision.md`, sorties CLI réelles `T6-installed-cli-fr.txt` et `T6-installed-cli-en.txt` | PARTIAL : mesures publiques disponibles ; coûts manquants et interventions hors Swarm exclus du total rapporté. |
| Livraison vérifiée | `T5-candidate.json`, `T5-go-final.log`, `T5-targeted-final.log`, `T5-npm-final.log`, `T5-vet-final.log`, `T5-remise.md` | Candidat de développement HEAD cc3069dc7bb61b90168d21f945cb2eb5e27578ed, dirty. Go complet PASS 408,945 s avant la dernière traduction ; contrôles ciblés/frontend/vet après. Aucun commit, publication ni release. |

Les précédents logs de recette et les tentatives interrompues sont conservés. Le premier échec du regroupement provenait d'une synchronisation du test QW7 : le bouton n'était pas encore rendu. L'attente du bouton a corrigé le test, pas le moteur.

## Consommation observée

Snapshot public CLI du 4 octobre à 10:13:21 UTC, pris avant la fin de la seconde tentative T6 : responsable 7 activations, reviewer 16 réservations/appels enregistrés, producteurs T1–T6 respectivement 9/65/23/62/67/30 appels d'outils observés. Ce sont des unités différentes. Le total rapporté n'inclut pas tous les coûts, ni les interventions de cette conversation.

L'ancien RETEX `docs/plans/clear-launch-recovery/RETEX-CLOTURE.md` contient le bilan précédent : 45 revues et 154 contrôles. Les missions diffèrent ; cette différence n'établit pas un gain causal de performance.

Snapshot final complémentaire `T6-effort-final-observed.json`, 10:20:44 UTC : 26 contrôles, 5 reprises producteurs ; T6 total30 outils et somme des durées enregistrées486591ms, après confirmation de fin de sa seconde tentative. La somme des durées n'est pas le temps écoulé de la mission.

## Arrêts T6 et récupération

- Claude : 17 outils, arrêt après quota 429, échéance conservée 12:20 UTC.
- Codex : 13 outils, processus interrompu à 10:13:47 UTC après 180 secondes sans sortie visible, avant son timeout total de 600 secondes. Aucun résultat final. La trace ne prouve pas ce que le modèle faisait intérieurement.
- Le moteur conserve les deux tentatives. `submit-recovered-result` et `requalify` sont réservés aux résultats Git gérés ; ils ne permettent pas de soumettre cette consolidation pour cette mission en dossier partagé. Aucun état terminé, avis ou résultat attribuable n'est inventé.

## Enseignements et prochain projet

1. Comparer version installée, version serveur et candidat : le CLI ancien masquait `observed_at` pourtant présent dans le serveur récent.
2. Vérifier l'observabilité des preuves avant l'appel payant : des contrôles PASS trop succincts puis une capture tronquée ont coûté des revues supplémentaires.
3. Séparer silence visible et liveness vérifiable ; conserver une borne finie et ne jamais simuler une progression.
4. Exposer quota, échéance et relais configurable sans effacer le coût ni les tentatives consommées.

Les deux premiers sont des quick wins. Les deux suivants touchent le moteur et demandent des tests d'arrêt, de concurrence et de conservation des budgets. La nouvelle préparation ne remplace pas T6 et ne clôture pas cette mission.

Préparation créée par l'interface : `prep-8c84a9cf6e6351c100106270`, « Swarm — supervision fiable et preuves économes ». Brief adopté après correction manuelle des rôles et des bornes de silence ; demande de plan Codex standard gpt-5.6-sol lancée à 10:22:45 UTC. Cela prouve un démarrage de préparation, pas encore un démarrage des tâches.
