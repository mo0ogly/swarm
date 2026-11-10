# Méthode — observations1260–1279
- Paquet GLM initial trop large :38729tokens input, sortie tronquée à4000.
  Audit ciblé avec callers utiles et sans tests hors question :2795input/611output,
  complet. Préparer la question/contrats avant les extraits, ne pas transmettre
  tout le module de tests pour une sélection de rapport. Ne pas omettre le caller
  prouvant Root versus cwd : contexte réduit doit rester complet pour la question.
- Statut sous conducteur actif : cohérence supervision peut refuser si heartbeat
  change ; une série de tests de sélection doit mettre au repos publiquement sa
  fixture, sans contourner le refus. Garder test concurrence séparé de l'attribution.
-105tests unitaires exposent2 anciennes frontières délibérément non portées.
  Les remplacer par positifs de la nouvelle forme et négatifs conservés, jamais
  supprimer une assertion sans son nouveau contrat.
-Clippy nonminimal_bool détecté après recettes PASS a imposé une requalification
  malgré équivalence métier. Exécuter à l'avenir le check statique ciblé avant
  profil de livraison/recette consolidée ;1278 commence par Clippy.
Cette note/plan est transmise au prochain contexte par le suivi existant, pas
aux agents déjà lancés ni comme garantie de gain/token universel. Les preuves
originales sont gardées et chaque appel/build conserve son résultat réel.
