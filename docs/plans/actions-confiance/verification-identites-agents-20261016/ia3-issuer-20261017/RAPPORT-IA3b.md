# IA3b — émetteur humain interne et preuves de publication

17 octobre 2026. Codex seul, travail direct, sans mission native, fournisseur réel,
revue indépendante, commit, push ou déploiement. Branche `codex/clear-product-method-names`, HEAD `4e58f7cbf8200a58f69f1f4d03d657c3313fb642`,
checkout partagé déjà modifié. **Noyau privé vérifié ; admission globale partielle.**

## Résultat et périmètre

Un permis peut maintenant être produit depuis une session OIDC résolue par Trust
et une autorité de publication explicitement fournie par le contrôleur. Le permis
lie le candidat complet, les racines, le principal, le projet, le plan courant,
la révision Trust, l'événement, la session et son échéance. Les nouvelles racines
et l'activation d'anciennes racines non reconnues restent dans les permissions,
dates et possibilités de délégation de cette autorité.

Le publiant revalide cette restriction après l'attente SQL et avant son intention
durable. Le producteur contrôle l'autorité avant recherche du travail, puis la
réauthentifie après attente. L'émission est sans publication ni consommation.
Une reprise ne peut que terminer l'intention historique exacte déjà autorisée.

Cette autorité de contrôleur est **une fixture** : sa référence syntaxiquement
valide ne prouve pas un enrôlement. Aucun service protégé d'enrôlement, témoin de
non-révocation, compte humain externe ou source indépendante déployée n'est livré.
Le chemin bas niveau antérieur reste une capacité privée de contrôleur utilisée
par les fixtures sans preuve humaine ; aucun endpoint public ne choisit entre
ces chemins. Le futur adaptateur devra imposer le producteur et sa source protégée.
Une session OIDC catégorisée humaine ne prouve pas présence physique, MFA ou
accord d'un approbateur distinct sur une création résolue.

## Vérifications et preuves originales

| Contrat observé | Résultat | Preuve et limites |
|---|---|---|
| Émission OIDC, candidat exact, publication puis lecture après réouverture | PASS local | Protocole réel contre fournisseur local à jetons signés ; source d'enrôlement simulée. |
| Identité absente/locale/service, rôle seul, homonyme, autre projet/plan/révision, dates ou droits invalides | PASS local | Refus exact, permis nul et stockage inchangé ; 28 variantes de la table principale. |
| Extension des racines/délégation, racine nouvelle non listée, activation d'une racine conservée | PASS local | Plafond explicite, conservation de l'historique. |
| Transfert du permis à une autre session du même humain | FAIL avant → PASS après | `issuer-before*`, sources avant correction ; accord erroné réellement observé. |
| Expiration pendant attente SQL au publiant | FAIL avant → PASS après | Attente observée par `DB.Stats().WaitCount`, vraie échéance ; aucune intention après refus. |
| Travail absent distingué avant autorisation | FAIL avant → PASS après | Trois variantes dans `lookup-before*` ; `lookup-after*` vérifie aussi la fraîcheur après attente. |
| Plan amendé/autorité expirée pendant l'attente de l'émetteur | PASS local | Temps frais et plan effectivement engagé, pas un hash déclaré. |
| Session suspendue après émission ; reprise historique après expiration | PASS local | Nouvelle publication refusée ; seule l'intention exacte peut être finalisée. |
| Jeu final de l'émetteur | **47/47 PASS normal et 47/47 PASS sous `-race`** | `frozen-normal-final-run.json`, `frozen-race-final-run.json` ; zéro FAIL/SKIP, noms exacts retrouvés. Les répétitions ne sont pas des attaques supplémentaires. |
| Statique et compilation finales | PASS | `frozen-vet-final-run.json`, deux compilations de tests ; binaire applicatif ordinaire également compilé avant optimisation des seules fixtures. Aucun binaire déployé. |
| Consigne documentaire persistante et pack | PASS fichier/assemblage | Accords personnels Codex référencés par Claude, AGENTS projet, contrat et docs FR/EN. `method-pack-final.stdout` : 12 skills ; quatre rôles assemblés dans les contrôles. Ni obéissance d'un modèle ni mise à jour d'un processus déjà lancé prouvées. |

Les deux tentatives initiales d'émission/publication donnent deux FAIL nommés.
La recherche du travail avant autorisation donne trois variantes FAIL et le
parent FAIL, soit quatre noms ; ne pas compter ce parent comme une quatrième
attaque. Les originaux et les sources sont conservés, sans écrasement par un PASS.

## Gel, ressources partagées et récupération de la vérification

La série ordinaire conserve **278 PASS / 1 FAIL**, sans SKIP. Le FAIL
`TestEngineContractRevisionKillsInferenceOnContractMutation` mesure 2,273 s pour
une garde de 1,8 s incluant le démarrage. Il passe ensuite seul, avec le même seuil
et sa source inchangée (`timing-isolated-final*`). CPU très occupé et compilation
concurrente sont observés ; la causalité précise et la tenue sous charge ne sont
pas établies. Ce rapport ne revendique pas une série globale intégralement verte.
La limite du test sous charge reste attribuée à la vérification moteur.

