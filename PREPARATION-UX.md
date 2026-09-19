# Préparer et réviser un Swarm depuis le web

[English](docs/en/PREPARATION-UX.md) · Français

Pour le parcours complet avec captures et dépannage, voir le [guide utilisateur](GUIDE-UTILISATEUR.md).

## Du besoin à une équipe

1. Depuis le cockpit, choisir **Préparer un projet**, également visible en mode simple.
2. Rédiger le besoin. Choisir un fournisseur IA et un niveau ; le modèle résolu est affiché avant l’envoi. Les appels de préparation n’ont aucun outil de modification du dépôt.
3. Relire et adopter le brief. Demander un plan, répondre aux décisions ouvertes, puis utiliser **Modifier les missions** pour éditer titre, rôle, périmètre, livrable, critères, dépendances et limites sans écrire de JSON.
4. **Vérifier le plan** contrôle le contrat, les décisions et l’absence de cycle ; cela ne prouve pas encore les livrables.
5. **Créer les missions** demande l’IA, le dossier de travail, les plafonds et le mode de validation. La création enregistre ensemble le responsable de mission, les responsabilités, les politiques de validation, le profil par défaut et les tâches. Aucun agent ne démarre à cette étape.
6. **Relire et autoriser le démarrage**, puis ouvrir le pilotage et lancer la mission avec ses réglages. La création et l’autorisation ne contournent pas les contrôles de dépendances, de budget ou d’espace occupé.

Le responsable IA est enregistré dans l’arbre des responsabilités, séparément du graphe des tâches. Il traite les retours des agents. Une tâche portant le rôle « planificateur » n’est pas à elle seule un responsable durable. Les nouvelles missions préparées configurent aussi un vérificateur IA distinct. Les missions historiques ne reçoivent pas ce rôle rétroactivement ; voir la section dédiée ci-dessous.

### Validation explicite

- **Moi — revue humaine** : les départs et les échanges peuvent être automatiques, mais chaque résultat attend une acceptation humaine. Le responsable ne peut pas accepter un livrable par son seul avis.
- **Le moteur — contrôles autorisés** : l’opérateur fournit les programmes et arguments exacts qui prouvent les conditions de départ, de validation, de livraison et chaque critère. Le moteur refuse une couverture incomplète. Les contrôles restent bornés par le contrat existant : huit contrôles au maximum par tâche et 300 secondes cumulées. Le formulaire propose 30 secondes par contrôle ; au-delà de cinq critères par tâche, regrouper les critères de façon pertinente ou utiliser la revue humaine.

Le dossier choisi est partagé par défaut : les réservations existantes peuvent sérialiser les tâches. Cette préparation ne crée pas automatiquement des copies Git parallèles.

## Réviser une mission existante

Le bouton **Réviser ce Swarm** du pilotage revient à la préparation source. Modifier le brouillon ne modifie pas les tâches en cours.

Après vérification du nouveau plan, **Comparer et appliquer la révision** affiche les tâches modifiées, ajoutées et celles à revérifier par propagation des dépendances. L’application est atomique : elle garde les anciens plans et les tentatives, réouvre les résultats touchés, met la mission en pause et verrouille les prochains départs. Une nouvelle autorisation de départ puis une reprise de mission sont nécessaires.

La transaction refuse une révision périmée, une tentative active, une décision en cours ou des branches déjà déléguées. Elle refuse aussi la suppression d’une mission ou d’un critère appartenant à une organisation : la couverture des responsabilités ne doit pas devenir fictive. Avec validation automatique, l’ajout de tâches ou le changement de critères nécessite une politique explicitement réautorisée ; ces cas ne sont pas convertis silencieusement en revue humaine. Le brouillon reste disponible en cas de refus.

## IA et connexions

Le menu **IA et connexions** est visible en mode simple et depuis la préparation. Il configure les modèles des trois niveaux pour les fournisseurs CLI déjà installés, montre leur disponibilité et propose un test explicite.

Le niveau par défaut suit la politique du fournisseur ; il ne signifie pas que Swarm choisit librement n’importe quel modèle. Le niveau exigeant reste un choix explicite. La préparation et le responsable utilisent désormais le même résolveur que les agents : modèle et effort sont transmis à l’exécutable, et la route est conservée avec l’appel ou l’organisation. Une configuration périmée est refusée au lieu de provoquer une montée en gamme silencieuse.

Cette page n’installe pas les programmes agents. Elle permet toutefois d’ajouter des connexions API et leurs clés locales avec **Ajouter une IA** ; voir la section correspondante ci-dessous. Les programmes agents restent déclarés dans `.swarm/providers.json` et authentifiés dans leur environnement.

