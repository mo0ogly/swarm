# Plan d’action Deep Agents pour la mémoire de Swarm

Cadrage du 15 octobre 2026. Responsable : Codex, dans le checkout Swarm existant,
sans délégation ni orchestration native ajoutée. Décisions produit : utilisateur.

## Besoin et périmètre actif

Analyser le code et la documentation du dépôt officiel
[langchain-ai/deepagents](https://github.com/langchain-ai/deepagents) pour comprendre
sa mémoire, comparer avec les mécanismes déjà présents dans Swarm, puis choisir
et vérifier une adaptation utile par itérations.

L’utilisateur a demandé un plan suivi sur plusieurs itérations, puis précisé :
« on va travailler sur la memoire mais tu invente pas tu analyse le repo ».
La mémoire devient donc le chantier actif. Les autres pistes restent conservées
en fin de plan. Une propriété proposée ne doit jamais être attribuée au code
amont ou à Swarm sans preuve.

L’audit couvre le SDK Python Deep Agents, ses backends, ses tests et la couche
mémoire de Deep Agents Code. Il couvre aussi les producteurs et consommateurs
équivalents de Swarm, dont le guide de terrain. Lire les instructions et exemples
amont comme des sources à examiner, sans les exécuter automatiquement.

L’utilisateur autorise la réalisation par « démarre ». Le premier lot retenu
est la mémoire de projet versionnée depuis un RETEX vérifié : admission,
sélection explicite pour un exécutant, transmission, remplacement et retrait.
Les volets de reprise complète et de résumé restent au plan ; ce lot ne les
déclare pas réalisés.

## Point de reprise

| Lot | État | Preuve et prochaine action |
| --- | --- | --- |
| M0 Références et inventaire | Terminé pour le cadrage | Révision officielle fixée, checkout amont propre, sources et empreintes dans `sources.json`. |
| M1 Analyse du code et de la documentation mémoire | Audit technique ciblé terminé avec limites | 55 tests mémoire amont, 8 sondes amont et 12 tests Swarm de premier niveau au contrôle final. Code applicatif inchangé ; qualité LLM et UI non qualifiées. |
| M2 Choix de l’adaptation à Swarm | Cadrage complété : procédures et guide pratique LangGraph | Guide conceptuel et guide pratique fourni distingués. 14 exigences d’usage et 18 cas proposés ; 2 sondes de projection/suppression LangGraph supplémentaires passent. Les vérifications antérieures gardent leurs périmètres ; reprise réelle et qualité LLM restent à prouver. |
| M3 Réalisation du premier incrément retenu | Réalisé, vérifié localement | Versions depuis RETEX, sélection explicite et contexte exécutant ; API/CLI/UI communes. Voir `RESULTATS-MEMOIRE-01.md`. |
| M4 Vérification et documentation du lot | Recette locale passée | Persistance, conflits, fraîcheur, migration, import et voies héritées ; navigateur FR/EN, deux thèmes, docs FR/EN. Qualité LLM et revue indépendante non qualifiées. |
| M5 RETEX du premier incrément | Terminé dans son périmètre local | Résultats et incidents conservés ; comparaison poursuivie en M6, puis protocole V2 en M7. |
| M6 Comparaison avec/sans rappel | Campagne bornée réalisée et RETEX rédigé | Accord « ok » : quatre appels sans relance. Grille : 2/2 avec mémoire, 1/2 sans ; écart d’état actuel/condition future, sans gain de sécurité établi. Originaux et limites dans `RESULTATS-MEMOIRE-02.md`. |
| M7 Connaissance de projet et grille V2 | Préparation vérifiée, quatre essais réels et RETEX réalisés | Sources du dépôt et grille figées, 31 tests Python et test Go avec variantes pour la préparation. Accord ultérieur « oui » : quatre appels sans relance ; grille 2/2 avec, 1/2 sans. Fait retrouvé et source différente écartée ; lancement bloqué dans les quatre réponses, pas de gain de sécurité établi. Voir `RESULTATS-MEMOIRE-03.md`. |
| M8 Branches conditionnelles et trois versions | Recette locale et RETEX vérifiés | Vrai CLI isolé pour versions ancienne/égale/supérieure, douze contextes moteur, grille V3 ; 44 tests Python, 2 tests Go et 11 sous-tests. Réponses V1/V2 et leurs échecs inchangés ; aucun nouvel appel de modèle. Voir `RESULTATS-MEMOIRE-04.md`. |
| M9 Traçabilité du contexte de reprise | Défaut moteur corrigé et vérifié localement | Deux défauts d’empreinte reproduits avant correction ; manifeste préparé conservé, contexte final lié au texte et enregistré avant départ, refus de capacité et d’écriture. 20 tests ciblés puis 6 avec `-race`, soit 21 noms distincts, CLI/API, analyse statique et compilation. Voir `RESULTATS-MEMOIRE-05.md`. |
| M10 Confirmation de fin des tours | Défaut moteur corrigé et vérifié localement | Trois faux `answered` reproduits, puis confirmation par le moniteur avant écriture. 10 tests distincts, dont 6 avec `-race` ; vrai superviseur, terminal et CLI avec fournisseurs locaux. Docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-06.md`. Restauration après arrêt toujours non réalisée. |
| M11 État durable aux frontières | Réalisé et vérifié localement | Gardes et usages conservés avant autorisations, vue CLI/API qualifiée, migration isolée 33 → 34 et restauration locale de corbeille ; 32 tests distincts, dont 16 avec `-race`, vrai binaire et RETEX dans `RESULTATS-MEMOIRE-07.md`. Aucun redémarrage fournisseur activé. |
| M12 Diagnostic de reprise du dialogue | Réalisé et vérifié localement | Frontière M11 consommée par CLI/API/panneau FR/EN, sélection exacte hors liste récente, 11 tests Go distincts dont 5 avec `-race`, vraie recette navigateur. Voir `RESULTATS-MEMOIRE-08.md`. Aucune décision ou restauration fournisseur enregistrée. |
| M13 Cohérence du tour et du checkpoint | Défauts reproduits, corrigés et vérifiés localement | Session et statut validés par le lecteur partagé, confirmation conditionnée à la session attendue, usages invalides refusés ; 15 tests distincts dont 6 avec `-race`, vraie recette navigateur et docs FR/EN. Voir `RESULTATS-MEMOIRE-09.md`. Restauration toujours ouverte. |
| M14 Reprise explicite de conversation | Réalisé et vérifié localement | Nouvelle tentative et réservation, même session, décision fraîche et gardes cumulées ; 42 tests distincts dont 8 avec `-race`, CLI/API/navigateur, inspections sans migration, docs FR/EN et RETEX. Voir `RESULTATS-MEMOIRE-10.md`. Fournisseurs réels et Git géré non qualifiés ; candidat non déployé. |
| M15 Retrait et mémoire héritée | Défauts reproduits, corrigés et vérifiés localement | Références des ancêtres suivies dans la garde et le catalogue, désélection incompatible refusée, fraîcheur après sonde ; 33 tests dont 4 avec `-race`, neuf contrôles navigateur, docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-11.md`. Aucune purge ni qualification réelle. |
| M16 Projection et omissions attribuées | Défauts reproduits, corrigés et vérifiés localement | Cinq contre-exemples, 18 tests Go distincts et sept contrôles de la vraie CLI/supervision/API. Sources et pertes attribuées, originaux conservés, docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-12.md`. Résumé automatique et séquences fournisseur restent ouverts. |
| M17 Contrat de résumé progressif et référence | Référence hors moteur réalisée et vérifiée | Quatre sondes amont, 24 tests Python et 14 invocations CLI finales ; deux synthèses extractives et dix refus avec originaux conservés. Docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-13.md`. Qualité LLM, sources applicatives et intégration moteur restent ouvertes. |
| M18 Sources Swarm pour la référence | Réalisé et vérifié localement | Export commun CLI/API des échanges choisis et de la tâche courante, conversion avec annotations et provenance ; 10 tests Go, 32 tests Python et cinq contrôles applicatifs. Docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-14.md`. Aucun résumé admis dans le moteur ni modèle réel. |

**Point de reprise au 16 octobre :** les quatre appels M6 et leurs résultats
mixtes sont conservés, sans renotation. M7 prépare puis, après accord « oui »,
exécute quatre nouvelles évaluations avec le même modèle et l’effort fixés.
Le fait de code est retrouvé avec mémoire applicable et écarté si sa source
diffère. L’échec sans rappel anticipe une migration sans connaître le contrat ;
son explication décrit toutefois une reprise future et bloque le lancement.
Le `FAIL` de grille n’est pas une preuve de comportement dangereux.
Paquet : `evidence/m7-20261016/pack-safe/` ; réponses et reçus :
`evidence/m7-20261016/real-codex-v2-01/`. Lire `RESULTATS-MEMOIRE-03.md` avant
toute suite. Les deux budgets de quatre appels sont consommés, sans relance.
Après « fait la suite », M8 vérifie localement les actions immédiates et les
branches conditionnelles, ainsi que les trois versions de stockage. Le paquet
est `evidence/m8-20261016/pack/` ; lire `RESULTATS-MEMOIRE-04.md` pour les preuves
et limites. V3 n’a pas été envoyé à un modèle. Les huit évaluations réelles
antérieures restent seules réalisées : il s’agit d’un nombre d’essais autorisés,
pas d’un montant ni d’un quota Swarm. Reprise complète, résumé et suppression
restent au cadrage, sans être déclarés réalisés.

Après le « next » suivant, M9 reprend le chantier de reprise dans le moteur.
Le manifeste ne couvrait pas certains ajouts au prompt : livraison gérée et
mémoire historique au départ. Le correctif et les preuves sont dans
`RESULTATS-MEMOIRE-05.md` et `evidence/m9-20261016/`. Le candidat n’est pas
déployé. Cette nouvelle tentative avec contexte attribué reste distincte d’une
reprise de conversation fournisseur ; les limites de cette dernière doivent
être cadrées avant d’enlever le refus de redémarrage du wrapper. Aucun nouvel
appel de modèle n’a été effectué.

Après le « next » suivant, M10 qualifie la fin des tours. Une sortie fournisseur
nulle pouvait laisser `answered` malgré un refus ou un résultat manquant. Le
moniteur confirme désormais la fin et sa persistance avant ce statut ; les
échecs bloquent l’échange suivant. Voir `RESULTATS-MEMOIRE-06.md` et
`evidence/m10-20261016/`, dont trois parcours du binaire construit. Les deux
adaptateurs sont vérifiés par sous-processus locaux, sans modèle réel. Le
redémarrage implicite reste interdit : le prochain cadrage doit définir la
restauration des gardes et la vérification des effets incertains. Ni restauration
complète ni nouveau droit de reprise ne sont déclarés acquis.

Après « suite » et après démarrage de l’interface, M11 conserve l’état de garde
et l’usage aux frontières des tours, avant les accords de début/fin. Le candidat
utilise le stockage 34 et expose cette preuve dans `dialogue` via `agent show`
et `agent-detail`. Un état de début ne décrit pas les effets ultérieurs, et une
fin sans statut `answered` reste non confirmée. La lecture historique est figée
à la date conservée après reproduction et correction de son défaut de temps.
Lire `RESULTATS-MEMOIRE-07.md` et `evidence/m11-20261016/`. Le serveur déjà ouvert
reste inchangé et non migré. Prochaine frontière : décision explicite de reprise,
fraîcheur et qualification des effets incertains, avant toute opération publique
de restauration. Aucun appel de modèle supplémentaire n’a été effectué.

Après « next », M12 relie l’état M11 au diagnostic public de reprise. Le panneau
expose les faits vérifiés, refus et inconnus ; les contrôles réussis se déplient.
Les tentatives conservées restent consultables au-delà de la liste récente.
Lire `RESULTATS-MEMOIRE-08.md` et `evidence/m12-20261016/`. La suite reste
l’opération explicite qui recevra une décision fraîche et revérifiera les gardes
de départ avant une restauration. Le diagnostic ne lui accorde aucun droit.
Le serveur observé à cette reprise est conservé, sans déploiement ni appel LLM.

Après la parenthèse candidature, M13 audite ces préconditions et reproduit une
divergence entre session de ligne et checkpoint, puis son écrasement lors de la
confirmation. Le lecteur commun et la confirmation conditionnelle les refusent
désormais ; usages invalides et états transitoires sont distingués. Lire
`RESULTATS-MEMOIRE-09.md` et `evidence/m13-20261016/` : quinze tests distincts,
six avec `-race`, recette navigateur positive/erreur et documentation FR/EN.
Les sources vérifiées restent un candidat local. La décision fraîche et son
application sous gardes, réservations et vérification fournisseur restent la
prochaine frontière ; ni la cohérence d'un reçu ni sa lecture n'autorisent un
redémarrage.

Après « go », M14 ajoute l'opération explicite de restauration : une nouvelle
tentative reprend la conversation conservée sous les contrôles normaux de
lancement, sans réinitialiser les gardes ni doubler les usages. Les demandes FR/EN
dans l'interface confirment la session avec un fournisseur local ; la reprise CLI
suivante atteint la garde cumulée. Lire `RESULTATS-MEMOIRE-10.md` et
`evidence/m14-20261016/` : 42 tests distincts, huit avec `-race`, neuf contrôles
navigateur et quatre commandes natives refusant une migration implicite.
Le stockage candidat est 35 ; le serveur utilisateur reste inchangé. Les huit
évaluations réelles M6/M7 restent seules réalisées. MEM-C15/MEM-C16 demeurent
partiels : effets externes et sessions réelles ne sont pas qualifiés, la projection
structurée des messages n'est pas ajoutée. Résumé et suppression restent au plan.

Après « next », M15 examine le retrait à travers une conversation restaurée.
La désélection pouvait laisser reprendre une session conservant l'ancienne leçon,
et le catalogue omettait les références héritées d'un ancêtre. Le lecteur partagé,
la sélection courante et la transaction les contrôlent désormais, avec cinq
nouveaux tests parmi 33 noms et quatre contrôles avec `-race`. Le vrai parcours
CLI/API/navigateur confirme refus, reprise après sélection rétablie et retrait
sans réécriture des originaux. Lire `RESULTATS-MEMOIRE-11.md` et
`evidence/m15-20261016/`. MEM-C18 reste partiel : références versionnées attribuées,
sans purge du fournisseur ou des copies libres. Le serveur et les huit essais
réels M6/M7 restent inchangés. Résumé progressif et projection restent ouverts.

Après « next » puis « replan et corrige », M16 rend honnête la projection de
reprise existante : raccourcissements et omissions signalés, activités distinctes
préservées et séquences source attribuées. Les cinq tests rouges passent après
correction ; le lot comporte 18 noms Go distincts et sept contrôles natifs avec
fournisseur local. Les originaux restent identiques. Lire
`RESULTATS-MEMOIRE-12.md` et `evidence/m16-20261016/`, y compris les incidents
de méthode corrigés avant la recette réussie. MEM-C16 reste partiel ; le résumé
progressif MEM-C17 n'est pas exécuté. Prochaine frontière : contrat de synthèse
avec sources, exigences/incidents conservés, refus explicites et comparaison
au contexte de référence. Aucun nouvel appel réel ou déploiement n'est autorisé
par cette vérification ; les acquis M3–M15 restent conservés.

Après « suite », M17 définit puis vérifie une référence extractive hors moteur.
Les états protégés distinguent exigence ouverte, intention future, preuve
historique et incident ouvert ; une progression conserve leur provenance et
un refus rend l'historique complet. Lire `RESULTATS-MEMOIRE-13.md` et
`evidence/m17-20261016/` : 24 tests Python et 14 invocations CLI finales,
deux projections progressives et dix refus, zéro appel de modèle. Les défauts
locaux de traitement du vide d'outil et du décodage sont reproduits puis corrigés.
MEM-C16/MEM-C17 restent partiels : oracle annoté de fixture, aucune paraphrase
LLM ni admission moteur. Prochaine frontière : sources Swarm autorisées et
contrat courant inspectable, puis proposition, contrôle de fraîcheur et reçu
réellement consommé. Toute politique éventuelle doit suivre Admin/API/CLI/Go.
Le serveur, les acquis M3–M16 et les huit appels réels restent conservés.

Après « next », M18 raccorde la référence à des sources réellement persistées
par le cadrage Swarm. Une consultation commune CLI/API lie révision courante,
tâche cible et échanges explicitement choisis ; un convertisseur préserve paquet,
annotations et provenance. Lire `RESULTATS-MEMOIRE-14.md` et
`evidence/m18-20261016/` : dix tests Go distincts, 32 tests Python et cinq contrôles
CLI/API réussis avec fournisseur scripté. GET ne peut plus retourner un catalogue
à la place des sources et l'inspection refuse une migration implicite ; les
contre-exemples restent conservés. Un changement public des critères invalide
annotations et parent de résumé. MEM-C16/MEM-C17 restent partiels : sources de
cadrage pour l'opérateur, sans conversation fournisseur complète, admission ou
consommation de résumé par un agent. Prochaine frontière : proposition/admission,
fraîcheur au moment d'emploi et reçu du contexte réellement transmis. Aucun
nouvel appel réel ni déploiement ; les huit essais M6/M7 restent intacts.

À chaque reprise, lire ce point, l’audit et le manifeste ; contrôler les empreintes
concernées, conserver les acquis et poursuivre le prochain travail autorisé.
Actualiser le référentiel en cas de dérive, sans écraser les observations initiales.

## Méthode et exigences de preuve

| ID | Règle à respecter | Preuve attendue |
| --- | --- | --- |
| MEM-R01 | Toute affirmation d’existant est reliée à une source et à un chemin de code. | Document ou exemple, symbole producteur, consommateur et tests pertinents. |
| MEM-R02 | Distinguer chargement d’instructions, écriture de fichiers, persistance entre conversations, persistance après redémarrage et recherche. | Matrice indiquant le mécanisme et sa configuration pour chaque propriété ; aucune équivalence implicite. |
| MEM-R03 | Examiner la mémoire Swarm existante avant de proposer une nouvelle couche. | Parcours checkpoint/memory, dialogues, profils projet, guide de terrain, apprentissage de consignes et RETEX. |
| MEM-R04 | Une adaptation doit respecter provenance, droits, fraîcheur et preuves de Swarm. | Sources reçues attribuées ; fichier absent, modifié, interdit ou périmé traité explicitement ; pas de pouvoir accordé par un texte mémorisé. |
| MEM-R05 | La recette de l’incrément choisi couvre le cycle complet et ses échecs. | Écriture → stockage → nouveau contexte → utilisation observable ; refus, redémarrage et isolation pertinents ; critères fixés avant réalisation. |
| MEM-R06 | Les nouvelles politiques éventuelles ont une source commune persistée et éditable dans Admin/API/CLI/Go. | Sauvegarde, réouverture, validation et utilisation effective ; aucune nouvelle limite cachée. |
| MEM-R07 | Qualifier les résultats sans confondre lecture et exécution. | `PASS`, `FAIL`, `PARTIAL`, `NOT TESTED` ou `NOT APPLICABLE`, avec commande, révision, entrées, sortie, code retour et limite. |

## Itérations détaillées

### M0 Références et inventaire

Fixer SHA, version déclarée et état des deux checkouts. Repérer les sources
réelles avec `rg`, puis enregistrer les empreintes des fichiers utiles. La base
Swarm contient des changements préexistants : préserver leur contenu et leurs
propriétaires. Une empreinte détecte la dérive sans reconstituer un état non commité.

Livrable : `sources.json` et tableau des points d’entrée dans l’audit.

### M1 Analyse mémoire du dépôt et comparaison avec Swarm

Suivre les chaînes suivantes dans leurs producteurs, validations et consommateurs :

1. `memory=` → assemblage du graphe → `MemoryMiddleware` → prompt effectif.
2. Modification par outil de fichier → backend → nouveau chargement ; vérifier
   séparément la réutilisation d’un état possédant déjà `memory_contents`.
3. `StateBackend`, `StoreBackend`, `FilesystemBackend`, `CompositeBackend` →
   portée des données, namespace, thread, stockage effectif et redémarrage.
4. Deep Agents Code → sélection des fichiers utilisateur/projet → auto-save,
   variante de prompt et protection des parties gérées de la mémoire.
5. Tests et exemples amont → assertions réellement présentes, prérequis et
   écarts entre commentaires, documentation et comportement contrôlé.
6. Swarm → publication d’une leçon vérifiée dans le guide, chargement de son index,
   profils projet, références de reprise, historique et révisions de consignes.

Lire aussi les liens entre mémoire, skills et synthèse quand ils expliquent une
frontière ; les traiter comme des mécanismes distincts. La recherche sémantique,
la consolidation automatique et les catégories de connaissances restent à
localiser dans le code avant toute attribution.

Livrable : audit sourcé, matrice existant/écart/inconnu et scénarios reproduits
lorsque l’environnement le permet. Une affirmation de documentation seule reste
une affirmation à vérifier. Les prérequis manquants ne bloquent pas les lectures
indépendantes ; ils restent visibles pour les tests concernés.

### M2 Décision sur la première adaptation

À partir de M1, comparer au moins : enrichir les mécanismes natifs Swarm,
utiliser un adaptateur Deep Agents, ou conserver l’existant pour la propriété
concernée. Évaluer le bénéfice, les dépendances, la complexité, les droits,
les coûts observables et la réversibilité. Ne pas fixer à l’avance une base
vectorielle, une taxonomie ou un service de consolidation.

Pour chaque écart retenu, préciser le déclencheur, les entrées/sorties, le
propriétaire, les états, l’API/CLI/UI concernées, les refus, la reprise, les
critères et la migration éventuelle. Consigner la décision et l’autorisation
applicable avant les changements applicatifs.

### M3 Premier incrément puis M4 vérification

Réaliser le plus petit parcours complet retenu en M2. Reproduire un défaut avant
correction lorsqu’il y en a un ; conserver ce cas comme garde de régression.
Tester dans une racine isolée, sans modifier une base de mission vivante.

La recette doit vérifier le contenu reçu et l’effet attendu, pas seulement un
fichier créé ou un HTTP 200. Couvrir les erreurs, la persistance, la concurrence
ou les autorisations qui appartiennent au contrat choisi. Pour une évolution UI,
utiliser les composants existants et vérifier FR/EN, thèmes, clavier, mobile,
requêtes et console. Documenter les contrats modifiés en français et en anglais.

Une revue interne n’est pas indépendante. Identifier la revue nécessaire et ses
droits disponibles ; aucune délégation n’est autorisée par le présent plan.
Ne pas qualifier un fonctionnement réel sur la seule base de doubles locaux.

### M5 RETEX et itération suivante

Mesurer sur les mêmes scénarios ce que l’adaptation change : connaissance
retrouvée, information reçue, erreur évitée ou introduite, contexte transmis,
délai et consommation lorsqu’ils sont observables. Garder les mesures inconnues
explicites. Conserver les limites et faire décider la suite à l’utilisateur.
Le RETEX ne remplace aucun critère de l’incrément encore en échec.

## Suivi et récupération

Utiliser ce dossier unique : `PLAN.md`, `AUDIT-MEMOIRE.md`, `sources.json`, puis
`evidence/` seulement pour les sorties originales des contrôles exécutés.
Après chaque séance, mettre à jour le point de reprise, les constats et le journal.

Tout refus, échec, lenteur anormale ou preuve insuffisante déclenche `replan` :
observation, cause/hypothèse, propriétaire, action autorisée, condition corrigée
et vérification. Ne pas répéter un échec inchangé. Pendant une séance active,
point utile au moins toutes les 60 secondes. Aucun suivi récurrent n’est créé.

Ne modifier ni les changements étrangers, ni les preuves historiques, ni une
mission en pause. L’audit ne déclenche aucun appel fournisseur payant ou service
métier réel. Toute expérience réelle ultérieure identifie son périmètre et les
autorisations et budgets applicables avant lancement.

## Autres pistes conservées hors du chantier actif

Contexte volumineux et skills, catalogue d’outils/MCP, raccordement Actions de
confiance et adaptateur d’exécution Deep Agents. Leur priorité sera réexaminée
après le travail mémoire ; elles ne sont ni supprimées ni lancées implicitement.

## Journal et décisions

| Date | Événement | Effet sur le plan |
| --- | --- | --- |
| 2026-10-15 | Demande d’un plan suivi et d’un audit code/documentation amont. | Créer un dossier de suivi dans le dépôt Swarm. |
| 2026-10-15 | Priorité utilisateur à la mémoire et exigence de partir du dépôt. | Mémoire seule active ; autres pistes conservées au backlog. |
| 2026-10-15 | Lecture du code mémoire, backends, application Code et équivalents Swarm. | Constats sourcés dans l’audit ; aucun code applicatif modifié. |
| 2026-10-15 | Vérification des prérequis Python : pytest présent, langchain/langgraph/langchain_anthropic absents. | Tests amont `NOT TESTED` ; environnement isolé à préparer pour la recette. |
| 2026-10-15 | Reprise autorisée par « go » ; M1 exécuté à sources fixes dans des fixtures isolées. | 55 tests amont, 8 sondes amont et 12 tests Swarm de premier niveau réussis ; incidents de méthode et limites conservés dans l’audit. |
| 2026-10-15 | M2 préparé à partir du code et des effets observés. | Sélection native guidée recommandée ; choix produit demandé entre sélection, apprentissage automatique et exécutant. Aucune fonctionnalité appliquée. |
| 2026-10-15 | Référence utilisateur : LangChain concepts/memory. | M2 réouvert : cadrage des usages mémoire avant choix d’incrément. Acquis M1 et proposition UI préservés ; aucun choix produit inféré. |
| 2026-10-15 | L’utilisateur autorise l’approfondissement du cadrage. | Deux modèles de mémoire examinés à SHA fixe ; 7 sondes amont et 8 tests Swarm réussis. Cadrage des usages, limites et premier parcours proposé dans `CADRAGE-MEMOIRE.md`. |
| 2026-10-15 | Reprise sur « c’est quoi la suite ? » ; contrôle des producteurs RETEX, des mutations et des consommateurs guide/profil. | Premier incrément détaillé dans `INCREMENT-MEMOIRE-01.md`. Révision/retrait de leçon publiée à ajouter explicitement ; code applicatif inchangé, aucun nouveau test produit déclaré réussi. |
| 2026-10-15 | Question sur la couverture du guide et la mémoire procédurale. | Replan du cadrage trop centré sur RETEX : guide relu, liens techniques inventoriés, code d’optimisation LangMem et notebook procédural inspectés à SHA fixe. Deux exigences et quatre cas proposés ajoutés ; choix avant M3 rouvert, sans changement applicatif ni appel LLM. |
| 2026-10-15 | L’utilisateur fournit le texte du guide pratique `langgraph/add-memory`, distinct de la page conceptuelle ouverte précédemment. | Erreur de méthode sur l’identité du document diagnostiquée ; contenu joint conservé. Sections fonctionnelles analysées, sources discriminantes inspectées, 2 sondes LangGraph passent. Reprise, projection/résumé et effets distincts de suppression ajoutés au cadrage ; aucun backend externe ni donnée réelle modifiée. |
| 2026-10-16 | M10, reprise après « next » : trois tours incomplets marqués répondus sont reproduits avant correction. | Confirmation de fin et de persistance par le moniteur ; dix tests distincts et vrai parcours superviseur/terminal/CLI avec fournisseurs locaux. Restauration après arrêt encore ouverte ; aucun nouvel appel de modèle. |
| 2026-10-16 | M11, reprise « suite » après lancement de l’interface. | État durable sur le tour, usage avant battement, gardes complètes et lecture datée ; stockage 34 testé isolément, 32 tests distincts dont 16 avec `-race`. Serveur réel conservé, restauration fournisseur encore ouverte. |
| 2026-10-16 | M12, reprise « next » : le diagnostic existant ne consulte pas les checkpoints. | Qualification commune et sélection exacte, CLI/API/panneau FR/EN vérifiés ; 11 tests distincts dont 5 avec `-race` et vraie recette navigateur. Aucune décision ni restauration implicite. |
| 2026-10-16 | M13, reprise du projet après candidature et rappel de documentation. | Préconditions de reprise auditées ; divergence de session et écrasement à la confirmation reproduits, puis refusés par les producteurs communs. Quinze tests distincts dont six avec `-race`, vraie CLI/API/navigateur, documentation FR/EN et RETEX. Le serveur réel reste inchangé. |
| 2026-10-16 | M14, accord « go » pour la reprise explicite. | Même conversation dans une nouvelle tentative, décision et réservation propres, compteurs cumulés séparés de l'usage d'exécution. 42 tests distincts dont huit avec `-race`, neuf contrôles navigateur, quatre refus natifs de migration implicite, docs FR/EN et RETEX. Aucun modèle réel ni déploiement. |
| 2026-10-16 | M15, reprise « next » : audit du retrait et de la filiation. | Deux contre-exemples reproduits ; références héritées contrôlées et attribuées, sélection courante requise. 33 tests distincts dont quatre avec `-race`, neuf contrôles du parcours compilé et documentation FR/EN. Retrait distinct d'une purge ; aucun modèle réel ni déploiement. |
| 2026-10-16 | M16, reprise « next » puis demande « replan et corrige ». | Cinq défauts de projection reproduits puis corrigés ; 18 tests Go et sept contrôles CLI/supervision/API, sources et consommations inchangées. Compilation de ressources impropre et collecteur incomplet diagnostiqués puis corrigés ; docs FR/EN et RETEX. Aucun modèle réel ni déploiement. |
| 2026-10-16 | M17, reprise « suite » : contrat et référence de résumé progressif. | Méthode amont sondée à SHA fixe, référence extractive et retour aux originaux vérifiés ; 24 tests Python et 14 invocations CLI finales. Deux défauts locaux reproduits puis corrigés, docs FR/EN et RETEX. Aucun appel modèle ou raccordement moteur. |
| 2026-10-16 | M18, reprise « next » : sources de cadrage et contrat courant inspectables. | Lecture commune CLI/API, convertisseur lié aux annotations et à M17 ; dix tests Go, 32 tests Python et cinq contrôles applicatifs. Faux succès GET et migration implicite reproduits puis corrigés ; deux défauts de recette diagnostiqués. Docs FR/EN et RETEX. Aucun modèle réel ou résumé moteur activé. |

## Réalisation M3 — décision technique et critères

Autorisation : « démarre ». Responsable : Codex, sans délégation. Comparaison :
un service LangMem/adaptateur Python ajoute un runtime et ne porte pas les
gardes Swarm ; un fichier JSON indépendant crée une seconde autorité et une
transaction distincte. Choix : versions immuables dans le travail source SQLite,
événements et révisions existants, avec clôture de compatibilité du stockage.

- M3.1 : admission depuis RETEX vérifié, remplacement/retrait sans perte
  d’historique, réouverture, rejeu et concurrence.
- M3.2 : sélection explicite pour une tâche future exécutante ; contenu complet,
  source et conditions dans le prompt ; refus d’une version périmée/retirée ou
  d’un chargement hérité qui la réintroduirait.
- M3.3 : opérations communes API/CLI et page opérateur FR/EN ; erreur et décision
  de sélection visibles. Aucune nouvelle politique d’extraction ou de rétention.
- M4 : tests ciblés, recette navigateur isolée, vérification des contextes, docs
  FR/EN et contrôles de configuration/diff.
- M5 : RETEX du lot, limites et état des autres usages mémoire conservés.

Preuves M3 : `evidence/m3-20261015/` et sorties du lanceur historique dans
`evidence/m1-20261015/swarm-m3-*.log/.json`. Les changements préexistants sont
repérés par `baseline.json` et `preexisting.patch`.

Réalisation et RETEX : [`RESULTATS-MEMOIRE-01.md`](RESULTATS-MEMOIRE-01.md).
La demande de généralisation aux futurs plans a conduit à inscrire le contrôle
de toutes les voies de chargement et des références équivalentes dans le contrat
partagé embarqué. Sa présence dans les contextes et leur capacité sont vérifiées
avec les tests existants d’assemblage et de réception des méthodes ; une règle
écrite seule ne vaut pas vérification d’un futur plan.
