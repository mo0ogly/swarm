# Qualification globale — préparation T8

État au 16 octobre 2026. La collecte eBPF réelle reste une exigence ouverte ;
le diagnostic indisponible ne la remplace pas. Aucun verdict global, revue
indépendante ou RETEX final n'est déclaré. Les preuves sont attribuées à leurs
sources, entrées, binaires et périmètres ; les campagnes historiques ne sont pas
additionnées à une campagne unique.

## Matrice R01–R20 et prochaine preuve

| Exigences | Fonction / preuve disponible | Limite de qualification |
| --- | --- | --- |
| R01, R13 | Identités, mandats délégués, step-up, contexte, groupes et introspection ; dossiers `verification-identites-20261015` et `verification-confinement-20261015/introspection-verification.json` | Émetteurs isolés ; fenêtre locale de révocation explicitée, aucune activation tierce. Consommateurs d'identité raccordés selon leur tranche. |
| R02–R06, R09 | Mandat, accord exact, Biba, révocation, effet unique et reprise ; 126 cas T2 rejoués dans `verification-ebpf-20261016/check-regression-race.out`, sept scénarios publics dans `check-pilot.out` | Deux connecteurs simulés ; aucun paiement ou droit externe réel. |
| R07, R16 | Preuves, signatures, témoin acquis séparément, archives non destructives ; `verification-preuves-20261016/archive-verification.json` | Vérification locale historique ; ni temps tiers ni stockage physiquement immuable contre le même UID. |
| R08 | Validation/persistance/aperçu/Admin de chaque politique dans les recettes de sa tranche ; `kernel_check` vérifié par CLI/API/UX dans `verification-ebpf-20261016/check-verification.json` | Les contrôles ciblés sont attribués, sans annoncer un rejeu global de toutes les politiques. |
| R10, R15 | Processus construisible séparément, dossiers versionnés et validations ; `verification-processus-20261015/process-verification.json` | Portée locale public/internal ; transition sans paiement implicite. |
| R11 | Parcours publics du pilote et connecteurs exécutés ; rejet sans mutation, reprise et concurrence en Go | Rassembler la recette de l'ensemble sur les entrées finales, puis revue indépendante applicable. Les reçus locaux ne constituent pas cette acceptation. |
| R12 | Documentation FR/EN et recettes de chaque tranche ; présente matrice | Inventaire final, recette globale et RETEX final ouverts. Ce document est une préparation. |
| R14 | Admission métier exacte et remise restreinte au helper sous listes actives ; `verification-sensitive-20261016/secret-data-verification.json` | Contrôle local vérifié ; providers non maîtrisés refusés, aucune qualification générale ni acceptation globale. Swap/traçage privilégié hors portée. |
| R17 | Laboratoire fixe versionné ; `verification-laboratoire-20261016/laboratory-verification.json` | Simulations, aucune preuve de collecte noyau ou d'isolement d'un fournisseur réel. |
| R18 | Deux connecteurs simulés distincts, SPI et réconciliation ; 29 cas natifs rejoués dans `verification-ebpf-20261016/check-connectors-race.out` | Aucun connecteur arbitraire ou effet extérieur. |
| R19 | Matrice NIST contextualisée v5, cinq reçus et 36 entrées vérifiés par `controls.py` | Intégrité du dossier, aucune baseline ni certification ; neuf partiels/deux non évalués/un hors périmètre. |
| R20 | Source du registre, collecteur explicite opt-in, réception/backend et diagnostic public ; `verification-ebpf-20261016/observe-verification.json` | Raccord public vérifié sur fixture possédée avec available:false/code3. EPERM avant attachement : noyau positif, exclusion étrangère réelle, pertes/reprise et surcoût ouverts. |

## Ordre de reprise et propriétaires