## CLI : mêmes mutations, mêmes garanties

- `swarm prepare conversion ID` : aperçu du plan et de l’effet attendu.
- `swarm prepare create-missions ID --input requete.json` : fournir `organization` avec `provider`, `level`, `workspace`, `validation`, `max_tasks`, `max_calls` et éventuellement `controls` indexés par identifiant local de tâche.
- `swarm prepare revise-missions ID --input requete.json` : version du brouillon vérifié, `sha256`, `expected_revision`, `expected_work_revision`, `event_id` unique.
- `swarm prepare release-plan ID --input requete.json` : autoriser le plan enregistré.
- `swarm prepare send ID --input requete.json` : `level` et `model_policy_hash` facultatifs ; `auto` utilise le défaut de travail du fournisseur.

Chaque requête de mutation conserve `version: 1`, `action` et `preparation_id`. La CLI et HTTP appellent le même moteur transactionnel. Le terminal ne propose pas encore le formulaire graphique d’édition des missions ; il manipule le même document JSON versionné. Une ancienne conversion sans `organization` reste un plan manuel : le garde d’organisation interdit son lancement autonome incomplet.

## Remise du résultat au responsable

Pour une organisation sans dépôt Git géré, l’exécutant écrit `docs/<identifiant-de-tâche>.md` dans son espace de travail puis termine normalement. Il n’a pas à construire une requête JSON ni à appeler une commande de remise. Le conducteur recherche le rapport dans cet espace, vérifie la tentative courante, sa fin normale, la présence, la fraîcheur et l’absence d’ambiguïté du rapport. Le fichier doit rester dans le périmètre du projet.

Le passage à « À vérifier » et le message au responsable sont enregistrés dans la même transaction, avec l’identité de la tentative et l’empreinte du rapport. Une remise répétée ne crée pas de doublon. Un conflit de révision ou d’écriture déclenche une reprise bornée avec relecture ; il ne permet jamais de sauter les contrôles. Une tentative interrompue, un rapport ancien, vide, ambigu ou modifié avant la soumission restent à examiner.

Cette remise n’accepte pas le résultat. La politique de validation enregistrée continue de s’appliquer. Dans une nouvelle préparation, la revue IA indépendante ne remplace pas l’acceptation humaine choisie. Les missions historiques sans vérificateur conservent leur configuration. Les anciens essais interrompus restent interrompus après installation du correctif ; ils ne sont pas transformés rétroactivement en réussites.

## Vérificateur indépendant des missions préparées

Les nouvelles missions issues de la préparation configurent trois responsabilités : responsable, exécutants et vérificateur IA. Le vérificateur utilise le fournisseur et le niveau choisis pour l’équipe, dans une session distincte sans outils ni reprise de la conversation de production. Le maximum d’appels choisi s’applique séparément au responsable et au vérificateur ; chaque appel réserve aussi le budget financier commun.

Après soumission d’une tentative terminée, le conducteur lance une revue bornée à 90 secondes. Il transmet les critères et le rapport (48 Ko maximum, sans troncature). L’avis doit traiter chaque critère. Une réponse favorable doit citer un passage exact du rapport ; si une preuve externe manque, le vérificateur doit demander cette preuve. Cette revue documentaire ne prétend pas remplacer une exécution de tests ou une inspection du code. Les contrôles déterministes et l’acceptation humaine configurée restent obligatoires.

L’avis est lié à la tentative, aux critères et à l’empreinte du rapport. Il ne s’applique plus si l’un change. Les demandes de correction sont transmises au responsable et bloquent la tâche ; le responsable peut proposer une nouvelle tentative dans les limites existantes. Un incident d’appel reste enregistré et ne déclenche pas de boucle payante. Une interruption du serveur laisse un appel consommé, jamais un succès supposé.

CLI et HTTP partagent les mutations : `planning configure-reviewer WORK --input requete.json` ajoute explicitement ce rôle à une mission historique sans exécutant actif (`provider`, `level`, `max_activations`, `schema_version`, `event_id`, `expected_revision`). `planning review-step WORK` déclenche une passe, sous réserve que la mission ne soit pas en pause. `mission status WORK` expose les avis et `planning show WORK` conserve leur détail structuré.

Compatibilité : les organisations historiques restent sous leur politique humaine ou déterministe existante jusqu’à configuration explicite du vérificateur. Les nouvelles préparations portent `reviewer_required` : supprimer la configuration ne permet pas de lancer ni d’accepter. La voie des dépôts Git gérés garde son contrôleur d’intégration et ses tests ; elle n’est pas présentée comme ayant cette revue IA. Le stockage passe en version 20, avec sauvegarde préalable et refus des anciens binaires.

