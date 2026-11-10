# Actions de confiance dans Swarm

## Objectif

Un opérateur définit un mandat, examine une demande, autorise ou refuse l’action et retrouve son résultat. Un agent ne peut pas étendre ses droits ni présenter son propre texte comme une autorisation officielle.

## Socle existant

La console des journaux est en lecture seule. Les réponses originales de revue sont conservées sans écrasement et liées à une empreinte. Les journaux de revue sont vérifiés contre une empreinte ancrée dans l’état durable. Les journaux d’activité ordinaires sont distincts et soumis à rétention. Le programme réutilise ces mécanismes après vérification de leurs producteurs, lecteurs, droits et limites ; une lecture seule dans l’interface ne démontre pas une interdiction d’écriture au niveau du stockage.

## Pilote initial

Pilote de facturation simulée avec grand livre déterministe et service factice en racine isolée. Aucun accès bancaire ou paiement réel. Le connecteur pilote constitue le périmètre de protection à démontrer ; les exécutables et chemins d’accès non médiés sont inventoriés et leurs limites explicites.

## Exigences

R01 Identité — Le point d’entrée authentifie le demandeur. Un champ actor fourni par le demandeur ne constitue pas une identité authentifiée.

R02 Mandat — Sujet, délégation, actions, ressources, contraintes, version et expiration sont vérifiés. Modifier un élément couvert invalide les approbations incompatibles. Les droits proviennent des règles et autorisations, sans compensation par un score agrégé.

R03 Effet — Le service vérifie l’autorisation et les paramètres exacts au moment d’engager l’effet. Mandat absent, expiré, révoqué, mauvais sujet ou ressource hors périmètre : refus sans mutation métier.

R04 Révocation — L’ordre entre révocation et engagement concurrent est défini et testé. Après prise en compte de la révocation, aucun nouvel engagement n’est autorisé. Les effets déjà engagés restent conservés et observables.

R05 Approbation — L’accord porte sur l’empreinte exacte de l’action : montant, devise, bénéficiaire et ressource. Le moteur applique la séparation des responsabilités ; le demandeur ne fabrique pas sa propre approbation.

R06 Reprise — Une clé d’idempotence est liée au contenu. Même clé et contenu différent : conflit sans mutation. Même clé et contenu identique : retour de l’état connu. Réponse perdue ou arrêt après engagement : état inconnu puis réconciliation avec le service avant toute nouvelle tentative.

R07 Preuves — Relier intention, identité, mandat, politique, décision, engagement et résultat observé en réutilisant les reçus, originaux et vérifications existants. Les refus et états inconnus restent conservés. Ajouter seulement les garanties manquantes pour l’export signé du pilote. La vérification utilise une clé et une ancre acquises séparément ; elle détecte altérations, réordonnancement et omissions par rapport à l’ancre. Aucun secret dans l’export.

R08 Configuration — Toute nouvelle règle, durée, reprise ou rétention possède une source de vérité persistée, un aperçu sans mutation, une application protégée des révisions anciennes et une édition Admin. API, CLI et Go partagent validation et valeurs effectives. Unités, héritage, défauts et sens de zéro sont définis avant implémentation. Aucune seconde limite cachée.

R09 Intégrité Biba — Définir la variante du modèle Biba retenue, les niveaux d’intégrité des sujets et objets et les règles de lecture, écriture et invocation. Les entrées externes et sorties d’agents conservent leur provenance et leur niveau ; un agent ne peut pas s’attribuer un niveau supérieur ou réécrire politiques, clés et preuves officielles. Toute admission dans un état protégé passe par une procédure de validation explicitement autorisée et attribuée. Vérifier les flux interdits, les admissions permises, les contournements et la persistance via API, CLI et stockage. La seule lecture du journal dans l’interface ne démontre pas ces garanties.

R10 Interface — Présenter demandes, approbations, refus, expiration, révocation, engagements, états inconnus et réconciliations. Motifs bloquants et prochaine action visibles ; statuts exprimés par du texte. Vérifier FR/EN, État/sombre, clavier, Escape/focus, mobile, chargement/vide/erreur/succès et erreurs réseau. L’architecture conserve la livraison séparée des UX Projet et Processus métier.

R11 Recette — Les opérations publiques vérifient les changements réels du grand livre, les refus sans mutation, la persistance après redémarrage et la concurrence. Les contrôles et la revue indépendante portent sur le même candidat. Séparer simulation, fournisseur réel, intervention opérateur et acceptation ; aucune autoacceptation.

R12 Livraison — Fournir documentation courante FR/EN, recette reproductible, matrice des exigences et preuves puis RETEX final. Distinguer contrôles locaux, reçus moteur et revue. Les travaux non vérifiés restent ouverts.

## Parcours

Définir les règles et le mandat → présenter une demande → vérifier l’autorité → obtenir l’approbation requise → engager l’action autorisée → observer ou réconcilier le résultat → consulter et exporter les preuves.

## Architecture et responsabilités

Le moteur contrôle droits, versions et transitions. Le service métier applique ses invariants dans la transaction qui engage l’effet. Le connecteur porte l’idempotence et la réconciliation. Les agents demandent une action dans leur périmètre ; l’opérateur autorise les règles et la revue indépendante évalue le candidat.

Le cadrage compare les contrôles de départ seuls et un passage contrôlé par action. Il définit le passage obligatoire du connecteur pilote, la protection des secrets hors des copies d’agents et les restrictions des accès directs. Une configuration dont l’isolation n’est pas démontrée est refusée. Les contrôles de conformité du résultat et les contrôles préalables à l’effet restent complémentaires.

## Observation système avec eBPF