1. Codex : conserver le diagnostic fermé livré et raccorder le lanceur fixe au registre autoritaire des tentatives, sans permettre un PID/une commande arbitraire ni redémarrer l'ancienne mission. Définir d'abord la recette de liaison identité/digest/autorité et la portée exacte des descendants.
2. Environnement Linux, décision humaine si nécessaire : le lancement public révisable est `SWARM_TRUST_TOKEN=… bin/swarm-actions-confiance-kernel-public --root RACINE_ISOLEE --json trust kernel-check`, après politique active dans cette seule fixture. Il observe uniquement son helper. Ring refusé EPERM avant programme/perf ; aucun sudo, setcap, sysctl ou changement memlock autorisé implicitement. Ne pas répéter sans précondition modifiée.
3. Codex : préparer puis exécuter la recette globale applicable au candidat exact, en réutilisant uniquement les preuves aux entrées pertinentes inchangées. Tout critère obligatoire manquant bloque le verdict correspondant ; aucun succès moyen.
4. Revue indépendante selon l'autorité disponible, puis RETEX final. Codex seul dans cette session ne fabrique pas une revue indépendante. Les essais locaux et le diagnostic indisponible restent explicitement distincts.

Les travaux indépendants T8 continuent pendant l'indisponibilité noyau. La condition de reprise de la recette native est un environnement explicitement autorisé qui charge les programmes fixes ; la cause unique d'EPERM n'est pas établie.

## Campagne publique commune préparée

Motif : les recettes fonctionnelles ont jusqu'ici été exécutées par tranche sur
plusieurs binaires. Préparer une campagne qui utilise le même binaire pour le
pilote, Processus, les signatures/témoins/archives, le laboratoire, les deux
connecteurs et le courtier distant. Chaque recette conserve sa racine isolée ;
ce n'est pas une transaction commune à tous les services. Les recettes d'identité
aux entrées inchangées restent réutilisées séparément et ne sont pas annoncées
rejouées. Le contexte d'identité local est consommé par les parcours communs.

Le conducteur conserve argv/PID/démarrage dès spawn, empreintes du binaire,
entrées avant/après, sorties privées et résultats nommés. Chaque recette conserve son watchdog et son nettoyage ; le conducteur attend
sa fin et enregistre son état, sans second délai caché ni politique Swarm. Une erreur
ou une recette vide arrête la campagne et reste FAIL ; elle ne devient pas une
indisponibilité attendue. R20 indisponible est référencé à sa preuve existante,
sans relancer la sonde inchangée et sans verdict global PASS. Revue indépendante,
collecte métier/noyau et RETEX final restent ouverts.

### Incident du conducteur et reprise

`campaign-first.out` conserve RuntimeError: No active exception to reraise. Une branche résiduelle du conducteur exécutait raise après la sortie0 du parcours connecteur, ensuite mal étiquetée watchdog_failed. Le produit et ses11 contrôles étaient passés. `run-public-before.py`, les sorties et le reçu original restent intacts. Le conducteur corrigé exige --resume, vérifie argv/empreinte/sortie0/preuve et disparition du processus, puis crée un reçu de reprise distinct sans relancer ce parcours ni écraser l'original. Les six étapes suivantes ont réellement été exécutées.

### Liaison des entrées de recette

La campagne exécutée figeait1000 entrées de compilation et vérifiait leur absence de dérive, mais les empreintes des recettes Python/Node ont été capturées après, pas avant/après. Aucune recette fonctionnelle n'a été éditée pendant l'exécution ; cette limite reste explicite. Le conducteur réellement exécuté est conservé dans run-public-resumed.py. Les prochains lancements ajoutent recettes, scripts de build/i18n, package et conducteur aux empreintes avant/après. Cela ne transforme pas rétrospectivement la preuve actuelle en snapshot préalable. Aucun rejeu inutile des effets pour cette seule amélioration de métadonnées.

## Prochaine preuve après le collecteur explicite

Ne pas recréer le lanceur ni répéter sa disponibilité native à environnement inchangé. Propriétaire Codex : préparer la recette positive fixe avec identité/FD/filtre/arrêt et mesure avec/sans collecte ; l’environnement BPF autorisé demeure une décision distincte, aucun sudo/setcap/sysctl implicite. En parallèle, reprendre la cible sensible bornée R14 selon donnees-r14-coverage.md : lire le SPI et le courtier existants, définir l’octroi/action/destination/confinement avant toute extension. Ne pas requalifier le helper d’attestation existant en fournisseur général. La campagne T8 précédente reste historique ; le pilote et la nouvelle UX viennent d’être rejoués sur le binaire final, pas les sept étapes complètes de cette ancienne campagne. Revue indépendante et RETEX final non déclarés.

## Réexamen après jonction R14 vérifiée

