# MOD-01 — Choisir et voir le modèle de chaque tâche

État : choix persistant par tâche de production implémenté et recetté (tests Go et navigateur, sans appel IA payant).
Voir ../TASK-MODELS.md pour la portée livrée. Restent l’édition par responsable
et les propositions automatiques du planificateur. Demande utilisateur du 23 septembre 2026.
Rattachement : mission w-f923632399eea4d404bc94ac, évolution après qualification E6.
Cette fiche ne constitue ni une tâche adoptée dans le graphe ni une autorisation
supplémentaire d’appels IA. Ne pas retarder la preuve E6 pour cette évolution.

## Besoin
Utiliser Sonnet pour certaines tâches et Opus pour d’autres. Configurer le choix
avant le lancement autonome, sans devoir intervenir à chaque tentative.

## Livrables
- Configuration persistante par tâche dans le web et le CLI : fournisseur,
  niveau, modèle résolu et effort lorsqu’il est supporté.
- Priorité explicite : choix de tâche, sinon profil de mission, sinon défaut
  du fournisseur. Afficher l’origine de chaque valeur héritée.
- Carte du graphe et détails : distinguer modèle prévu et modèle enregistré
  pour chaque tentative. Si la version exacte n’est pas rapportée, afficher
  l’alias sans inventer de numéro de version.
- Planificateur, sous-planificateurs et vérificateur : réglages distincts et
  visibles ; modifier une tâche de production ne doit pas les modifier.
- Proposition de modèle par le planificateur possible, adoption uniquement
  dans les limites autorisées. Aucune montée vers Opus ou substitution silencieuse.
- API moteur commune, documentation et aides FR/EN, exemples CLI et RETEX.

## Règles moteur
Prévisualisation puis confirmation, révision et politique fournisseur vérifiées.
Refuser les modèles/efforts non supportés. Conserver les preuves et le choix des
anciennes tentatives ; ne pas modifier une exécution active. Avant un futur départ,
une politique devenue différente doit être explicitement réconciliée, sans changement
silencieux. Ne pas inventer un prix et ne pas assimiler une modification du modèle
à une autorisation de tentatives ou à un résultat accepté.

## Recette obligatoire
1. Deux tâches de la même mission : Sonnet et Opus ; vérifier les arguments
   réellement transmis au connecteur par des doubles, puis les valeurs persistées.
2. Vérifier héritage et surcharge, lancement manuel et autonome, reprise,
   redémarrage, modification concurrente et politique fournisseur périmée.
3. Refuser modèle inconnu, effort incompatible et modification d’un agent actif.
4. Vérifier indépendance des réglages du planificateur et du vérificateur.
5. Parité CLI/API/web ; clavier et thèmes clair/sombre ; français/anglais.
6. Montrer les choix sur les cartes sans perdre les flèches ni les couleurs de rôle.
7. Aucun appel payant de recette sans autorisation distincte et bornée.

## Adoption dans le plan
Le plan hiérarchique exige une décision versionnée du responsable pour créer une
carte. Son quota est actuellement épuisé. Conserver cette demande dans le backlog
jusqu’à adoption autorisée, sans réinitialiser ses compteurs ni contourner ce contrôle.
