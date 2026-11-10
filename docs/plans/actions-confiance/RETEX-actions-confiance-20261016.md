# RETEX — Actions de confiance

RETEX final du périmètre convenu au 16 octobre 2026. La revue indépendante du candidat corrigé est favorable (`approve`, 07:27:54 UTC), sans défaut restant confirmé dans ce périmètre. L’utilisateur a reporté les essais eBPF natifs. R20 reste non qualifié ; ce document et le verdict de code ne constituent pas une acceptation métier globale.

## Besoin et résultat observé

Reprendre le code existant et livrer des actions attribuables, autorisées sur leur contenu exact, sans effet dupliqué, avec données/secrets contrôlés, processus, preuves et recettes publiques. Le travail a été réalisé directement par Codex ; les scénarios métier sont simulés, les fournisseurs d’identité sont des émetteurs de fixture isolés. Aucune mission native n’est utilisée comme preuve d’autonomie de cette livraison.

Le candidat corrigé et ses preuves sont dans `verification-globale-20261016/review-corrections/review-packet.json`, binaire `bin/swarm-actions-confiance-proof-exact`, SHA256 `5b9ce6f43d0498624d120c70ab72b8c6b3a66785f0d856df8a357115fedfd2c6`. Le paquet initial `candidate-current` reste historique. Les contrôles locaux ne remplacent pas la revue indépendante. Le report utilisateur des essais noyau ne transforme ni EPERM ni les doubles de test en observations eBPF positives.

## Mesures attribuées

Campagne préliminaire historique : neuf étapes publiques sur le même ancien binaire dans des racines distinctes. Une série Go sous race passait407cas utiles (111principaux/296sous-cas),408PASS bruts moins un parent de fixture vide. Six parcours d’identité passent104contrôles navigateur. Les290cas R14 restent une série distincte liée à ses entrées ; les unités de scénario, sous-cas et interaction ne sont pas additionnées.

Durées calculées depuis les reçus originaux : campagne publique99,299s ; série Go ciblée263,914s ; six recettes d’identité86,400s séquentielles. Les groupes se chevauchent dans le temps : ces durées ne sont pas un temps total de livraison. Les coûts et la consommation Codex ne sont pas établis par ces reçus ; inconnu ne signifie pas zéro. Mesures natives de surcoût eBPF : non réalisées, reportées.

## Défaillances établies et corrections vérifiées

| Frontière | Fait observé et cause | Correction et preuve |
| --- | --- | --- |
| Image du helper | Le chemin du serveur de fixture remplacé lançait un substitut : matière factice capturée et faux HMAC accepté. | Image courante ouverte et transmise par FD ; le substitut ne s’exécute plus, vrai HMAC et14gardes OS observés. Originaux image-before et image-final-public conservés sous verification-sensitive-20261016. |
| Classification des ressources | Une ressource non administrée était admise sur la classification déclarée par le client. | Règle de ressource exacte administrée ; CLI code4 et octets d’état inchangés dans data-policy-resource-after.json. |
| Admission des secrets | Les listes actives n’étaient pas appliquées au transfert du courtier. | Tuple restreint calculé par le serveur, contrôlé avant lecture applicative, copie mémoire, octroi et consommation ; contexte lié à l’attestation. Cas avant et recette publique active conservés. |
| Conducteurs de preuves | Un raise après succès et un chemin relatif traité comme absolu interrompaient des consolidations. | Résultat produit distingué de l’erreur de méthode ; adoption contrôlée de la sortie0 sans rejouer les effets ; résolution explicite des chemins, originaux conservés. |
| Communication | Plusieurs blocs de rédaction/analyse ont dépassé la borne60s malgré les rappels. | Découpage/horloge appliqués ensuite, mais absence de récidive générale non démontrée. Aucun lien causal établi avec une reconnexion. Ce point reste un apprentissage ouvert. |