La cible sensible locale est positivement vérifiée sous politique de données, avec classification intrinsèque restricted et contexte de remise lié à l’attestation. Ne pas recréer le courtier ni prolonger T4 par une exigence de fournisseur arbitraire : le plan initial impose de refuser les chemins non démontrés. L’inventaire courant et sa distinction contrôle local/revue/acceptation sont dans donnees-r14-coverage.md.

Prochaine preuve indépendante : préparer le lancement noyau positif borné et le dossier de revue de la version actuelle ; lire les recettes de diagnostic/observation réelles, sans relancer BPF sous même précondition EPERM. Aucune modification de capacités, sysctl, memlock ou lancement de mission native. La campagne fonctionnelle globale doit distinguer les nouvelles preuves R14 de ses sept étapes historiques. La revue indépendante applicable manque encore d’autorité/contextes séparés sous la contrainte Codex seul ; préparer le candidat concret avant toute question. RETEX final non déclaré.

## Reprise du16octobre05:21UTC — candidat commun et lancement positif

Propriétaire Codex. Précondition modifiée : la jonction R14 et l’épinglage du helper ont changé les chemins sensibles depuis la campagne commune précédente. Réutiliser seulement cette ancienne campagne laisserait ces chemins hors du même candidat. Choix : une nouvelle campagne des recettes publiques existantes sur le binaire secret-data-qualified, en racines distinctes ; ajouter les recettes R14 active et substitution d’image, sans répéter la disponibilité BPF inchangée. L’alternative reconstruire tous les tests historiques n’ajoute pas de preuve pertinente aux branches inchangées et n’est pas retenue.

Avant exécution, le conducteur doit refuser dossier occupé, binaire différent, entrées de compilation périmées et recette sans assertions ; enregistrer les entrées de recette/conducteur avant et après, l’identité de chaque processus dès départ, la sortie et la preuve exactes. Une reprise adopte seulement une sortie0 intacte, jamais un échec/interruption ; aucun effet rejoué automatiquement. Les originaux de la campagne précédente restent conservés.

La recette positive BPF doit être préparée sans collecte : binaire et politique exacts, racine créée isolée, helper possédé seulement, assertions process/file/network et nettoyage. Son lancement exige un environnement BPF autorisé différent ; aucune capacité/sysctl/memlock modifiée ici. Elle ne suffira pas seule pour attribuer les tentatives métier, exclure un étranger réel ou mesurer pertes/surcoût. Le dossier de revue reliera candidat, exigences, preuves actuelles/réutilisées/périmées et critères ouverts ; aucune revue indépendante lancée sans autorité.

## Résultat du16octobre05:34UTC — dossier concret disponible

Neuf étapes publiques passent sur le même binaire secret-data-qualified : connecteurs11, preuves/témoin/archives18, laboratoire8, courtier16, courtier sous listes actives17, substitution d’image4 assertions, pilote7scénarios dont23contrôles navigateur, Processus4scénarios dont38navigateur, matrice13. Ces unités ne sont pas additionnées. Six recettes d’identité supplémentaires passent104contrôles navigateur (26/14/16/10/20/18), sans diagnostic inattendu.

Le contrôle de fraîcheur avait constaté des frontières communes modifiées depuis plusieurs reçus historiques. Une nouvelle série ciblée T3/T5/R16/R17 passe407cas utiles sous race (111principaux/296sous-cas) ;408PASS bruts moins le seul parent de fixture laboratoire vide. Les290R14 restent séparés. Entrées de compilation/recettes inchangées pendant les exécutions ; aucune source Go ou ressource embarquée modifiée pour cette reprise. Les12contextes, matricev5 et diff passent ; manifeste1510sources et test nommé passent.

Dossier candidate-current/review-packet.json :1056empreintes d’entrée/42preuves, exigences et critères ouverts, SHA du binaire exact. Auto-contrôle seulement ; aucune acceptation indépendante ou globale. Le consolidateur échoua avant publication car un argv navigateur contenait un chemin relatif ; résolution contre la racine canonique, original build-review-packet-before.py et incident review-packet-path-before.json conservés, exécution corrigée code0. Cette erreur de méthode n’est pas un défaut produit.