Un groupe instrumenté de 90 s expire pendant les migrations répétées d'une fixture,
après 19 noms PASS ; c'est **INCOMPLET**, même sans FAIL individuel. La pile est
gardée dans `race-final.jsonl`. Les refus immuables partagent ensuite une fixture
par table (28→1 et 3→1), avec snapshots/listes copiés profondément, mêmes assertions
et stockage inchangé après chaque variante. Les mutations, reprises et attentes
gardent leurs fixtures distinctes. La seconde base du helper partagé reste une
amélioration I06 ouverte. Ni délai ni assertion n'est augmenté ou supprimé.

Des changements web concurrents ajoutent deux entrées embarquées et modifient
`web/project-memory.*` ; aucun de ces fichiers n'est modifié par ce chantier.
Le paquet racine embarque aussi les méthodes : le jeu d'entrées ordinaire varie
alors de 1 058 à 1 060. Un overlay temporaire fige 136 ressources web et remplace
virtuellement le seul glob `web/*` par sa liste identique à la capture. Les
sources Go restent dans le checkout. Cette vue est décrite et hachée dans
`frozen-embed-view.json` ; elle ne qualifie pas la dernière interface web en cours
d'édition. Aucun fichier du checkout n'est remplacé par cet overlay.

Un binaire instrumenté est compilé une fois puis exécuté directement avec
`go tool test2json`, de même pour le binaire normal. Leurs empreintes avant/après
et toutes les entrées de compilation sont conservées. Chaque étape finale garde
un jeu inchangé de 1060 entrées dans cette vue. Les sources centrales
sont dans `source-final/` ; les 136 ressources figées sont conservées dans
`frozen-web/` avec `retained-web-view.json`, sans dépendre du seul répertoire temporaire. Les groupes de partition préparés n'ont pas été exécutés :
le partage contrôlé des fixtures a remplacé cette option avant lancement.
La comparaison `preservation.json` garde 1 049 anciennes entrées inchangées ;
hors vue web et `resources.go` virtuel, seuls les deux fichiers Go existants du
permis/publiant et le contrat commun changent, avec deux nouveaux fichiers Go.
Les trois fixtures de revue rejetées auparavant restent exactement identiques.
Le premier lecteur de ce manifeste a supposé un champ absent `inputs` ; la
correction utilise le vrai `inputs-final.json`, après inspection du format,
sans modifier les preuves historiques (`preservation-diagnosis.json`).

La durée d'une compilation ne doit pas être confondue avec `-test.timeout`, qui
borne l'exécution des tests, pas la préparation du binaire. Aucun gain causal
isolé ni benchmark comparatif n'est déduit des facteurs de récupération combinés.

## Documentation, réception et améliorations conservées

[Contrat FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md),
[contrat EN](../../../../en/AGENT-IDENTITY-CONTRACT.md),
[contrat IA2](../../identites-agents-contrat-IA2.md) et
[plan de reprise](../../identites-agents-replan.md) décrivent objets, erreurs,
verrous, limites et recette. Deux schémas FR/EN sont rendus en SVG/PNG et inspectés ;
leurs sources restent identiques aux derniers blocs Mermaid des deux contrats.
La note scientifique et le programme de recherche conservent preuves et limites,
sans revendication de première scientifique ou de sécurité comparative mesurée.

Les premières condensations du contrat dépassaient 32 000 octets ; une relance
est aussi partie après un script d'édition échoué. Ces erreurs sont conservées.
Le texte est consolidé sans troncature/augmentation de capacité et le contrôle
final du pack passe. La consigne détaillée persiste dans
`/home/fpizzi/.codex/AGENTS.md`, référencée par `/home/fpizzi/.claude/CLAUDE.md` ;
la session a reçu le texte actualisé. Cela ne prouve pas une future exécution.

Les améliorations I04 (reçu durable d'émission), I05 (manifeste ancien pendant
relance), I06 (coût des fixtures) et I07 (ressources embarquées couplées) gardent
origine, propriétaire, critère, état et condition de reprise dans le plan.
Leçons vérifiées : une émission ne dispense pas de revalider au publiant ; une
identité n'est pas une autorité ; un contexte écrit doit encore être assemblé ;
un délai de tests ne borne pas la compilation ; la provenance doit porter la
vue réellement compilée. Les contrôles par les outils ne constituent pas une
supervision Swarm autonome ni une revue indépendante.

## Travaux ouverts et condition de suite

F01–F07, les scénarios IDA, IA4, IA5 et le RETEX global ne sont pas clos. Restent
l'enrôlement explicite/protégé, son administration, son témoin et sa révocation,
le reçu durable d'émission, le watermark indépendant du plan, l'appartenance et
le contenu résolu d'une création, l'origine attestée de chaque délégation,
l'approbation distincte hors plan, l'admission
commune de tous les lanceurs, la consommation/départ unique et sa réconciliation,
ainsi que les droits mémoire. Les protections contre le même UID hôte ne sont
pas acquises par des champs Go privés.

Codex porte le raccord et les preuves ; l'opérateur doit désigner l'identité
humaine initiale (issuer/sujet), la frontière du contrôleur et son enrôlement réel.
Aucun premier appelant, rôle Admin ou cockpit partagé ne peut les remplacer.
Le défaut de fixture de reprise de revue précédemment reproduit sur la base
initiale reste ouvert, propriétaire moteur/reprise ; ses trois fichiers d'essai
rejetés restent inchangés. Il n'est ni rejoué sans précondition nouvelle ni masqué
par ces résultats. La limite de timing sous charge reste séparée.
