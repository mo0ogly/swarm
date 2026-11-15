# Garde d'initialisation Sécurité

Statut au 15 novembre : sources et garde intégrées au checkout ; image380 qualifiée et installée localement ;
session humaine Authority distincte non vérifiée.
Le test ciblé adapté a passé sept scénarios sur les deux assets candidats. La
recette permanente proposée n'a pas été exécutée intégralement : ses étapes n'ont pas été lancées. Cette preuve synthétique seule ne qualifie aucune image native ; les résultats380 distincts sont indiqués plus bas.

`tests/security_initialization_ui.cjs` utilise Chrome réel, le HTML et les scripts
versionnés. Tous les échanges sont interceptés : GET de ressources/session/show,
POST config-preview synthétique explicite uniquement. Aucune connexion à un
serveur, moteur, Authority ou fournisseur, aucune écriture config-apply. Les
valeurs de la fixture sont des données de test, pas des politiques produit.

Les sept scénarios sont un témoin normal, données avant montage, montage avant
données, conservation des saisies/jeton/preview, refus initial et reprise, saisie
avant réponse et reprise. Les trois démarrages doivent remplir les champs sans
clic Admin ; les reprises utilisent un geste explicite. Aucune temporisation,
retry ou watchdog global. Les états attendus et la réception des requêtes libèrent
les réponses différées. Les diagnostics inattendus interrompent les attentes ;
fermetures page/navigateur et leurs erreurs sont conservées avant statut final.

Exemple après intégration, depuis la racine du dépôt : créer un JSON privé avec
`{"output":"/chemin/admis/preuve-fraiche"}` puis exécuter :

```sh
SWARM_SECURITY_INIT_CONFIG=/chemin/prive/config.json python3 tools/verification/recipe.py run --recipe tools/verification/recipes/security-initialization-ui.json --output /chemin/admis/recette-fraiche
python3 tools/verification/recipe.py verify --evidence /chemin/admis/recette-fraiche
```

La syntaxe run/verify a été rapprochée du parseur actuel ; les chemins sont
des exemples à remplacer après admission du support. La configuration JSON
admet uniquement output, cases et asset_root. Par défaut tous les cas et web/.
Sélection directe des seuls nouveaux cas via cases, parmi les sept noms du code ;
le témoin normal est toujours ajouté. La recette complète attend les sept cas ;
pour une sélection, lancer directement `node tests/security_initialization_ui.cjs
/chemin/prive/config.json` et annoncer exactement les noms exercés. Ne pas modifier
l'assertion de recette pour fabriquer un PASS avec une campagne partielle.

asset_root sert seulement à une proposition privée de sources, explicitement
tracée dans result.json ; les fichiers absents viennent de web/. Il ne représente
pas des ressources embarquées ou protégées. La recette finale doit omettre cet
override, comparer ses inputs avant/après et vérifier ses preuves. Les empreintes
des octets réellement fournis et du harness sont contrôlées avant/après ; le helper
browser_verification_runtime collecte Node/npm/Chrome. Un baseline npm PASS
explicitement fourni via SWARM_VERIFICATION_NPM_BASELINE est relu et comparé,
sans transférer sa preuve runtime. Son absence déclenche la collecte normale.
Les profils Chrome restent privés et sont exclus des archives/remises.

Limites : doubles API, pas de mutation/journal ni de refus backend établis, pas de
session humaine Authority, pas de recette de certification. Une nouvelle image
contenant les assets exige sa qualification et les parcours protégés pertinents.
La livraison380 et les notes FR/EN sont établies par leur reçu distinct ; cette
garde synthétique ne les établit pas. Cette garde ne rejoue pas les campagnes historiques53/152.

## Corrections de revue ORACLE-02 et READY-03

Les anciens rapports reproduce/guards/early-edit écrivaient adminClicks:0 en
constante. Ces champs sont NON MESURÉS ; leurs journaux restent inchangés. Le
driver maintenable instrumente les clics avant navigation et sépare phase
initiale0 mesurée et reprise1 mesurée. Aucun comportement n'est déduit de la
constante historique ; la portée des observations DOM distinctes est conservée.

Le septième cas injecte un échec de rendu synthétique après affectation de state,
puis retarde le montage ; il ne prétend pas qu'un moteur fournit cet état. Après
témoin normal valide, l'ancienne source échoue sur absence de champs automatiques.
Le nouveau booléen de readiness est effacé avant chaque lecture et posé seulement
après validation et rendu réussis ; le montage ne promeut plus un état partiel.
Aucun champ n'apparaît automatiquement après échec ; une relecture explicite saine
reprend. Ces doubles ne prouvent pas l'état du journal moteur ; config-apply est
refusé par le harness. La révision privée02 passe sept cas ; la preuve47 assertions
reste historique, remplacée pour le contrat load/readiness modifié. Pas de rejeu53/152.

Docstring load clarifiée en révision03 : le refus concerne la readiness du
remplissage initial, pas une désactivation générale des contrôles. Les gates et
la relecture Admin explicite restent inchangées. Seul le commentaire bloc est
remplacé, octets exécutables hors de ce bloc identiques à révision02. La preuve
61 assertions conserve ses sources exactes en révision02 ; aucune nouvelle
exécution ni image courante n'est déduite de cette correction documentaire.

Le contrôle natif BLP couvre séparément les deux ordres par continuation des vrais GET, sans réponse API synthétique, puis les quatre parcours existants. Exécuté sur380 :59 assertions PASS et intégrité de recette VERIFIED.

Le contrôle Version maintenu accepte un build propre ou modifié selon le booléen explicitement attendu. Il vérifie l’absence ou la présence du seul avis de modifications locales ; une provenance non booléenne est refusée. La recette `version-provenance-oracle` conserve les témoins propre/modifié, les avis incohérents, le refus sans mutation et la reprise. Elle ne prouve ni installation ni session Authority.
