# Parcours guidés pour préparer une mission

Dans **Préparer un projet**, choisir **Choisir un modèle de mission**. Dix
modèles sont proposés : quatre structures générales (évolution d’application,
défaut reproductible, interface et nouvelle application) et six sujets
spécialisés : outil interne, portail client, API et intégration, gestion des
stocks, traitement de données et tableau de bord. La modale permet de lire le brouillon avant
insertion. Compléter les indications entre crochets, puis enregistrer le besoin.
Le modèle ne remplace jamais une saisie existante : télécharger le brouillon ou
ouvrir une nouvelle préparation pour repartir d’une structure différente.

Le catalogue FR/EN est embarqué et versionné dans
`tools/agent-workflows/templates/preparations.json`. Le même catalogue est
accessible en ligne de commande, sans appel IA ni modification de mission :

```sh
swarm prepare templates
swarm prepare template application
swarm prepare template correction
```

La sortie JSON contient `title.fr`, `title.en`, `need.fr`, `need.en`, les phases
référencées et la version du modèle. Ces champs peuvent servir à préparer une
requête `prepare create --input requete.json` avec le contrat habituel : version,
action, event_id unique, expected_revision zéro, title, text et method.
L’enregistrement ne constitue ni un brief adopté ni un plan validé.

## Structure et méthode sont distinctes

Les parcours décrivent les étapes utiles : cadrage, recherche, conception si
nécessaire, planification, réalisation, revue et livraison. Ce sont des
propositions, pas des commandes exécutées. Le pack fournit désormais les méthodes
produit adaptées à Swarm et les modèles documentaires génériques du parcours produit ; il n’importe
ni règles privées d’application ni permissions de publication.

Les évolutions, interfaces et nouveaux produits utilisent **Parcours de création d’application** ; la correction utilise le
diagnostic structuré. La disponibilité est contrôlée avant un appel IA. Un
brouillon reste conservé si la méthode est indisponible. La préparation complète
exige la structure produit pour vérifier le plan ; aucune phase ne lance d’agent.
Voir la [étapes du parcours et les vues des graphes](PRODUCT-WORKFLOW.md).

Les rôles, dépendances, workspaces, preuves et limites sont des exigences du
brouillon à compléter. Le modèle ne crée pas une équipe ni un vérificateur. La
vérification du plan, l’organisation et l’autorisation de départ restent les
contrats existants. Il ne préautorise ni budget, acceptation, publication ou merge.

Le catalogue contient quatre structures générales et six sujets embarqués. L’import, l’édition et
la sauvegarde de modèles personnalisés ne sont pas encore proposés.

## Préparation guidée et équipe proposée

Chaque modèle expose huit questions : résultat ou défaut reproductible, public,
périmètre, exclusions, contraintes, réussite observable, contrôle et reprise.
**Vérifier mes réponses** produit le brouillon et une liste de réponses manquantes.
Chaque élément de la liste ramène le focus sur le champ concerné. Le choix
**Utiliser ce brouillon** refait ce contrôle avant insertion. Un besoin incomplet
peut être conservé ; il est explicitement signalé comme incomplet.

Les réponses sont conservées pendant les changements de modèle et les fermetures
de la modale, dans la page courante. Pour les conserver après rechargement,
insérer le brouillon puis enregistrer le besoin avec le formulaire habituel.
Une panne du contrôle n’insère aucun texte et préserve les réponses dans la page.

Les cartes présentent le planificateur, les exécutants et le vérificateur
indépendant. Elles décrivent une organisation proposée, sans compter des agents
actifs. Un sous-planificateur reste conditionnel à un découpage justifié ; il
n’est pas créé par le modèle. Les rôles, espaces et modèles IA effectifs sont
ensuite définis et contrôlés dans le plan.

Le contrôle est déterministe et vérifie la présence des réponses, pas leur
pertinence ni la vérité des affirmations. `need_complete` ne veut pas dire que
le plan est valide ou prêt à partir. `launch_authorized` reste toujours faux.
Les vérifications et autorisations du moteur restent obligatoires.

### Même contrôle depuis le CLI

```sh
swarm prepare template correction
swarm prepare template-check correction --input reponses.json
```

Exemple de réponses partielles (aucune écriture ni appel IA) :

```json
{
  "language": "fr",
  "answers": {
    "objective": "Le bouton Enregistrer ne conserve pas le nom après rechargement.",
    "scope": "Formulaire de préparation uniquement",
    "exclusions": "Préserver le graphe et les missions existantes"
  }
}
```

Les identifiants attendus sont `objective`, `audience`, `scope`, `exclusions`,
`constraints`, `acceptance`, `verification`, `recovery`. Les autres clés sont
refusées. Une réponse est limitée à 1 000 octets UTF-8. La sortie contient le
brouillon rendu, `missing` (identifiants des réponses manquantes), `answered`,
`total`, `need_complete`, `proposed_team`, `recommended_method` et
`launch_authorized`. Un sujet retourne aussi `intervention`. Les réponses
vides ou contenant les indications à compléter du modèle restent manquantes.