Ces corrections sont générales : aucun identifiant de mission ou formulation d’erreur n’accorde de passe-droit. Les permissions de fichiers ne prouvent pas un coffre contre le même UID ; le helper scellé n’est pas un fournisseur arbitraire.

## Ce qui a apporté des preuves utiles

Réutiliser le pilote existant a conservé ses propriétés d’accord exact, effet unique et réconciliation. Les tests aux frontières publiques ont discriminé de vrais défauts des erreurs de fixture : refus métier, identité, délai de recette et échec de consolidateur restent attribués séparément. La double vérification empreintes avant/après et l’exécution sur un binaire commun réduisent les ambiguïtés de version. Les politiques sont persistées et visibles dans Admin, avec validation Go/API/CLI commune et limites techniques distinctes.

La revue séparée n’a été lancée qu’après l’accord humain explicite. Le premier verdict a demandé deux corrections ; le même réviseur a ensuite discriminé le défaut numérique F3 puis approuvé le candidat final après vérification de ses corrections. Le report des essais eBPF est une décision utilisateur tracée, aucune réussite technique supposée. Les scénarios automatisés ne prouvent pas à eux seuls une supervision native autonome.

## Revue indépendante et corrections demandées

Le contexte indépendant review_actions_candidate a reçu et revérifié1056sources/42preuves et le même binaire. Verdict request_changes : F1/P1, une chaîne altérée détectée par verify autorisait encore un nouvel effet ; F2/P2, les exports ne liaient pas l’intention SPI, le résultat natif et la politique effective. Les deux cas ont été reproduits par CLI dans des fixtures distinctes, sans modifier le candidat ni un état vivant. Originaux et verdict conservés sous verification-globale-20261016/review-corrections.

Cette revue a apporté une preuve supplémentaire que les seules campagnes vertes ne fournissaient pas. Les corrections ont été vérifiées, avec gardes écrites avant code et FAIL originaux conservés. Les407cas et104contrôles précédents restent attribués à l’ancien candidat ; ils ne qualifient pas automatiquement le candidat corrigé. Le même contexte indépendant a contrôlé de nouveau le candidat corrigé avant le verdict favorable ; aucun résultat préliminaire n’a été converti rétroactivement en acceptation.

## Corrections du lecteur et suivi des exécutions

La revue a aussi discriminé une normalisation numérique du lecteur de preuve : un décimal non admissible en entier Go devenait un entier valide dans le navigateur. La correction conserve la source lexicale des nombres ou refuse le parcours ; elle ne modifie ni le schéma signé ni le vérificateur Go. La garde publique conserve9007199254740993 dans l’archive, et refuse1.00000000000000001 pour une séquence. Un test de comparaison d’archive arrondissait lui-même la valeur : son FAIL reste un défaut de recette distinct du produit, corrigé par lecture du texte exact.

La preuve du second groupe Go illustre une autre limite : le conducteur est interrompu mais son enfant identifié continue. Le package émet PASS et les sources sont conformes ; le code de sortie reste inconnu. Ce résultat ne devient pas un exit0 reconstruit. Prévention : rattacher les reçus de fin à l’exécuteur qui peut effectivement observer la sortie, conserver une fin structurée distincte de la mort du superviseur de recette. Aucun effacement ni répétition de722cas n’a été utilisé pour masquer cette limite.


## Prévention priorisée et responsabilité

| Priorité | Mesure générale | Responsable et vérification |
| --- | --- | --- |
| P1 | Contrôler la chaîne et les originaux avant chaque nouvelle publication, y compris au SPI ; le rejeu historique authentifié reste en lecture seule. | Moteur/mainteneur ; corruption des deux connecteurs et absence de mutation, gardes F1. |
| P1 | Exporter les instantanés effectifs signés de politique/intention/résultat et vérifier leurs liens. | Moteur/mainteneur ; export, deux ancres, altération et réconciliation sans réécriture, gardes F2. |
| P2 | Préserver la source des nombres avant le contrôle serveur ; une preuve invalide ne doit jamais être réparée par le client. | Client/mainteneur ; décimal trompeur, int64 extrêmes, primitives indisponibles, gardes F3. |
| P2 | Séparer résultat nommé, résultat du package et sortie de processus ; conserver les interruptions. | Conducteur de recette/Codex ; reçus attribués, jamais de code0 reconstruit. |
| P2 | Couper aussi la rédaction et l’analyse, pas seulement les appels d’outils. | Codex ; borne60s contrôlée par l’horloge. Des dépassements ont encore eu lieu pendant cette reprise ; la correction générale de communication n’est pas démontrée. |

