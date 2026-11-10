# Préparation du collectif réel — amendement du 10 octobre 2026

La conception `conception-collectif.md` reste historique. Les points suivants sont vérifiés dans le moteur main `3faa8a9`, sans appel de modèle ni modification du cockpit vivant.

## Ce qui existe déjà

- Trois conditions réelles sont implémentées dans le banc : B0r (agent payeur), W (workflow fixe) et S (moteur Swarm). Seul le préparateur est réel dans S ; la coordination et la revue restent scriptées.
- Le lot du 6 octobre contient 80 essais : B0r 20 (18 mesurables, 2 invalides), W 30, S 30. Le rejeu F8 du 7 octobre contient cinq W et cinq S. Ces dix observations sont distinctes du lot initial : ne pas additionner automatiquement les anciennes F8 invalides et les nouveaux essais.
- Les trois faux succès observés en B0r ne sont pas des paiements inexacts : publier ces mesures séparément.
- `internal/engine/assist_provider.go` construit les adaptateurs de planification et de revue. Claude est lancé sans outils ni MCP et en mode safe ; Codex sans outils et avec configuration/règles ignorées. Ces faits de code ne démontrent pas leur isolement en exécution. Les connexions API empruntent un chemin distinct qui doit avoir sa propre recette.
- La consommation des responsables est enregistrée par le moteur (`savePlanningUsage`) et exposée dans `mission status` / `spending`. Les champs sans consommation ou coût déclarés restent inconnus.
- Le délai de revue se configure par l'opération publique `planning review-timeout`. Il faut fixer le paramètre avant le pilote et enregistrer sa valeur ; ne pas recycler les 90 secondes historiques comme garantie générale de disponibilité.

## Travaux avant les deux essais pilotes

1. Implémenter un mode distinct S-collectif, sans remplacer S-réel ni ses journaux. Le responsable crée les tâches ; le banc ne doit pas soumettre une décision initiale puis qualifier cela d'autonomie.
2. Préparer une liaison atomique des profils : seul le rôle règlement porte le jeton. Vérifier dans l'API actuelle comment le profil est lié avant dispatch. Une lecture puis écriture concurrente du banc n'est pas une garantie.
3. Refuser comme résultat de coordination toute double attribution de req-2 et toute tâche mêlant req-1/req-2. Ne pas corriger silencieusement la planification du modèle.
4. Préautoriser de vrais contrôles métier et lier entrées/candidat/règlement ; documenter la frontière entre refus du moteur et refus du script de paiement.
5. Tester les adaptateurs, formats structurés, absence d'outils/MCP, journalisation et quota signalé sur des racines isolées. Une commande présente ne prouve pas la disponibilité de l'abonnement.
6. Figer budgets, délais, nombre maximal de tâches, sorties brutes privées, politique de coût inconnu et arrêt causal. Aucun relèvement automatique lors d'un timeout.
7. Publier le protocole garde/libre, les injections F10/F11, les mesures de coordination et la sélection du modèle avant appels réels.
8. Exécuter deux pilotes sans faute ; établir pour chaque rôle appels, jetons, coût déclaré ou inconnu, durée, décisions refusées, couverture et paiement. Dimensionner ensuite le lot exploratoire de 90 essais à partir de ces observations.

## Séparation des résultats

Le rejeu F4e en cours est une recette de moteur avec doubles scriptés. Il ne constitue aucune observation S-collectif. L'injection est connue ; la cause de chaque blocage doit être établie par sa trace. Les cinq délais historiques ne sont ni des timeouts LLM démontrés ni des erreurs SQLite démontrées.
