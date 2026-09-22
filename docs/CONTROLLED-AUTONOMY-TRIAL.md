# Préparer un essai d’autonomie avec faute contrôlée

Statut : composants de recette testés avec des processus simulés. Aucun essai
avec un modèle réel ni acceptation de mission ne découle de ce document.

## Défaut métier et preuve attendue

Le petit projet expose `feature.py:is_expected(value)`. Il doit renvoyer le
booléen vrai seulement pour la chaîne `expected`. Le contrôle indépendant teste
`expected`, `wrong`, la chaîne vide et `None`.

L’adaptateur de l’exécutant attend la fin réussie de sa première production,
vérifie que celle-ci satisfait le contrôle, puis remplace la fonction par une
version renvoyant toujours vrai. Il démontre aussitôt l’échec métier. Il conserve
hors des copies les identités mission/agent/tentative, le contenu original, les
empreintes avant/après, celles du contrôleur et de l’injecteur, et les codes de
sortie. Ce défaut est une injection explicite du banc d’essai, pas une erreur
attribuable au modèle.

Le moteur doit ensuite refuser le résultat lors de ses propres contrôles. Le
planificateur doit décider de la correction dans les limites existantes. Les
productions suivantes ne sont plus modifiées. Une interruption entre réservation
et preuve d’injection laisse un état incomplet : l’essai doit être examiné, pas
rejoué silencieusement.

## Composants

- `tests/controlled_business_fault.py` : injection unique, contrôle métier et preuves.
- `tests/autonomy_worker_adapter.py` : lecture de l’identité par `agent list`,
  transmission des arguments, du prompt et des sorties au producteur ; injection
  seulement après sa sortie réussie. Vérifie l’empreinte du moteur avant et après.
- Les fichiers `test_controlled_business_fault.py` et `test_autonomy_worker_adapter.py`
  vérifient ces comportements sans fournisseur IA.

L’adaptateur est réservé au fournisseur **worker** d’un Store jetable dédié.
Le planificateur et le vérificateur gardent leur fournisseur normal. Ne jamais
brancher cet injecteur sur une mission de production ou une copie utilisateur.
Les restrictions de chemin sont une garde de recette, pas un bac à sable contre
un agent hostile exécuté sous le même compte système.

Configuration JSON de l’adaptateur, stockée hors des copies :

```json
{
  "engine": "/chemin/absolu/swarm-fige",
  "engine_sha256": "EMPREINTE_VERIFIEE_DU_BINAIRE",
  "store": "/chemin/absolu/store-jetable",
  "work": "IDENTIFIANT_REEL_DE_LA_MISSION",
  "copies_root": "/chemin/absolu/des/copies-gerees",
  "evidence_dir": "/chemin/absolu/preuves-injection",
  "producer": "/chemin/absolu/du/fournisseur-reel"
}
```

Son invocation est `python3 tests/autonomy_worker_adapter.py CONFIG [arguments du
fournisseur]`, avec le prompt sur stdin. Conserver l’identité de programme attendue
par le connecteur lors de la création du lanceur dédié ; ne pas sélectionner ce
lanceur pour la planification ni pour la revue. Le moteur contrôle toujours les
limites habituelles. L’adaptateur ne crée pas la mission et ne l’autorise pas.

## Vérification locale

```sh
python3 -m unittest discover -s tests -p test_controlled_business_fault.py -v
python3 -m unittest discover -s tests -p test_autonomy_worker_adapter.py -v
```

## Conditions avant un essai réel

1. Figer binaire, commit moteur, commit de recette et schéma de stockage. Refuser
   de changer de moteur pendant l’essai.
2. Préparer le dépôt jetable et un contrôle moteur utilisant les mêmes assertions
   métier, hors des fichiers modifiables par l’exécutant. Ignorer les caches Python.
3. Fixer et autoriser séparément les budgets planificateurs/producteurs/revues.
   Les plafonds épuisés d’une mission existante ne sont pas réinitialisés par cette recette.
4. Brancher l’adaptateur seulement sur l’exécutant et tester la configuration sans
   appel de modèle avant le lancement autorisé.
5. Enregistrer les événements publics : délégation réelle, refus sur le candidat
   injecté, décision réelle de reprise, nouvelle production, contrôles et avis
   indépendant sur le même SHA, puis fermeture des périmètres.
6. Observer uniquement après départ : aucune décision `claim/decide/retry/close`
   fabriquée par le script de recette. Toute réparation extérieure doit apparaître
   comme telle et interdit de conclure à une autonomie sans intervention.

## Lanceur et observateur

`tests/controlled_autonomy_campaign.py` expose deux commandes séparées :

```sh
python3 tests/controlled_autonomy_campaign.py prepare DOSSIER_NEUF --engine /chemin/swarm --providers /chemin/providers.json --provider claude
python3 tests/controlled_autonomy_campaign.py run DOSSIER_NEUF
```

`prepare` initialise un Store et un dépôt jetables par le CLI public, configure
la planification et les fournisseurs, puis s’arrête sans activer la mission ni
appeler un modèle. Le dossier doit être neuf. Aucun ancien compteur n’est remis
à zéro. Le moteur est copié ; son empreinte, les fichiers de recette et les
configurations sont figés. Les configurations de précontrôle personnalisé sont
refusées plutôt que désactivées ou exécutées implicitement.

**`run` démarre des appels de modèle et exige leur autorisation préalable.**
Plafonds : 12 activations de planification, une tâche avec 2 productions maximum,
4 revues, 300 secondes par production, 100 appels d’outils au plus par production
(ou moins si le fournisseur l’impose), observation limitée à 20 minutes. Un marqueur
empêche de relancer silencieusement une campagne déjà commencée. Un dépassement,
un manque de preuve ou une panne rend le résultat non favorable ; aucune relance
illimitée n’est prévue.

L’observateur lit les états, l’historique et les agents par le CLI public. Il
contrôle l’attribution de l’injection, le diagnostic métier du moteur et son
contenu Git, la correction, la fermeture des périmètres, la fraîcheur et l’identité
SHA contrôles/revue/publication, ainsi que les empreintes des preuves. Il refuse
les reprises externes détectées. Le lanceur n’expose aucune commande pour fabriquer
une décision de planificateur ou une acceptation. Il arrête la mission à la fin ;
si l’arrêt des agents/revues ne peut être confirmé, il signale un échec et conserve
le serveur pour diagnostic.

Le test intégré suivant utilise le vrai moteur mais un fournisseur déterministe,
pas Claude :

```sh
SWARM_CAMPAIGN_TEST_ENGINE=/chemin/swarm python3 -m unittest discover -s tests -p test_controlled_autonomy_campaign.py -v
```

Le test prouve la chaîne mécanique. Lancer Claude et observer ses décisions reste
une expérience distincte, avec budget autorisé. Une sortie PASS du banc n’accepte
pas E6 à la place du moteur de la mission principale.