Reprendre l’étude existante dans docs/architecture/CURSOR-ENGINE-EVOLUTION-20261010.md, section « Extension demandée : observabilité eBPF ». Elle définit une option étudiée, sans collecteur intégré ni activation privilégiée réalisée.

Intégrer un collecteur Linux optionnel, séparé du moteur, pour les événements de processus, accès disque et réseau. Relier PID, descendants et cgroups disponibles aux identités mission/tâche/tentative. Les décisions métier, tokens et verdicts restent attribués aux événements du moteur.

L’incrément d’observation précisera les sondes, capacités nécessaires, disponibilité du noyau, événements perdus, confidentialité, nettoyage et surcoût mesuré. Activation, sélection des sondes, échantillonnage et rétention seront persistés et administrables. Le moteur reste utilisable sans collecteur ; son absence et ses pertes de mesures restent visibles.

L’observation complète les traces du moteur. Le blocage d’une écriture ou d’une action sensible exige un mécanisme de contrôle explicitement configuré et vérifié ; la collecte ne constitue ni une autorisation ni une garantie d’inaltérabilité des journaux.

## Exigences des extensions

R13 Identités de l’application — Authentifier personnes et services via le protocole choisi, contrôler les attributs de sécurité et gérer délégation, cycle de vie, révocation et séparation des responsabilités. Étendre ce contrôle aux instances d’agents : identité non autodéclarée, création et délégation limitées au plan adopté et aux droits courants. Une création hors plan reste bloquée avec explication jusqu’à décision d’une personne authentifiée et habilitée, distincte du demandeur ; aucun agent ou service ne fabrique cet accord. Vérifier contenu exact, rejeu, concurrence, révocation avant départ, protection du plan de contrôle et toutes les voies CLI/API/reprise/dispatch. Les droits de lecture, résumé et transmission du cadrage dépendent du propriétaire et de l’identité attestée. Les 18 scénarios et leur réalisation restant ouverte figurent dans [Identité des agents](identites-agents-replan.md) ; les preuves R13 historiques ne qualifient pas cette extension.

R14 Données — Appliquer classification, destinations et secrets par action aux flux effectivement contrôlés. Refuser une action sensible lorsque la protection de son chemin n’est pas démontrée. Vérifier exfiltration et contournement en simulation isolée.

R15 Processus — Définitions versionnées, dossiers, validations et exceptions avec historique ; UX métier livrable séparément. Les modèles respectent les opérations publiques réellement supportées.

R16 Preuves durables — Rotation et révocation des clés, ancrage indépendant, rétention et export ; détecter les altérations et omissions sous ancre. Conserver l’attribution des originaux et protéger les secrets.

R17 Simulation — Rejeu et scénarios adversariaux isolés sans effet de production ; politiques, résultats et limites visibles et persistés.

R18 Connecteurs — Catalogue extensible et second connecteur sensible simulé avec invariants du service, autorisation, isolation, idempotence et réconciliation vérifiés.

R19 Référentiels — Établir l’applicabilité, les contrôles, preuves et écarts pour le contexte cible ; aucune certification ou conformité globale déduite d’un test local.

R20 eBPF — Intégrer le collecteur Linux optionnel, corréler les événements au périmètre du projet, gérer permissions, pertes et nettoyage, rendre les paramètres persistés et administrables, et mesurer collecte réelle et surcoût. Une autorité système manquante reste une preuve bloquée avec condition de reprise ; les autres travaux continuent.

## Départ et acceptation

Le lancement couvre les quatorze stories, organisées en huit lots dépendants. Configurer l’équipe Codex, l’isolation Git et les contrôles exécutables via les opérations publiques. Les agents peuvent décomposer les lots dans les droits et budgets attribués, sans créer une acceptation implicite. La recette et le RETEX couvrent l’ensemble ; aucun paiement réel, publication ou déploiement externe n’est autorisé.

## Périmètre détaillé des lots

T1 Définir l’architecture complète — Inspecter producteurs, API/CLI, stockage et UX ; reprendre l’étude eBPF et le système visuel. Livrer docs/architecture/ACTIONS-CONFIANCE.md et docs/en/ACTIONS-CONFIANCE.md, avec architecture et interfaces précises de tous les lots.

T2 Livrer le pilote avec Biba — Moteur et connecteur de facturation factice, mandat/approbation, idempotence et réconciliation ; Admin et parcours opérateur FR/EN. Réutiliser les journaux, reçus et ancrages existants.

T3 Intégrer les identités d’entreprise — Un protocole OIDC explicitement choisi, personnes et services, droits délégués et cycle de vie ; API/CLI/Admin et parcours de connexion. Issuer factice isolé pour les tests ; aucun compte tiers créé.

T4 Maîtriser les données et secrets — Classification et politiques de destinations, fournisseurs et outils ; secrets par action et frontières d’exécution, avec refus des configurations dont les garanties ne sont pas démontrées.

T5 Livrer les processus et dossiers métier — Définitions versionnées, dossiers, validations et exceptions ; UX Processus métier livrable séparément avec moteur commun. Utiliser .claude/skills/swarm-model-design/SKILL.md pour les modèles de processus.

T6 Renforcer les preuves et éprouver les contrôles — Clés et ancrage externe, rétention, exports ; laboratoire de simulation/adversarial isolé ; un second connecteur sensible simulé et matrice des référentiels applicables.

T7 Intégrer le collecteur eBPF — Collecteur Linux optionnel séparé du moteur, transport local, attribution des événements et Admin. Sondes limitées aux processus du projet et à leurs descendants ; aucune observation étrangère au périmètre.

T8 Qualifier l’ensemble et livrer le RETEX — Recette indépendante du candidat complet, contrôles cumulés, documentation FR/EN, matrice R01–R20 et RETEX. Aucun changement du candidat pendant la revue.
