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
| M19 Sélection extractive pour un lancement explicite | Réalisé et vérifié localement | Aperçu et confirmation CLI/API, citations reconstruites dans le moteur et sources revérifiées avant départ ; 21 tests Go dont deux avec `-race`, 49 tests Python, cinq contrôles applicatifs M19 et cinq M18. Docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-15.md`. Qualité LLM, hiérarchie et progression générale restent ouvertes. |
| M20 Propriété et droits du cadrage | Replan : identité et autorité préalables à l’extension hiérarchique | Attribution technique distinguée de la responsabilité et des droits ; ordre révisé dans `INCREMENT-MEMOIRE-16.md`, lié à R01/R13 et aux 18 scénarios IDA de `../actions-confiance/identites-agents-replan.md`. Création hors plan bloquée avec explication et décision humaine : exigence à réaliser, aucun code M20 ni partage activé. |
| M21 Liaison des citations et références | Correction vérifiée dans la préparation mémoire | Alias et doublons reproduits puis refusés par le validateur partagé, y compris le choix extractif ; 57 tests Python et parcours source → référence → worker scripté sur binaire M19 conservé. Voir `INCREMENT-MEMOIRE-17.md` et `RESULTATS-MEMOIRE-17.md`. M20/IA2–IA3 restent ouverts ; aucune identité ou autorisation déduite des empreintes. |
| M22 Cachets de contexte et repères | Composant et démonstration isolée vérifiés | Direction « cachet de cire au bas d'un parchemin », 29 repères FR/EN et descriptions, états inconnus par défaut. API réutilisable pour la session identité ; deux thèmes, cinq largeurs et contrastes vérifiés. Voir `RESULTATS-MEMOIRE-18.md` ; intégration aux écrans réels et droits moteur encore distincts. |
| M23 Cachets dans la page mémoire | Intégration frontend vérifiée sur API conservée | Deux types et transitions réelles, identités/droits inconnus, 11 groupes de contrôle et 20 configurations FR/EN/thèmes/largeurs. Assets générés communs ; voir `RESULTATS-MEMOIRE-19.md` pour l'embarquement, les preuves et la limite du serveur non redémarré. |
| M24 Liaison des usages du résumé | Vérification locale sur vue identité figée | 34 noms PASS dont 32 du nouveau contrat ; replan après changement concurrent des entrées. Ressources exactes et demandes séparées sur les sources réelles M19, contenu/sélection/destinataire liés ; aucun raccord public ni identité fabriquée. Statique/compilation et périmètre de preuve dans `RESULTATS-MEMOIRE-20.md`. M20.2 partiel, M20.3/4 ouverts. |
| M25 Cloisonnement des sources et verdict complet | Vérifié localement, aucun raccord public | Alias v1 entre snapshots de tâches reproduit puis corrigé en v2 ; droits des deux acteurs vérifiés ensemble, aucun accord partiel. 12 tests principaux et 60 sous-cas PASS, watchdog global désactivé, docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-21.md`. Autorité protégée, affectation, propriété/auteur et engagement courant restent ouverts. |
| M26 Reçu extractif au départ | Correction vérifiée localement dans le parcours existant | Reçu altéré/supprimé et fausse observation préparée reproduits puis refusés avant fournisseur. Demande confirmée, octets reconstruits, contexte et métadonnées contrôlés ; échec d’écriture sans fausse estampille. 81 noms du parcours final PASS et 33 sous race, 92 gardes source/usages réutilisées. Docs FR/EN et RETEX : `RESULTATS-MEMOIRE-22.md`. Identité et engagement atomique restent ouverts ; serveur non redémarré. |
| M27 Lignes source de mémoire versionnée | Correction vérifiée localement dans les trois voies mémoire | ID/schéma/révision/UTF-8 incohérents reproduits puis refusés par le lecteur partagé ; catalogue CLI/API sans succès partiel, guide en flux et transactions conservés. 30 tests principaux et 33 sous-cas PASS, 21 noms sous race ; statique/compilation, archives de fixture, docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-23.md`. Lecteurs globaux du Store et conformité identité restent ouverts. |
| M28 Lecteurs généraux et mutations communes | Correction vérifiée localement sur le contrat partagé | Noyau Work extrait puis utilisé par get/list, mutation/rejeu et wrappers mémoire ; refus avant callbacks sans succès partiel. 56 noms PASS normal, 32 sous race et une garde de version de stockage, avec recouvrement ; vet/compilation, inventaire corrigé, docs FR/EN et RETEX dans `RESULTATS-MEMOIRE-24.md`. Autres SQL directs, propagation dans chaque appelant et critères publics M20 restent ouverts. |

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

Après « ok next », M19 admet un choix extractif explicite pour le premier
lancement manuel automatisé d'un worker hors hiérarchie. Le moteur reconstruit
les textes, exige l'empreinte du prompt et du choix, puis conserve le contrôle
des sources juste avant démarrage. L'instantané opérateur n'est pas injecté et
le profil ne reprend pas ce choix. Lire `RESULTATS-MEMOIRE-15.md` et
`evidence/m19-20261016/` : 21 tests Go, dont deux avec `-race`, 49 tests Python,
cinq contrôles M19 et cinq M18 par la vraie CLI/API, fournisseur scripté.
Deux défauts de candidat et les erreurs de méthode sont reproduits ou conservés,
puis corrigés et vérifiés. MEM-C16/MEM-C17 restent partiels : autres portées,
progression générale, états évolutifs et qualité LLM encore ouverts. Aucun nouvel
appel réel ou déploiement ; serveur et évaluations M6/M7 conservés.

Après « la suite ? », la question sur le propriétaire des échanges corrige la
priorité M20 : attribution et droits du cadrage avant extension hiérarchique.
`Task.Owner` est le fournisseur au lancement et la réponse référence l'agent ;
ces faits ne prouvent pas une responsabilité métier ou un droit de transmission.
Voir `INCREMENT-MEMOIRE-16.md` : audit initial, proposition de contrat et critères
restant à réaliser. Ne pas inférer un propriétaire pour l'historique ni fabriquer
un dialogue hiérarchique dans une fixture privée. M19 reste inchangé et vérifié ;
aucune attribution, migration ou ouverture de partage n'est appliquée par ce replan.

Après ouverture par l'utilisateur d'une session identité dédiée, M21 reprend un
travail mémoire indépendant. IA1 a reproduit sept défauts sans corriger les
contrôles ; les sources communes restent en lecture seule dans ce lot. Un
défaut du convertisseur, qui acceptait les identifiants de citation renommés et
les échanges dupliqués, est reproduit puis corrigé au validateur partagé avec
le préparateur de choix. Les 57 tests Python passent ; la chaîne CLI/API réelle
passe cinq contrôles M19 et cinq M18 avec trois invocations scriptées sur le
binaire M19 conservé, sans modèle réel. Voir `RESULTATS-MEMOIRE-17.md` pour les
preuves et les limites. Les identités, propriétaires et droits restent à
appliquer avec le contrat IA2/IA3 ; M21 n'ouvre aucun partage supplémentaire.

Reprise du 17 octobre après les avancées identité : le noyau privé, le registre
durable, le canal d'instance et la fraîcheur du plan permettent de préparer M20.2
sur des types réels. Le [contrat d'usages](CONTRAT-USAGES-MEMOIRE.md) conserve dix
critères ouverts et deux contraintes : garde du contenu distincte du plan,
autorité explicite pour le cadrage avant adoption d'un plan. IA3b est active ;
pas de modification des sources communes ni d'activation mémoire implicite.
Le statut des opérations publiques reste inchangé ; M21 conserve ses résultats
à entrées Python identiques.

Demande suivante : ajouter des icônes démonstratives réutilisables par le chantier
identité, puis préciser leur forme comme un cachet de cire au bas d'un parchemin.
M22 livre le composant dans `frontend/signals/`, un catalogue interactif isolé,
29 repères et descriptions FR/EN. Le build reste hors checkout pour préserver
les ressources embarquées pendant IA3b. La recette navigateur vérifie états,
inconnus, texte non fiable, deux thèmes et cinq largeurs ; aucune activation de
droit ou intégration aux écrans existants n'en est déduite. Documentation partagée
et RETEX : `RESULTATS-MEMOIRE-18.md`.

Reprise suivante « merci la suite » : M23 intègre le composant à la page mémoire
et ses états aux métadonnées réellement fournies. La recette de l'ancienne page
échoue sur l'absence de cachet, puis onze groupes passent sur les cinq assets
candidats avec l'API M19 isolée conservée ; vingt configurations sont vérifiées.
Les ressources communes sont appliquées après la fin du processus Go observé.
La documentation FR/EN précise génération et réutilisation, sans attestation
inventée ni message à l'autre session. Résultat et RETEX :
`RESULTATS-MEMOIRE-19.md`. Serveur utilisateur non redémarré, moteur identité
courant non validé par cette recette. M20.2 et ses dix critères restent ouverts
jusqu'au raccord des autorités publiques, dont le cadrage avant plan.

Reprise « on fait la suite » : IA3b a vérifié son noyau humain, mais son admission
publique et l'autorité avant plan restent ouvertes. M24 réalise la liaison privée
de M20.2 sur le pipeline extractif M19 : sources/ressources exactes, sélection et
attachement reconstruits, portée/tâche/principal/instance destinataire liés.
Les demandes lecture/synthèse/transmission restent distinctes ; snapshot opérateur
et omissions ne sont pas transmissibles. Les 32 noms du nouveau contrat et deux
gardes CLI/API existantes passent. Le premier pack final conserve ses commandes
réussies mais n'a pas d'entrées stables : trois fichiers identité ont changé.
La vérification est reprise avec ces fichiers isolés dans une vue identité Go
figée (22 fichiers, sans copie complète du dépôt). Preuves, statique et RETEX dans
`RESULTATS-MEMOIRE-20.md`. Aucun contrôle mémoire public n'est encore activé.
Persistance propriétaire/auteur, autorité protégée du demandeur et du destinataire,
avant-plan, engagement et effet restent les préconditions de M20.3/4. Les fichiers
identité et le serveur utilisateur sont préservés.

Reprise « go next » avec la nouvelle règle sur les délais : M25 utilise
`go test -timeout=0`, mesure les entrées et suit la progression. Le chantier
identité a livré preuve et journal d'enrôlement isolés ; ses choix opérateur et
son raccord public restent ouverts. Côté mémoire, une collision de ressource
snapshot entre tâches est reproduite avant correction générale en v2. Le
verdict pur vérifie tous les droits demandeur et la lecture destinataire avant
de retourner un résultat complet, avec réévaluation intégrale au réemploi.
Une fixture illégale d'octroi est corrigée avec validation réelle de transition.
72 noms passent à sources stables, sans overlay ; voir `RESULTATS-MEMOIRE-21.md`.
La prochaine activation dépend toujours de l'autorité, des attributions et du
point d'engagement protégés ; aucun octroi ou effet n'est créé par ce résultat.

Reprise « next » : la décision IA3c.2f est désormais adoptée (pilote local sous
compte distinct, mode distant prévu) ; elle n'est pas redemandée. M26 renforce le
parcours existant pendant que l'autorité protégée reste en construction. Deux
corruptions de reçu démarrent le fournisseur dans les fixtures rouges ; après
correction, la demande originale, le reçu complet, les extraits reconstruits et
le préfixe confirmé doivent concorder avant départ. Une erreur d'écriture ne
marque plus l'appelant vérifié. La revue reproduit puis corrige aussi une fausse
observation du contexte préparé. Les preuves finales lient 81 noms PASS et
33 sous race aux sources stables ; 92 gardes source/usages restent réutilisables
à dépendances pertinentes inchangées. Voir `RESULTATS-MEMOIRE-22.md` et son audit.
Aucun auteur/propriétaire/droit n'est créé ; les dix conformités M20, l'autorité
avant plan et l'engagement/réception protégés restent ouverts.

Reprise « merci la suite maintenant » : M27 inspecte les lectures versionnées
pendant que le chantier identité termine sa configuration/checkpoint privés.
Le lecteur et le catalogue CLI acceptent une identité JSON différente de la clé
SQL ; six formes invalides sont reproduites. Résolution, catalogue et guide
partagent maintenant la lecture clé/révision/corps et refusent tout résultat
partiel. Une source invalide arrête le scan, sans effacer les originaux ni se
présenter comme une réparation. Le test d'archive initialement SKIP est exécuté
avec une destination isolée ; un diagnostic sans clé source est reproduit puis
corrigé sans exposer une clé invalide. 63 noms PASS normal et 21 sous race,
statique/compilation et archives vérifiées ; voir `RESULTATS-MEMOIRE-23.md`.
Les lecteurs globaux `Store.get/list` doivent être instruits avec leur périmètre
transversal avant extension ; cette découverte et sa condition de reprise sont
conservées dans le plan M27. L'attribution persistée, l'autorité protégée et les
dix conformités M20 restent ouverts ; aucun serveur n'est redémarré.

Reprise « suivant » : M28 généralise la validation de l'enveloppe SQL au Store
et aux lectures communes de mutation/rejeu. Les originaux rouges comprennent
un échec de compilation à sources mobiles (zéro test), puis onze noms FAIL
stables qui reproduisent les défauts. Après extraction, 56 noms passent en
normal (36 parents/20 sous-cas), 32 sous race (17/15), plus la garde de version
du rédacteur connecté. Les trois lots couvrent 58 noms distincts (38/20).
Sources stables, analyse statique et binaire isolé liés aux reçus. La revue
corrige aussi le filtre de notre inventaire : neuf requêtes workspace étaient
comptées par sous-chaîne ; 31 sites exacts initiaux et 27 après extraction,
168 sélecteurs syntaxiques get/list. Ce n'est ni un audit complet de tous les
lecteurs SQL ni une preuve d'identité. Contrats FR/EN, revue locale et RETEX :
`RESULTATS-MEMOIRE-24.md`, `evidence/m28-20261017/audit-final.json`.
Le service identité est désormais écrit dans le checkout partagé, sans
déploiement attesté par ce lot. Les critères M20-U01–U10, l'attribution
persistée, l'autorité avant plan et l'engagement courant restent ouverts.

Suite conservée après M28 : instruire les SQL directs restants et leurs appels
à partir de `callers-current.json`, en priorisant les sources mémoire et les
lectures avant effet. Propriétaire : chantier mémoire/Store, avec le chantier
identité pour ses propres lecteurs. Critère : refus d'une enveloppe incohérente
avant publication/effet, causes conservées, absence de mutation, reprise valide
et vérification des champs historiques. État : inventorié, pas déclaré corrigé.
L'analyse par module est nécessaire avant extension : certaines requêtes ne
lisent que des métadonnées, d'autres ont déjà leurs propres contrôles. Aucun
outil de réparation de base corrompue n'est ajouté ; conserver les originaux.

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
| 2026-10-16 | M19, reprise « ok next » : admission extractive et contexte reçu. | Choix explicite reconstruit par le moteur, aperçu confirmé CLI/API et contrôle avant sous-processus ; 21 tests Go, dont deux avec `-race`, 49 tests Python, cinq contrôles M19 et cinq M18. Confirmation du motif et garde des sources corrigées après reproduction ; docs FR/EN et RETEX. Zéro modèle réel, serveur conservé. |
| 2026-10-16 | M20, reprise « la suite ? », puis question sur la propriété des échanges. | Replan avant extension : distinguer responsable logique, auteur attesté et droits de transmission. `Task.Owner` désigne le fournisseur ; contrat de propriété à construire, anciens échanges non réattribués implicitement. Aucun code applicatif ou appel fournisseur. |
| 2026-10-16 | Exigence explicite de propriété/auteur/usages, puis scénarios d'usurpation et de création non autorisée, confirmés comme gestion des identités de l'application. | R01/R13 étendu aux agents : 18 scénarios IDA, refus par défaut, demande hors plan expliquée et décision humaine authentifiée sur le contenu exact, protection du contrôle et des données. Plan IA1–IA5 relié à M20 ; architecture FR/EN actualisée comme cible, garanties non déclarées implémentées. |
| 2026-10-16 | Reprise mémoire après lancement de la session identité dédiée. | M21 : deux familles de défauts de liaison source reproduites, puis refus commun dans préparation et choix. 57 tests Python, quatre vecteurs Go, recette CLI/API avec trois invocations scriptées et zéro modèle. Documentation FR/EN et RETEX ; M20 et autorisations IA2/IA3 restent ouverts. |

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
