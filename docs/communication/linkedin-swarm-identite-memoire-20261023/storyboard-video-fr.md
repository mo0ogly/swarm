# Swarm scénario de vidéo sur identité mémoire et contrôle

23 octobre 2026. Scénario révisé sur demande utilisateur : intégrer la supervision
déjà réalisée. Proposition de vidéo mixte d’environ 120 secondes : architecture
et démonstration ciblée de supervision, après répétition du parcours à filmer.
Le minutage est un choix éditorial, pas une limite moteur ou de vérification.
**Rendu disponible :** [vidéo explicative MP4](Swarm_Supervision_Identite_Memoire.mp4),
120 secondes, sous-titrée en français, sans voix ni capture du logiciel. Ses huit
chapitres sont décrits dans le [dossier vidéo](video/README.md).
Le scénario mixte ci-dessous conserve la proposition de démonstration CLI :
cette capture d’application reste à répéter et filmer, elle n’est pas simulée
dans le MP4 livré.

| Temps indicatif | Visuel | Voix proposée |
|---|---|---|
| 0–12 s | Architecture, mention « architecture de conception » | « Swarm relie la conduite d’une mission, l’identité de ses agents et la provenance de leur mémoire. » |
| 12–30 s | Zoom sur le contrôle : racine, périmètre, observation, décision et application | « Le responsable décide de la conduite du travail. Le moteur observe, vérifie les droits, applique une décision attribuée et confirme ses effets. Une durée écoulée ne décide pas de la qualité du résultat. » |
| 30–65 s | Séquence CLI à filmer après répétition : planification → worker → suivi → fin ; variante de reprise queued. Afficher « fournisseurs de fixture, parcours ciblé » et l’identité du binaire | « Voici le périmètre de supervision déjà couvert dans Rust : lancement et suivi d’un worker, reprise d’un travail en attente et enchaînement planification–exécution. La séquence utilise un scénario de test déterministe. » |
| 65–85 s | Diagramme mémoire, puis identité et usages distincts | « La mémoire conserve des leçons versionnées et le contexte reçu. Lire, résumer et transmettre sont des usages distincts ; leur raccordement commun à l’identité reste à qualifier. » |
| 85–105 s | Bloc orange AC-02 ; mention « chantier futur non démarré » | « Le plan suivant complète ce socle par le confinement, le réseau, les secrets et la traçabilité, avec des références OpenShell, Bromure et Agent Baseline. » |
| 105–120 s | Carte des capacités : Go documenté, parcours Rust vérifiés, intégration ouverte ; références | « La vidéo montre des parcours précis et présente les étapes du projet. La supervision web, les autres rôles et les nouvelles protections ne sont pas déclarés terminés. » |

## Ce qui peut être produit maintenant

Une vidéo de conception avec ces diagrammes, des animations de mise en évidence
et une narration relue peut être produite sans démarrer AC-02 ni attendre la fin
de Rust. Elle doit garder les mentions de portée et d’état sur les plans concernés.
Les exports PNG/SVG sont les supports préparés ; aucun écran de produit futur
n’est simulé comme s’il était livré.

La supervision peut compléter cette présentation par une séquence réelle sur
fixture isolée. Les preuves existantes permettent de choisir le scénario ;
elles ne remplacent pas une nouvelle répétition ni la capture de son résultat.
Le montage peut condenser des étapes en le signalant ; son minutage ne constitue
pas une mesure de performance du moteur.

## Socle de supervision à présenter

Le [contrat de conduite](../../architecture/RUN-SUPERVISION.md) décrit la
séparation entre décision du superviseur et observation/application par le
moteur dans le socle Go. Il ne démontre pas une campagne multi-agent autonome
complète. Le [plan Rust](../../plans/migration-rust/PLAN.md) conserve les
branches ouvertes et les trois résultats PASS relus pour ce scénario :

- [213](../../plans/migration-rust/evidence/20261022-root-instance-201/watch-public213/result.json) : lancement/suivi d’un worker et arrêts du conducteur.
- [224](../../plans/migration-rust/evidence/20261022-g3-watch-restart/public224/result.json) : reprise d’un worker queued avec une nouvelle génération.
- [244](../../plans/migration-rust/evidence/20261022-g3-watch-planning/public244/result.json) : planification automatique jusqu’à la fin d’un worker de fixture.

Leur portée reste celle des recettes déclarées ; ces résultats ne constituent
ni acceptation globale, ni preuve d’un fournisseur IA réel, ni qualification
de tous les rôles. Les chemins registered/reviewer/validation et web restent
ouverts dans l’état consulté. Aucun de ces contrôles n’est relancé ici.

## Conditions pour une démonstration produit

| Séquence souhaitée | Précondition observable |
|---|---|
| Lancement/suivi/reprise CLI Rust | Choisir un parcours public déjà couvert, refaire une répétition sur fixture isolée et identifier le binaire filmé. Les preuves historiques ne prouvent pas la session filmée. |
| Catalogue, sélection et retrait mémoire | Identifier la version réellement servie. Le socle Go documenté n’établit pas le parcours Rust G8 ni les droits d’usage globaux. |
| Identité, approbation humaine et révocation | Qualifier le parcours public G7 et les consommateurs, y compris mémoire et départ des agents. Une fixture de noyau ne remplace pas cette séquence. |
| Nouvelle sécurité AC-02 et menus | Attendre réalisation/qualification des lots concernés ; le plan est non démarré. |
| Démonstration intégrée dans le navigateur | Qualifier la version servie, les ressources réelles et les parcours web G11, puis faire une répétition complète. |

Le filmage d’un service Go existant reste une possibilité à vérifier sur sa
version et son parcours. Il ne constituerait pas une démonstration Rust. Aucun
service, mission, fournisseur, base active, build ou restart n’est manipulé ici.

Préparation technique disponible localement : Mermaid CLI et FFmpeg détectés.
Cela permet de produire des illustrations et un montage explicatif ; cela ne
qualifie aucun parcours Swarm. Aucun accès à un générateur vidéo spécialisé ni
service de synthèse vocale n’a été établi dans cette session.
