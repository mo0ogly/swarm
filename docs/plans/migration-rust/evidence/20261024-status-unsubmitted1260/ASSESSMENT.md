# GLM1260/1261 — intégration racine
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
