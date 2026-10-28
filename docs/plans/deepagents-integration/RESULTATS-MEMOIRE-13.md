# M17 — référence vérifiée avant résumé automatique

16 octobre 2026, après « suite ». Le contrat et la référence exécutable de
[l'incrément 13](INCREMENT-MEMOIRE-13.md) sont réalisés. L'outil est hors moteur :
aucun contexte d'agent, base de mission ou fournisseur n'est modifié. Le résumé
automatique du produit reste à intégrer et qualifier.

## Sources et décision

Le dépôt Swarm possède une synthèse de mission écrite par `checkpoint`, des
échanges de préparation, une session fournisseur par dialogue et la projection
déterministe M16. Aucun de ces producteurs ne constitue une synthèse progressive
attribuée avec récupération sur échec. Ajouter un appel LLM et remplacer
l'historique aurait introduit une politique et une nouvelle confiance sans recette.
La voie retenue est une référence extractive testable avant ce raccordement.

Le code Deep Agents au SHA `29d646fd2d832a44c476964333eddecb0ec1c0cd`
conserve une frontière `cutoff_index`, un message de résumé et un chemin d'historique.
La méthode `_apply_event_to_messages` reconstruit résumé puis suffixe. Quatre
sondes exécutent uniquement cette méthode extraite par AST du fichier conservé,
dont l'empreinte est vérifiée contre M16. Elles observent : frontière 2 sur trois
messages → résumé et dernier message ; frontière -1 → même dernier message ;
frontière 4 → résumé seul avec avertissement ; booléen vrai → résumé et suffixe
à partir de l'index 1. Ce résultat ne prouve pas que le middleware complet crée
ces frontières ; nous n'attribuons pas cette observation à tout son parcours.
Source, sonde et résultats : `evidence/m17-20261016/upstream-boundary-probe.py`,
`upstream-boundary.json`, `upstream.json/.log` ; original amont conservé en M16.

## Contrat livré

`tools/memory-evaluation/summarization.py` valide un historique annoté de fixture
et un candidat fourni. Les messages, états protégés, portée et reçu parent sont
liés par empreinte ; une empreinte n'est ni une authentification ni un droit.
Le candidat doit conserver intégralement les messages protégés du préfixe :
exigence ouverte, preuve historique, intention future et incident ouvert.
Les états et citations contradictoires, absents ou ajoutés sont refusés.
La liste suit l'ordre de l'oracle. Il n'y a pas d'évaluation de prose libre.

La frontière est un entier strict, laisse des messages récents et ne coupe aucun
groupe d'outils. Les résultats vides sont valides dans un groupe complet. Pour
progresser, les anciens messages et annotations restent un préfixe inchangé et
la nouvelle frontière avance ; les protections précédentes restent incluses.
Un refus de candidat produit FALLBACK : aucun extrait admis et tous les messages
sources restitués. Un historique invalide est une erreur, pas une liste vide.
Les sorties sont exclusives et privées, les sources et anciens reçus intacts.

Les annotations constituent un oracle de recette, sans preuve de leur justesse
métier. La version ne modélise pas la résolution ultérieure d'un incident ou
l'abandon autorisé d'une exigence. Les messages d'outil sont une représentation
de référence ; aucune validité du protocole d'un fournisseur réel n'est déclarée.
Les reçus restent `reference_only: true`, `model_effectiveness: NOT TESTED`.

## Preuves et vérification

| Exigence | Résultat local / preuve sous `evidence/m17-20261016/` |
| --- | --- |
| SUM-01 sources, portée, schéma, état protégé et entrées invalides | PASS, tests nommés dans `reference-final.json/.log`. JSON dupliqué/non fini et historique invalide refusés. |
| SUM-02 frontières et outils | PASS, entiers négatifs/nuls/trop grands, booléens/flottants et coupure d'un groupe refusés. Groupes complets, contenus vides et résultats multiples couverts. |
| SUM-03 éléments protégés et fidélité extractive | PASS sur la fixture : citations intégrales et états exacts ; omission de l'incident, invention et promotion du futur/historique refusées. Pas de fidélité LLM déduite. |
| SUM-04 progression | PASS sur deux synthèses ; parent modifié, source ancienne changée, portée différente et frontière sans progression refusés. |
| SUM-05 échec et originaux | PASS, échec déclaré, vide et contradictions donnent tous les messages originaux, sans alias mutable sur les entrées. |
| SUM-06 publication de recette | PASS, vraie CLI de l'outil Python, fichiers nouveaux en 0600, sortie existante conservée et erreur explicite. |
| Prévention du refus des outils vides | ERROR reproduit avant correction, puis PASS : `empty-source-red.json/.log` et préimages du module/test. |
| Diff, sources courantes, anciens résultats et environnement | `audit-final.json`, `candidate.diff`, contrôle des empreintes M16 ; serveur utilisateur conservé. |

**24 tests Python distincts** passent ; les 22 tests préliminaires ne s'y ajoutent
pas. La vraie CLI de l'outil effectue **14 invocations sur la recette finale** : deux candidats de
référence générés, deux projections progressives PASS et dix refus FALLBACK.
Il s'agit d'invocations Python, pas d'appels fournisseur ou de la CLI applicative
Swarm. Les sources synthétiques initiales et étendues restent byte à byte
identiques après les essais ; originaux, arguments, sorties, codes et empreintes
sont conservés sous `cli/`.

Après la correction du décodeur, les preuves courantes sont sous `cli-final/`
et `reference-final.json/.log` ; le premier parcours `cli/` reste historique.

Le premier contexte de référence passe de 11 655 octets de messages sources à
1 236 octets d'entrées protégées et messages récents, sur cette fixture contenant
un long résultat d'outil synthétique. Les quatre protections restent présentes ;
la seconde projection en conserve cinq. Cette mesure n'est pas un gain établi
en tokens, coût ou qualité LLM. Aucun modèle, appel natif Swarm ou base réelle.

La documentation [FR](../../PROJECT-MEMORY.md), [EN](../../en/PROJECT-MEMORY.md)
et la [recette détaillée](../../../tools/memory-evaluation/README.md#référence-extractive-de-résumé-progressif)
expliquent les formats, sorties, refus et limites. Les sources Go, ressources
embarquées et paramètres de construction M16 n'ont pas changé ; ses preuves ne
sont pas renommées en nouveaux tests. Pas de compilation ou campagne navigateur
supplémentaire : aucun moteur ni écran modifié.

## Replan et RETEX

La revue du premier outil révèle une hypothèse trop stricte : « message vide »
était assimilé à une source invalide, même pour un appel sans texte et un résultat
d'outil vide. Le nouveau cas reproduit ce refus avant correction. Le validateur
partagé accepte ensuite ces enregistrements dans un groupe complet, garde le
refus d'une protection vide et ajoute le cas de résultats multiples. La correction
ne repose ni sur une mission ni sur une formulation d'erreur. Les 24 tests et
la recette passent avec ces sources corrigées.

Un second cas reproduit une exception `ValueError` du décodeur JSON (entier
dépassant sa capacité de conversion). Le candidat était refusé avec code 2,
sans reçu de FALLBACK, alors que les originaux étaient disponibles. Toutes les
erreurs de décodage de candidat de cette famille suivent désormais le même
retour à l'historique, sans relever la limite du parseur. Cas et sources rouges :
`candidate-decoder-red.json/.log` et préimages ; variantes JSON invalide,
champs dupliqués et nombres non finis conservées dans le même test.
Le collecteur omettait le nom d'un test dont seul le sous-test portait FAIL ;
le journal prouve un test exécuté. Le reçu initial est conservé, son évaluation
de comptage est séparée et le collecteur est corrigé pour les contrôles suivants.

La première recherche de test amont utilisait un nom supposé absent. La découverte
par `rg --files` identifie `test_summarization_middleware.py` ; aucun résultat de
cette suite non exécutée n'est déclaré. Les sources réellement exécutées sont
la méthode AST épinglée et l'outil de référence local. Les empreintes des sources
Python sont inchangées entre les contrôles finaux et la revue ; les versions
précédentes restent conservées. Les pages de documentation suivies par le
collecteur ont évolué après les premiers contrôles, sans être des entrées de
leur exécution Python.

La revue locale examine schémas, chaînes de sources, états, messages d'outil,
sorties exclusives, erreurs et absence de mutation. Ce n'est pas une revue
indépendante. L'apprentissage réutilisable figure dans la recette et les docs :
distinguer vide valide d'un outil, sortie vide de synthèse, contenu cité, état
historique et autorisation. Le contrôle de citations n'est pas une analyse
sémantique générale ; les étiquettes et empreintes ne remplacent pas les droits.

## État restant et suite du plan

MEM-C16 et MEM-C17 restent **PARTIAL** : frontières, attribution, fidélité
extractive des éléments annotés et récupération sont vérifiées dans la référence,
sans résumé LLM ni raccordement aux agents. Les évaluations historiques sont
conservées sans renotation. Le serveur réel reste au PID 2186583, même URL ;
aucun déploiement, migration, purge ou admission de résumé.

La prochaine intégration doit d'abord produire des sources Swarm autorisées et
un état courant inspectable, puis lier toute proposition à ce contrat, à ses
originaux et au candidat réellement consommé. Une politique de déclenchement,
taille ou rétention éventuelle doit être persistée et éditable dans Admin avec
les mêmes validations API/CLI/Go. Il faudra vérifier le reçu réellement transmis,
les erreurs de synthèse, la fraîcheur et la conservation avant remplacement.
La comparaison LLM devra garder les mêmes cas et des reçus d'exécution séparés ;
les huit appels M6/M7 restent consommés et aucun nouvel appel n'est inféré.
