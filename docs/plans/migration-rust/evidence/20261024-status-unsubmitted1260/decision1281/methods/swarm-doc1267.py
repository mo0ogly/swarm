from pathlib import Path
import json,sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260')
assessment='''# GLM1260/1261 — intégration racine
Deux appels directs autorisés, zéro retry automatique.1260 :38729input/4000output,
42729total déclarés, fin length (2923raisonnement/1077texte), non complet.
1261 :2795input/611output,3406total, stop complet (37raisonnement/574texte).
Total déclaré46135tokens, pas facture/prix ni gain Codex mesuré. API demandé et
rapporté glm-5.2 ; backendGLM5.3 selon utilisateur, pas attesté par le nom API.
Aucun outil/dépôt/writer ou acceptation confié au modèle.

Retenu : missing planned_pair + producer attribution + admission terminale ;
Go final completed appelle provenReport(t.ID,a.Started) sur s.root, pas cwd.
La réponse ciblée n'avait pas le caller ; cette limite est correcte et complétée
par lecture Root du commit553593a2, result_presentation.go198–214 archivé en1260.
Le prédicat actuel comporte un troisième refus dans la boucle terminale supported,
également raccordé. Les revues courantes gardent leur producteur séparé.

Rejeté : pseudo-code tronqué et skip des erreurs metadata dans1260. Garder les
refus/causes typed de task_reports et artifact_metadata. Ambigu/stale/absent sont
Unavailable avec raison, pas Err ; la réponse ciblée appelle cela «refuse», terme
réinterprété selon le vrai consommateur ReportAttribution et le Go. NOENT après
sélection est une course/erreur conservée, pas un skip. IncompleteDelivery reste
hors de cette forme sans inventer une observation vide.

Code Root1263/1264 ; recette publique1266 teste CLI et HTTP après vraie recovery,
rapport Root absent/frais/ambigu/ancien/vide, marge0→2 par preview/apply sauvegardé,
chemin hors Root refusé puis restauration, absence de mutation/acceptation/provider.
RED1262 sur ancien binaire7292e5 renvoie500 attendu avant le correctif.
Résultats de compilation/tests/recette restent leurs preuves distinctes.
'''
atomic(b/'ASSESSMENT.md',assessment)
for name,para in [('docs/architecture/NATIVE-WORKER-RECOVERY.md','''\n## Statut historique du résultat non soumis (candidat1263)\nLe couple planifié non géré tâche blocked/agent completed peut maintenant être\nprésenté sans revue courante, hors livraison incomplète. Le statut historique\nCLI/HTTP observe les rapports dans docs/ à la racine du projet, comme le Go ;\nil ne consulte pas le workspace de l'agent à la place. Une unique candidature\nnon vide assez récente est attribuable, sans authentifier auteur ni qualité.\nAbsence, fichiers anciens ou plusieurs candidats restent des résultats non\ndémontrés avec raison explicite ; erreurs chemin/IO/fermeture restent des refus.\nLa marge est agent_report_clock_skew_seconds, validée/persistée et éditable dans\nAdmin/API/CLI existants ; zéro exige une date au départ ou après, une valeur plus\ngrande peut admettre un rapport plus ancien. Le statut ne soumet pas le rapport,\nne modifie pas la tâche et n'autorise aucun lancement. Les revues, livraisons\nincomplètes et dépôts gérés gardent leurs contrats/refus propres. Ce port remplace\nla limite1256 completed/blocked sans revue décrite plus haut ; qualification en\ncours, aucune installation. Les preuves historiques1256 ne sont pas réécrites.\n'''),('docs/en/NATIVE-WORKER-RECOVERY.md','''\n## Historical status of an unsubmitted result (candidate1263)\nA nonmanaged planned blocked task with a completed Agent can now be presented\nwithout a current review, excluding incomplete delivery. Historical CLI/HTTP\nstatus observes Root docs reports, matching Go; it does not substitute the Agent\nworkspace. A single nonempty sufficiently recent candidate is attributable, with\nno authorship or quality proof. Missing/old/ambiguous candidates remain unproven\nresults with explicit reasons; path/IO/close failures remain errors.\nThe margin uses persisted, validated agent_report_clock_skew_seconds, editable\nthrough existing Admin/API/CLI settings. Zero requires mtime at/after start;\na larger value may admit an older report. Status never submits a report, changes\na task or grants execution. Reviews, incomplete deliveries and repositories\nretain their own contracts/refusals. This port supersedes the1256 completed/blocked\nlimitation above; qualification is pending and nothing is installed. Historical\n1256 evidence is preserved.\n''')]:
 p=Path(name);atomic(p,p.read_text()+para)
p=Path('docs/plans/migration-rust/REPLAYS.md');atomic(p,p.read_text()+'''\n1266 : la recette recovery exige désormais status CLI/HTTP lisible après reprise.\nElle remplace l'assertion de refus historique1256 par dix lectures de résultat\nabsent/frais/ambigu/ancien/vide/restauré, marge persistée0→2 et refus hors Root.\nChaque lecture compare Agent/Work/réservation/FIFO/financement/provider avant/après.\nLe binaire7292e5 reste le témoin RED1262 ; ne pas rejouer ce manifeste courant en\nattendant PASS sur cet ancien profil. Sources/profil actuels requis ; aucun watchdog.\n''')
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1267 : GLM1260 tronqué, conservé ; question réduite1261 complète, pas budget\nrelevé.2appels/46135tokens déclarés/0retry. Réponses arbitrées dans ASSESSMENT.md.\nRED1262 réel500 sur7292e5 ; port1263 et deux tests filesystem1264, recette1266.\nSource persistée→validation→Admin/API/CLI existants→consommateur status ; preuve\nattendue marge0→2 sauvegardée et effet sans soumission. Profil1265 en compilation.\n''')