La fixture kernel-positive-prepared persiste/reouvre17s/ring131072 sans mutation ni BPF. run-kernel-positive.py refuse l’environnement inchangé avant tout processus natif, garde réellement vérifiée. Le lancement positif reste non exécuté et ne modifiera lui-même aucun privilège. Deux décisions ont été demandées via le parcours utilisateur : environnement Linux autorisé et revue indépendante séparée sous la contrainte Codex seul. Sans réponse, aucune délégation/élévation/sonde identique ; R20/R11 et RETEXfinalR12 restent ouverts.

Les dernières rédactions ont encore dépassé la borne de communication ; aucun lien causal avec une reconnexion n’est établi. Petits blocs et horloge restent nécessaires, pas une réparation déclarée. L’historique des erreurs de recherche de noms supposés et du consolidateur est conservé ; rechercher les producteurs et résoudre explicitement leurs chemins avant consommation.

## Décision utilisateur du16octobre — revue autorisée, eBPF reporté

Instruction originale : « ok 1 et deux on dcomente pour plus tard ». Le point1 autorise une revue indépendante par un contexte Codex distinct en lecture seule. Le point2 reporte les essais eBPF : R20 reste explicitement non qualifié et n’est plus une décision à redemander maintenant. Aucune installation VM ni élévation Linux n’est autorisée. La reprise R20 exige une demande ultérieure et l’environnement BPF approprié.

Le candidat et ses empreintes restent figés pendant la revue. Le contexte review_actions_candidate a reçu la mission et confirmé la prise en charge ; aucun verdict encore acquis. Propriétaire de l’intégration/correction : Codex principal. Après retour, traiter les défauts éventuels et les revérifier, puis consolider la livraison et le RETEX du périmètre convenu avec R20 reporté. Le report ne transforme pas ses preuves manquantes en succès.

## Replan après revue indépendante — F1/F2

Verdict request_changes conservé dans verification-globale-20261016/review-corrections/independent-review-1.json ; deux reproductions CLI natives originales et leurs empreintes conservées. F1/P1 : une chaîne altérée détectée par verify permet encore commit. F2/P2 : intention/résultat natif et politique effective ne sont pas liés dans l’export. Propriétaire correction : Codex principal ; le réviseur ne modifie aucun fichier.

Choix comparés : créer un nouvel export incompatible, ou ajouter des contextes de preuve versionnés aux nouveaux événements et conserver le schéma2/hashes historiques lorsque ces champs sont absents. La seconde option est retenue : snapshots expurgés de politiques, intention/résultat et reçu natif liés au hash de l’événement. Aucun secret/session brute, nouvelle politique ou limite métier. Validation de chaîne commune avant opérations protégées, avant rejeu et dans la frontière SPI ; vues diagnostiques restent lisibles, effets déjà engagés réconciliables si leur chaîne est intègre.

Assertions avant code : corruption hash/ordre/précédent/approbation refusée sans mutation sur les deux connecteurs, accès SPI direct et rejeu ; export nouveau lie politique/intention/résultat natif, altération/omission refusée sous ancres HMAC et publiques, instantanés immuables à la réconciliation/reconfiguration, octets et ancres historiques sans nouveaux champs toujours vérifiables. Régression partagée nécessaire car format de hash consommé par identités/Processus/archives/laboratoire. Sources figées pendant chaque exécution. Après correction, nouveau binaire et reçu exact, recette publique puis nouvelle revue du candidat corrigé par le même contexte indépendant. R20 reste reporté.

### Régression de compatibilité détectée pendant F1/F2

Le premier groupe élargi détecte TestActionsConfianceT2ReplayBindsAuthenticatedAuthority : la garde commune rejette avec erreur d’intégrité un ancien rejeu dont l’original manque, alors que son contrat conserve la restitution historique exacte, sous autorité authentifiée et sans mutation. L’ancien test n’est pas assoupli. Le garde précède toujours toute écriture ; seul un échec d’intégrité typé permettra une seconde lecture sous verrou partagé, identité fraîche et eventReplay existant. Cette lecture ne crée ni événement, ni preuve, ni effet. Une nouvelle commande reste refusée. La garde SPI reste indépendante.

La première exécution ciblée avait aussi révélé le digest du résultat JSON brut réindenté à la persistance ; normalisation par la sérialisation canonique existante, nouvelle série ciblée code0 conservée dans effect-proof-canonical.out. La sonde tardive d’un processus disparu a rencontré ENOENT ; disparition normale pendant contrôle, sans diagnostic de panne produit. Les longues rédactions/analyse et la capture d’entrées après départ de cette série sont des limites de méthode explicites ; le build suivant a son snapshot préalable et son reçu final.

