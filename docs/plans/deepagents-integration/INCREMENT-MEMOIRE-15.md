# M19 — sélection extractive admise pour un lancement explicite

16 octobre 2026, reprise « ok next ». Codex seul, `replan` et APEX.
MEM-U08/U13, MEM-C16/C17 ; résultats et appels M6–M18 conservés.

## Contrats observés et choix

M18 exporte un instantané complet destiné à l'opérateur. `prepareLaunch`
construit le contexte de tâche, puis le lie à `ContextManifest` ; la transaction
conserve demande, événement, agent et réservation. `supervise` ajoute la reprise
et enregistre le prompt complet avant d'ouvrir le fournisseur. La sélection de
dialogues existante joint leurs textes complets. Ces voies sont les points de
contrôle, sans nouveau stockage de résumés.

Importer un reçu Python comme autorité imposerait un second validateur de son
protocole progressif, avec risque de fuite de l'instantané opérateur. Le choix
est une sélection déclarative : empreinte M18, références, frontière, annotations
et motif explicite. Le moteur relit les originaux et reconstruit les citations ;
aucun texte libre de synthèse n'est accepté. Un outil local peut préparer ce choix
depuis une référence M17 vérifiée, sans lui donner une autorité moteur.

Le contexte courant est toujours produit par le moteur. L'instantané opérateur
est contrôlé mais exclu du texte injecté. Seuls les messages protégés du préfixe
et le suffixe choisi sont joints comme données historiques. L'aperçu doit être
confirmé avec son `context_hash`. La demande et le manifeste conservent choix,
sources reçues/omises, empreintes et état de vérification avant démarrage.

| Lot | Propriétaire / dépendance | Critère observable |
| --- | --- | --- |
| M19.1 | Codex ; M18 | Schéma et test public rouge avant réalisation ; sources et différences avec l'import d'un reçu identifiées. |
| M19.2 | Codex ; M19.1 | Validation commune à l'aperçu et au départ, références/revision/frontière/états contrôlés, aucune mutation sur refus ; instantané opérateur jamais injecté. |
| M19.3 | Codex ; M19.2 | CLI `agent context-preview` et action API `agent-context-preview`, confirmation du prompt obligatoire ; sélection portée par le seul lancement, pas par le profil réutilisable. |
| M19.4 | Codex ; précédents | Vérification supplémentaire des sources mises en file juste avant le sous-processus ; dérive ou échec d'enregistrement interdit le départ. Vrai fournisseur scripté reçoit exactement le prompt persisté, sans acceptation métier. |
| M19.5 | Codex ; précédents | Préparation Python vérifiée, contrôles de non-régression, recette CLI/API isolée, docs FR/EN, revue locale et RETEX. |

La sélection est réservée au lancement manuel automatisé d'un worker hors mission
hiérarchique : ni brainstorming, terminal, dialogue, restauration, parent,
conducteur, tentative antérieure, ni références complètes simultanées. Les refus
ne deviennent jamais une sélection vide réussie : originaux conservés, départ
refusé et nouvel examen demandé. Le contrôle avant démarrage est daté ; il ne
promet ni fraîcheur continue après son observation ni révocation d'un processus
déjà démarré.

Pas de modèle réel, paraphrase LLM, politique automatique, nouvelle durée/taille
de résumé, purge, migration ou écran. Les limites de transport existantes restent
distinctes d'une politique opérateur. Aucun déploiement ou revue indépendante.
Les critères globaux restent partiels tant que la qualité sémantique, les autres
parcours et le résumé progressif général ne sont pas qualifiés.

## Résultat local

M19.1–M19.5 vérifiés : 21 tests Go distincts dont deux avec `-race`, 49 tests
Python (cinq nouveaux et 44 existants), cinq contrôles natifs M19 et cinq M18.
L'entrée réelle du worker scripté égale le prompt persisté et les citations de
référence ; omission attribuée, confirmation complète, fraîcheur, refus et rejeu
sont vérifiés. Voir [résultats et RETEX](RESULTATS-MEMOIRE-15.md) pour les défauts
reproduits, corrections, erreurs de méthode et limites. Aucune revue indépendante,
activation automatique, évaluation LLM réelle ou modification du serveur utilisateur.
