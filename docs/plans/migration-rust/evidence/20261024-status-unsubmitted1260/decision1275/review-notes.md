# Revue Root1260–1275 : statut historique completed/blocked
Revue sémantique locale, pas indépendante. Comparer normal1249 et normal1273.
La commande n'accepte/soumet/réconcilie ni ne lance rien. Ancien refus1262
reproduit sur vrai worker récupéré sous bail. Changement ciblé nonmanaged planned,
blocked/completed sans livraison incomplète ; review_shape et les autres refus
(recovery, managed, scopes fermés, ancienne revue) restent en place.

## Politiques
proven_report reçoit report_clock_skew_seconds depuis le document settings réel,
validé nonnegative. Le réglage agent_report_clock_skew_seconds existe déjà dans
catalogue/defaults/Admin/API/CLI ; aucun constant2 de Go ni fallback n'est ajouté.
Le test preview/apply/readback0→2 prouve le consommateur et la reprise, sans reset
consommation. Zéro est une vraie politique stricte, pas absence/default. Les
1_000_000_000 sont conversion secondes/nanosecondes i128 ; i64seconds/date Go
bornés tiennent cette représentation sans plafond opérateur. stale/fresh sont
comptés sans limite d'entrées nouvelle. Literals status/report state, prefix Go
Livraison incomplète, raisons FR sont protocole/presentation. Option attribution
est observation manquante/supplémentaire, jamais autorité fournie par requête.
Fixtures times98/100/margin2/0/path/report/version sont entrées explicites, pas
politiques cachées ; aucun watchdog ni retry ajouté. Une seule sous-question GLM
ciblée remplace l'échec tronqué, sans hausse d'allocation ou retry automatique.

## Fonctions/docstrings
proven_report : Root docs et vraie liste triée, métadonnées régulières contenues,
taille nonzero/date inclusive, issue Attributed/Unavailable ; pas de lecture du
contenu ni auteur/qualité/authentification établis. Unknown IO/type/close restent
Err. Date illisible retourne raison Go avec cause typée sidecar conservée.
unsubmitted_pair : forme read-only étroite, pas verdict. planned_pair/supported
admettent cette forme tout en gardant review_shape/full-history/view checks.
observed_result_inputs : observe d'abord vraie revue ; seule NotApplicable et
forme étroite déclenchent attribution/marge. result_inputs_with_review reçoit
Option réel ; l'adapter test None ne peut inventer de rapport. Submitted/accepted
et review Presented gardent leurs producteurs et n'utilisent pas l'attribution.
Les Rustdoc/contrats concernés expliquent la nouvelle forme et limites. Deux
nouveaux tests filesystem vérifient réel témoin, égalité de marge/zero, vieux,
vide, ambigu, invalidstart/sidecar, chemin externe refusé puis restauré. Deux
attentes historiques changées avec témoin blocked/completed autorisé, mais
incomplete-delivery/current-review invalide/queued/unknown toujours refusés.
La recette status_read compare vrais DTO CLI/HTTP et état/marqueurs exacts avant/
après. Public mission stop met la fixture au repos, pas seed SQL ou contournement
de refus. FR/EN/REPLAYS/version notes mis à jour sans prétendre installation.

## Erreurs
Pas de skip metadata contrairement au pseudo-code tronqué GLM1260 rejeté.
Missing docs seul ENOENT produit liste vide selon reader préexistant ; stat,
contenance, race et close conservent causevia Gate et Root. Date parse illisible
conserve le parse original dans causes ; ne devient pas une date par défaut.
Absence/stale/ambigu sont observation Unavailable avec raisons Go, pas succès
vide ni prétendue acceptation. Lire la réponse GLM «refuse» selon ce vrai type,
pas comme nouveau Err pour une absence légitime. Unknown date d'un vrai Agent
peut aussi être refusée plus tôt par ses lecteurs : le helper seul ne prouve
pas que toutes ses entrées invalides sont publiquement atteignables.
Les sidecars sont transmis à l'agrégateur historique existant et suivent close/
encode/output. Aucun nouveau wrapper/flush ou contrat cleanup n'est inventé.
Le report reader conserve ses propres garanties séquentielles/limites ABA ;
ce changement n'affirme pas audit intégral des helpers inchangés. Un rapport
attribuable n'est pas forcément lisible en contenu : Go fait stat, le Rust ici
aussi, aucun hash/authenticité ou soumission ne découle des métadonnées.

## Preuves/limites
GLM1260/1261 :1incomplet/1complet,46135tokens déclarés,0retry automatique ;
API glm-5.2 comme nom déclaré, GLM5.3 selon utilisateur seulement. Skill HTTP
sans outils ni writer. Root relit le caller Go manquant dans la question ciblée.
RED1262 réel500 sur profil1249.1269 : absence/frais/ambigu/restauration réussis,
puis mission-status-supervision-changed d'un heartbeat concurrent. Preuve gardée ;
aucun retry/guard supprimé.1271 ferme publiquement le consentement avant serveur
neuf et montre dix lectures exactes et politique persistée. La disponibilité
systématique sous writer concurrent n'est pas établie ; refus de snapshot reste.
1268 :103status PASS/2FAIL attentes anciennes, conservées ; assertions alignées
1272, pas affaiblissement des gardes invalides. Qualification fraîche requise.
Les résultats1274/recettes finales sont à lire, ne pas déduire PASS d'une build.
Aucun profile all-features/fixture ne qualifie le binaire produit. Source review
ne prouve ni revue indépendante, parité Go exécutée sur tout corpus, ni livraison
installée. Repository settlement/multi-tâches/G1–G12/R6/R7/RETEX restent ouverts.
