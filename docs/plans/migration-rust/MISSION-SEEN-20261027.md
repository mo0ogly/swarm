# Raccordement mission seen — lot CLI vertical

Owner Codex ; besoin migration complète, sans mutation des travaux d’autrui.
1. Observé : Go mission seen marque session_visits puis rend Visit ; Rust expose visit mark/show et HTTP visit, mais refuse mission seen.
2. Réutiliser le producteur mark_session_visit et ses reçus ; projection historique Visit uniquement pour cette entrée, message FR/EN hors JSON.
3. Même opérateur local observé, aucun acteur/horodatage client, aucun droit ou processus ajouté. Aucun contrôle d’identité d’un autre chantier modifié.
4. CLI mission/session_visit + aide seulement ; tests/recette dédiés, docs FR/EN et notes dans le même incrément. Annonce avant édition.
5. Avant : témoin visit mark valide puis refus mission seen. Après : JSON exact/texte FR/EN, deux révisions, autres opérateurs et Work/événements/budget intacts.
6. Couvrir entrée invalide et sortie cassée après écriture avec reçu auxiliaire et relecture ; reprendre sans fournisseur réel.
7. Profil normal qualifié et binaire réellement installé requis avant annonce de livraison. Ancien journal46 et neuf47 distingués.
8. GLM : revue bornée de la projection/erreurs, contexte utile transmis seulement ; avis sans outils, tests ni autorité d’acceptation.

Fichiers réservés : rust/crates/swarm-cli/src/mission.rs et session_visit.rs,
une ligne d’aide additive lib.rs ; tests/rust_mission_seen.py, recette et docs dédiés.
Aucun changement engine/session_visit ni service HTTP prévu.

État livré219 : recette1step/17assertions PASS, intégrité VERIFIED ; profil normal
fixture-free qualifié, SHA3e2d23ae…, quatre fichiers propres égaux à l’archive.
Installation préparée exacte depuis218, contrôle vivant20 +Version14/page11,
réglages et cinq travaux conservés. Sources tierces plus récentes non installées.
Journal original et revues/preuves : [evidence/20261027-mission-seen](evidence/20261027-mission-seen/EXECUTION-JOURNAL.md).
Cinq essais refusés conservés ; aucune conformité globale ou acceptation tierce.