## Méthodes fournies aux rôles

Les cartes et le brouillon précisent la méthode de chaque rôle : Analyse et planification pour cadrer le besoin, Réalisation et vérification pour
réaliser puis vérifier une tâche autorisée,
Revue indépendante pour examiner les preuves et proposer des corrections.
Le sous-planificateur conserve les limites de planification. La préparation
ne permet aucune exécution, même lorsqu’une méthode décrit aussi des phases de réalisation.

Les champs `team[].workflow` du catalogue et `proposed_team[].workflow` du
contrôle web/CLI viennent de la même
fonction `agentWorkflow` que les départs : version, rôle, méthodes et empreinte
du cadrage embarqué. Il décrit les méthodes fournies par le moteur, sans prouver
qu’un agent les a suivies. Les consignes effectives incluent ce cadrage lors du
départ ; une proposition de méthode dans le besoin ne peut pas le remplacer.
Les rôles ne gagnent ni permission, budget ni droit d’acceptation.

Le parcours « Examiner et améliorer » distingue Préparer, Réaliser, Vérifier et
Améliorer. Le vérificateur propose les corrections ; il ne les réalise pas. La
préparation reste limitée à l’analyse et au plan. Les identifiants techniques
et les méthodes embarquées ne changent pas avec ces noms d’usage.

Le parcours **Corriger un défaut reproductible** utilise désormais
**Diagnostiquer et corriger un problème**. En préparation, l’IA propose le plan de
diagnostic ; elle n’exécute aucun outil ni correction. Les nouveaux exécutants
reçoivent aussi la méthode embarquée selon les limites de leur rôle.


## Sujets et interventions

L’accueil **Projets et développement** présente les six sujets. Chaque entrée
expose le résultat attendu et les preuves à prévoir ; elle ouvre directement
le sujet choisi dans la préparation. La modale ajoute un exemple, les livrables
et les risques à examiner. Aucun besoin n’est enregistré par ce choix.

Choisir ensuite **Créer**, **Faire évoluer**, **Corriger** ou **Migrer**. Le sujet
et l’intervention enrichissent le brouillon sans remplacer le cadre commun :
**cinq étapes communes au produit, puis six étapes répétées pour chaque
fonctionnalité**. Les acquis sont examinés avant réutilisation ; une étape non
applicable doit être justifiée. Aucun suivi automatique d’étapes n’est créé.

Corriger propose la méthode de diagnostic ; les autres interventions proposent
le parcours produit. Le choix est conservé par sujet dans la page, comme les
réponses. L’enregistrement explicite conserve ensuite le besoin et la méthode.
Une migration exige encore la décision de bascule, les contrôles de parité et
une reprise vérifiée ; sélectionner Migrer ne donne aucune autorisation.

```sh
swarm prepare template service-api
swarm prepare template-check service-api --input reponses-api.json
```

Le fichier de réponses utilise le contrat précédent avec un champ optionnel
`"intervention": "migrate"`. Les valeurs admises sont `create`, `improve`,
`correct`, `migrate` ; l’absence du champ choisit `create` pour un sujet.
Les quatre structures générales conservent leur contrat sans intervention.
L’API publique `/api/v1/preparations/template-check` utilise la même projection
que le CLI. Le catalogue et ses métadonnées FR/EN sont versionnés ensemble.

## Modèle de travail Portail client

La version 2 expose un graphe inspectable et cinq incréments : fondations communes,
connexion et sessions, documents, administration des accès et révocation de bout
en bout. Les cinq fondations sont partagées ; chaque fonctionnalité suit les six
étapes du parcours produit. Les tâches design et backend convergent vers
l’intégration, la revue puis la livraison préparée.

Dans le dialogue du modèle, choisir l’incrément, inspecter les tâches et leurs
critères, puis compléter les réponses. **Télécharger le plan adapté** retourne
un `ActionPlan` v1 ; **Utiliser ce parcours**, puis enregistrer la nouvelle
préparation, conserve le besoin et insère le plan dans son éditeur. Le brief reste
à rédiger et adopter ; les décisions ouvertes doivent être résolues avant de
vérifier le plan et créer les missions. Les réponses ne sont jamais interprétées
comme des commandes de contrôle ou une autorisation de publication.

Le CLI utilise la même projection :

```sh
swarm prepare template-check client-portal --input reponses.json
```

Le fichier accepte `increment` : `foundations`, `session`, `documents`,
`administration` ou `revocation`. La réponse contient `work_plan` et `increment`.
Les plans ont au plus huit tâches ; les dépendances locales sont contrôlées par
le moteur. Les stories futures restent non planifiées. Les préconditions entre
incréments demandent une décision explicite et des preuves acceptées avant adoption ;
aucune synchronisation automatique entre missions n’est annoncée. Les refus et
reprises sont présentés séparément des dépendances : leur traitement utilise les
opérations existantes de correction et révision, sans introduire un ordonnanceur
conditionnel. Les politiques exécutables de contrôle restent à autoriser pour
le dépôt et le candidat réels.