Le dossier de reprise R20 a un propriétaire explicite : utilisateur pour le choix et l’autorisation de l’environnement, puis Codex pour la recette possédée. Le report ne crée aucune tâche automatique privilégiée. La préparation déjà conservée ne doit pas être relancée dans l’environnement inchangé.

## Résultat final et limites conservées

Le paquet corrigé lie 1073 entrées et 204 preuves ; le réviseur les a toutes relues avant et après ses contrôles. Son reçu final est `verification-globale-20261016/review-corrections/independent-final-review.json`, SHA256 `9ae216d30e5c2016d8f0541b7b50544edae0ff290071fb297775365ba375250f`. Il a exécuté 14 contrôles indépendants sur les trois défauts F1/F2/F3 et n’a confirmé aucun défaut restant dans le candidat examiné. Les refus sur chaîne altérée, rejeux authentifiés sans écriture, contextes signés, compatibilité historique et refus du décimal sont effectivement observés.

Le candidat final passe les neuf étapes publiques : connecteurs11, preuves/témoin/archives18, laboratoire8, courtier16, courtier actif17, image4, pilote7scénarios dont23contrôles navigateur, Processus4scénarios dont38contrôles navigateur et matrice13. La recette additionnelle de preuve passe15contrôles CLI, le client exact11gardes et le navigateur de preuve46contrôles FR/EN (20contextes de thème/largeur), dix refus attendus et zéro diagnostic inattendu. Les unités et campagnes historiques ne sont pas additionnées.

Les deux groupes Go sous race couvrent722cas utiles distincts (189principaux/533sous-cas), après exclusion de deux parents de fixture vides précisément nommés. Les deux packages structurés émettent PASS ; le premier processus a une sortie0 observée. Le second code de sortie reste inconnu après interruption du conducteur143, limite présente dans le consolidateur et examinée par le réviseur. Les747entrées Go/dépendances pertinentes sont restées inchangées. Les changements suivants ne concernent que le décodeur/recette UI, catalogues et matrice ; le binaire final et ses exécutions ont leurs entrées figées. Build, vet, traductions,12contextes, manifeste1515sources/test nommé et contrôle de diff passent.

Mesures finales depuis `final-measurements.json` : campagne commune114,443s ; navigateur de preuve40,011s ; premier groupe Go538,221s de processus ; second757,687s jusqu’au résultat structuré du package, pas jusqu’à une sortie observée. Les groupes se recouvrent ; aucun total artificiel ni coût nul n’en est déduit. La première campagne/candidat et ses erreurs restent historiques.

L’aperçu a été actualisé après le verdict favorable, sans mutation des deux demandes ni de l’unique règlement simulé. Son reçu est `qualified-preview-process.json` dans le même dossier. Aucune observation de rechargement de l’IAB n’est revendiquée.

Le périmètre convenu est livré et ce RETEX est final pour ce périmètre. La collecte eBPF native positive, son attribution/exclusion/pertes/reprise/nettoyage et son surcoût restent reportés, sans acceptation technique. Leur reprise exige une nouvelle décision d’environnement/autorisations réellement disponible. La revue ne constitue ni certification, ni acceptation métier globale, ni qualification des fournisseurs généraux exclus. Les améliorations de communication restent à surveiller : des dépassements60s ont encore été constatés dans cette reprise ; les rappels et réveils seuls ne prouvent pas leur réparation.
