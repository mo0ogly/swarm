# Auto-revue IA3a — non indépendante

Codex dans le contexte producteur, base `baseline.json`. Retour au producteur
avec demande de correction, sans acceptation globale.

- Les trois refus absents sont reproduits dans `plan-before.jsonl`. Le premier
  raccord passe 36 noms mais ne couvre pas le retour à un contenu ancien.
- La génération calculée seulement lors d'une publication réactive un octroi
  ancien après modification puis restitution : reproduit dans
  `generation-before.jsonl`. Exiger une génération à chaque écriture du contrat.
- Les RawMessage gardent l'ordre des clés imbriquées : le même fichier JSON
  autrement mis en forme produit un digest différent. Reproduit dans la même
  exécution ; canoniser tous les objets avec `UseNumber`, sans float64.
- La première recette d'attente panique en déréférençant `commandFailure(nil)` :
  `generation-first-after.jsonl`, sources préservées. Corriger le pilote, puis
  vérifier les tests nommés réellement exécutés, pas seulement la compilation.

La restauration physique de toute la base vers un plan ancien exige un témoin
qui couvre le watermark du plan. Une tête d'identité inchangée ne constate pas
un changement de plan non publié. La source privée et les fixtures ne ferment
ni cette récupération indépendante ni l'admission publique.