### Entiers exacts dans le nouveau contexte d’effet

Une fixture de transport exécutant le vrai trustAPI observe9007199254740993 converti en9007199254740992 avant sérialisation : nouveau résultat natif exporté, frontière JavaScript à traiter. effect-proof-js-before.json conserve cette preuve, sans prétendre à un navigateur exécuté. Le parcours de preuves utilise désormais une lecture JSON exacte pour les entiers hors Number, sans changer les autres parcours ; cinq gardes Node passent. La recette navigateur est enrichie d’un effet exact9007199254740993 et d’une vérification publique après copier-coller en FR/EN, pas encore exécutée.

Autre garde nécessaire : avec primitives JSON exactes indisponibles, trustAPI avale l’erreur du décodeur personnalisé et retourne un objet error malgré HTTP200. effect-proof-unsupported-before.json reproduit ce contrat incorrect. Après fin des tests en cours, propager l’erreur du décodeur de preuves au parcours d’erreur visible, conserver le comportement du décodeur standard et vérifier le refus réel. Les sources restent figées pendant les deux groupes Go finaux. Ce delta JS n’affecte pas leur code Go ; il exigera son propre contrôle et le binaire final embarqué.

### 16octobre07:23UTC — corrections de revue et lecteur exact

F1/F2 : deux groupes race figés produisent722cas utiles distincts (189principaux/533sous-cas),258effets/données et464identités/Processus/preuves. Les deux packages émettent PASS, aucun cas fail. Le conducteur a reçu SIGTERM143 après avoir enregistré exit0 du premier groupe ; le second Go a continué et émis son PASS. Son code de sortie n’a pas été récupéré. Origine du signal non établie, aucun faux exit0 ni relance identique : receipt récupéré et empreintes inchangées conservés.

F3 découvert par le même réviseur : le collage JSON transformait une séquence1.00000000000000001 en1 ; Go direct refusait, le flux décodé acceptait. Décision générale : conserver chaque jeton numérique non canonique via source du parseur, ou refuser explicitement si cette source n’est pas disponible. Onze gardes client passent ; la recette navigateur réelle vérifie maintenant le refus de cette séquence et la conservation de9007199254740993 sous signature/archive. Son premier lecteur d’archive utilisait JSON.parse et arrondissait lui-même : original conservé, comparaison corrigée sur le texte avec exactParse, aucun moteur Go assoupli.

Candidat courant bin/swarm-actions-confiance-proof-exact SHA2565b9ce6f43d0498624d120c70ab72b8c6b3a66785f0d856df8a357115fedfd2c6. Compilation exit0, entrées figées inchangées.15contrôles CLI de preuve/legacy et46contrôles navigateur FR/EN passent, dix refus attendus attribués, zéro diagnostic inattendu. Campagne publique commune en fin d’exécution, paquet final et verdict indépendant encore requis. R20 reste reporté par décision humaine, sans nouvel essai natif ni privilège.

### 16octobre07:29UTC — verdict et clôture du périmètre convenu

Le même réviseur indépendant rend approve à07:27:54UTC sur paquet2855f3bd… et binaire5b9ce6f4…, après1073/1073entrées,204/204preuves et14contrôles discriminants indépendants. Aucun finding restant confirmé ; R20 reste explicitement reporté, aucune acceptation métier globale. Reçu daté/haché conservé sous review-corrections/independent-final-review.json. Les sources du paquet demeurent figées ; les documents de bilan sont hors des entrées compilées.

Aperçu actualisé par le script préparé : ancienPID4018272/start6213956 et image vérifiés puis arrêtés ; nouveauPID505619/start7013049, binaire5b9ce6f4…, reçu écrit immédiatement avant HTTP. État métier SHA d3b01d34c5c750ca91cbe1591ccd1c978ecb3a52a6416ffbbb0d610cfddd48c1 inchangé, deux demandes/un règlement simulé ; politiques avancées inactives. RETEX final du périmètre convenu produit. Aucun nouveau privilège, compte, paiement, commit/push/déploiement ou redémarrage de mission native.