### Rapport avec la recherche Cursor

La section « The final system design » décrit le responsable racine, les sous-responsables récursifs, les exécutants isolés et les remises remontées au responsable. Le juge indépendant est décrit dans une version antérieure puis supprimé ; il n’est pas une exigence de leur architecture finale. La revue indépendante ci-dessus est un choix supplémentaire de Swarm, demandé pour ses garanties de livraison. Source : https://cursor.com/blog/self-driving-codebases#the-final-system-design.

### Responsabilités visibles dans le graphe et la liste

Le graphe affiche l’orchestrateur, les sous-responsables configurés et le vérificateur en plus des tâches de production, dès la création de l’organisation. Ces cartes représentent les services réellement configurés : leur présence ne signifie pas qu’un appel est en cours. La pause, l’attente et les appels sont indiqués. La liste des agents expose les mêmes responsabilités.

Les flèches en pointillés relient les responsables à leurs périmètres et les tâches au vérificateur. Les traits pleins conservent les dépendances entre tâches. Un clic sur un rôle ouvre ses décisions ou ses avis. Les rôles restent visibles lorsque les tâches sont filtrées ou repliées. Sans configuration du vérificateur, la carte annonce son absence ; le contrôleur déterministe d’un dépôt géré reste identifié séparément.

## Ajouter sa propre IA

Dans **IA et connexions → Ajouter une IA**, renseigner un identifiant local, un nom lisible, l’adresse de base et l’identifiant exact du modèle. Les préréglages Ollama et vLLM/LiteLLM proposent une adresse locale ; toute adresse compatible avec `POST /chat/completions` peut être saisie. Le modèle reste libre : aucun catalogue ancien n’est présenté comme preuve de disponibilité.

**Tester la connexion** envoie explicitement une courte question, sans contexte de mission, avec un délai maximal de 15 secondes. L’interface affiche la réponse et le délai, ou une erreur expliquée. Enregistrer est une action distincte et ne lance aucune mission. Le test confirme une réponse texte : il ne certifie pas tous les contrats structurés du planificateur. Les réponses métier restent validées par leurs contrats existants, sans extraction permissive ni succès supposé.

Une connexion enregistrée apparaît comme `api-<identifiant>` dans la préparation et les sélections de planification/revue. Elle utilise un processus séparé sans outils, une requête bornée à 60 secondes, une entrée de 1 Mo maximum et une réponse de 1 Mio maximum. Les appels conservent les bornes, réservations et validations du parcours qui les déclenche. Les jetons/coûts ne sont pas déduits d’une simple réponse de connexion.

Au passage du plan aux missions, choisir **IA du responsable et du vérificateur**, puis **Agent avec outils pour les exécutants**. Le second champ peut garder le même fournisseur pour un agent installé. Une API texte ne peut pas être choisie comme exécutant : le moteur refuse cette combinaison avant la création. Le niveau s’applique à chaque fournisseur selon sa propre politique. Les modèles API personnalisés utilisent le modèle enregistré pour les trois niveaux ; ils ne prétendent pas disposer de trois modèles différents. Les missions existantes conservent leur organisation.

Les clés sont enregistrées dans `.swarm/ai-connections.json`, fichier local en mode `0600` : accès réservé au compte du processus, sans chiffrement applicatif. Elles ne sont jamais retournées dans les listes, ni exportées avec les politiques. Une clé vide conserve l’existante ; l’action explicite Retirer la clé l’efface. Changer de destination nécessite de fournir explicitement la clé, et les redirections réseau sont refusées. Les modifications concurrentes sont refusées ; un appel préparé avant une modification de connexion ne reprend pas silencieusement la nouvelle configuration.

**Configurer et tester** permet de modifier ou désactiver une connexion. La désactivation la retire des prochains choix ; elle ne tue pas un appel déjà lancé. Aucun remplacement automatique des agents installés.

CLI : `swarm connections list` retourne les connexions sans secrets et leur empreinte. `swarm connections save --input connexion.json` utilise le même moteur que le web : `version: 1`, `expected_digest`, `connection` (`id`, `label`, `base_url`, `model`, `disabled`, éventuellement `key`) et `replace_key`. Le test interactif de connexion est disponible dans le web ; cette commande CLI ne déclenche aucun appel.

Les connexions personnalisées n’activent pas implicitement un fournisseur pour toutes les missions.
