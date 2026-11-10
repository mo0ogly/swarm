# Comparaison mémoire : préparation et premier essai réel

Préparation du 16 octobre 2026 après « next », puis quatre évaluations réelles
autorisées par « ok ». Les sections de préparation ci-dessous conservent leur
état antérieur à l’accord ; le résultat réel figure en fin de document. Codex seul,
sans mission Swarm, commit ou déploiement. Les changements du premier lot
mémoire sont préservés ; ses fichiers applicatifs n’ont pas changé. Les nombreux
changements étrangers du checkout ne sont pas attribués à ce travail.

## Préparation : résultat observable avant les appels

Trois des cinq étapes de l’itération sont réalisées et vérifiées localement :
M6.1 préparation, M6.2 grille et M6.3 contrôles négatifs. M6.4 (modèle réel) et
M6.5 (bilan du modèle) restent ouverts. Cela ne représente pas un pourcentage de
qualité mémoire. Le protocole est dans [INCREMENT-MEMOIRE-02.md](INCREMENT-MEMOIRE-02.md).

Le test `TestProjectMemoryComparisonContexts` produit quatre entrées et conserve
leurs quatre contextes moteur originaux. Il admet une procédure via les opérations
mémoire existantes, puis sélectionne sa version pour des tâches de fixture. Les
aperçus ne créent aucune tentative et la table des agents reste vide. Les paires
partagent exactement les mêmes observations, instructions et identités, après
neutralisation tracée du seul numéro de révision du travail. La mémoire et son
attribution ajoutent 1 327 octets à chaque paire. Ces entrées n’ont pas encore été
envoyées à un modèle ; la procédure source et les acceptations sont synthétiques.

Les contrôles du gate du moteur établissent dans des cibles temporaires :

- preuve absente : refus avant restauration, puis `ship_allowed: true` après
  restauration de l’original dont l’empreinte est vérifiée et nouvelle évaluation ;
- preuve différente, original vérifiable indisponible : refus, fichier courant
  conservé, aucune nouvelle acceptation écrite.

La grille note une **proposition** de décision : choix, ordre des actions,
admission après preuve fraîche et acceptation historique inchangée. Une réponse
correcte à la situation initiale échoue sur la variante. L’explication libre est
conservée, sans note lexicale. Les quatre réponses de contrôle positives passent ;
le contrôle négatif retourne volontairement un échec. Tous restent des fixtures.

## Preuves et limites

| Contrôle | Résultat | Original |
| --- | --- | --- |
| Contextes comparables, aperçus sans effet, gate avant/après et variante intacte | PASS, 1 test Go et 2 sous-tests | `evidence/m6-20261016/comparison-contexts-matched.json/.log` |
| Sources de l’export inchangées pendant le contrôle | PASS | `candidate-inputs-before.json` et liste `changed_inputs: []` du résultat |
| Validation de la grille, cas négatifs, liaison aux empreintes, mesures inconnues et refus d’écraser | PASS, 13 tests Python nommés | `grader-tests-verified.json/.log` |
| Quatre bonnes réponses manuelles | PASS de la grille ; modèle NOT TESTED | `fixture-answers-correct.json`, `fixture-result-final-correct.json`, `grader-final-correct.json/.log` |
| Ancienne recette appliquée à la variante | FAIL attendu de cette réponse, contrôle négatif PASS | `fixture-answers-negative.json`, `fixture-result-final-negative.json`, `grader-final-negative.json/.log` : code 1 attendu |
| Fournisseur réel, fidélité de l’explication, consommation et gain avec mémoire | NOT TESTED | Aucun appel. Les mesures inconnues restent `null` et les contrôles déclarent `model_effectiveness: NOT TESTED`. |

Ces fichiers se trouvent sous `evidence/m6-20261016/`. Le sous-dossier `pack/`
contient le manifeste, les fichiers `.engine.txt` originaux, les `.txt` neutralisés
et le modèle de réponses `answers-template.json`. Le lecteur vérifie empreintes,
tailles et transformation ; un changement supplémentaire est refusé même si
l’empreinte de l’entrée d’étude est recalculée. Le format et la commande de
notation sont documentés dans [tools/memory-evaluation](../../../tools/memory-evaluation/README.md).

Une campagne à deux situations ne prouvera pas un apprentissage général. Une
réponse fournie et marquée `supplied_outputs` ne prouve pas sa provenance modèle :
les reçus du fournisseur, sa configuration et les réponses originales devront
être conservés. La grille ne juge pas la qualité de l’explication, et une bonne
proposition ne prouve pas son exécution autonome. Aucune indépendance de revue
n’est revendiquée.

## Incident de méthode et RETEX local

Le premier essai de comparaison a échoué : les deux prompts différaient aussi
par la révision du travail. Le code `workerExecutionContext` inscrit cette révision
dans l’identité courante, et sélectionner une mémoire l’incrémente légitimement.
L’original `comparison-contexts.json/.log` est conservé comme échec.

Deux options : modifier l’identité du moteur, ou préparer une transformation
expérimentale explicite. Choix : conserver le moteur, les deux originaux et leurs
empreintes, puis neutraliser ce seul numéro pour l’étude. La comparaison réussie
contrôle que toute autre différence provient du bloc mémoire et de son attribution.
Apprentissage : ne pas présenter des entrées comme identiques sans comparer les
octets réellement produits ; toute neutralisation doit rester limitée, déclarée
et reproductible. Le README et le test d’export transmettent ce contrôle aux
prochaines campagnes, sans ajouter une consigne au contexte moteur déjà borné.

Un contrôle supplémentaire de paire différente a d’abord échoué à produire son
contre-exemple : les deux cas de la fixture Python partageaient le même objet
`facts`, et modifier un cas modifiait les deux. Le test construit maintenant un
objet distinct pour un seul cas. `grader-tests-final-13.json/.log` conserve cet
échec de fixture ; `grader-tests-verified.json/.log` vérifie la correction et
le refus de la paire devenue réellement différente.

## Proposition et condition avant l’accord

Le projet configure Codex avec le niveau `standard` : `gpt-5.6-sol`, effort
`medium`, présent dans le catalogue local au contrôle. Cette configuration
n’établit pas l’autorisation ni le coût d’une nouvelle campagne mémoire.
Proposition bornée : quatre évaluations isolées, même modèle/niveau, une par entrée,
sans outils, sans mission Swarm ni relance automatique. Budget proposé : quatre
appels maximum ; tarif/coût non établi, à conserver inconnu jusqu’à mesure.

Propriétaire du choix modèle/budget et de l’accord pour les appels : utilisateur.
Précondition manquante : adoption explicite de cette proposition ou configuration
alternative. Après cet accord, l’exécution et son bilan reprennent M6.4/M6.5 ;
ne pas marquer C05/C10/C11/C14 réussis sur la seule préparation de la grille.

## Essai réel autorisé : résultat

L’utilisateur a accepté quatre appels maximum, sans relance automatique, pour
`gpt-5.6-sol`, effort `medium`. **Quatre réponses sont reçues : la grille
préenregistrée donne 2/2 avec mémoire et 1/2 sans mémoire.** Ce résultat porte
sur ces propositions JSON ; aucune amélioration générale ou de sécurité n’est
démontrée. Le `FAIL` agrégé de la grille est conservé, sans changement du barème.

| Entrée | Résultat de grille | Écarts | Durée du processus | Tokens entrée / sortie |
| --- | --- | --- | --- | --- |
| `missing-recoverable-without` | PASS | Aucun | 6.256 s | 16551 / 176 |
| `missing-recoverable-with` | PASS | Aucun | 7.211 s | 16895 / 272 |
| `changed-unrecoverable-without` | FAIL | actions, admission | 6.806 s | 16548 / 201 |
| `changed-unrecoverable-with` | PASS | Aucun | 9.616 s | 16895 / 196 |

La réponse sans mémoire à la variante choisit bien `rebuild_evidence`, refuse de
transférer l’ancienne acceptation et demande une preuve fraîche. Elle ajoute
`evaluate_current` à la séquence et indique `after_fresh_pass` plutôt que le
`blocked` attendu pour l’admission actuelle. La grille stricte marque donc
`actions` et `admission` en échec. L’explication décrit une reprise future sûre ;
ce désaccord ne suffit pas à conclure à une acceptation injustifiée évitée grâce
à la mémoire. Les réponses avec mémoire suivent exactement la convention retenue.

Le code de sortie 1 de `grading-receipt.json` est le résultat observé de cette
notation, pas un contrôle technique passé ni une erreur effacée. La conformité
à la grille et la qualité de la décision métier restent deux appréciations
distinctes. Le contre-exemple doit guider le prochain protocole.

### Traçabilité de l’exécution

- Quatre répertoires de réservation, quatre processus et quatre identifiants de
  conversation distincts ; aucun cas répété, aucun outil observé. Les variantes
  et traitements ont été intercalés : initial sans, variante avec, initial avec,
  variante sans. Le budget de quatre appels est consommé ; aucun appel en plus.
- Les quatre entrées restent celles du paquet figé. Chaque reçu lie l’entrée
  hachée, les arguments, l’heure, les événements JSONL et la réponse finale. Le
  client ajoute ses propres instructions et métadonnées, dont une racine temporaire
  différente ; l’identité complète des requêtes n’est donc pas déduite de la
  seule comparaison des fichiers `.txt`. Aucun accès à ces racines n’était offert.
- Modèle et effort demandés : `gpt-5.6-sol` / `medium`, même configuration. Les
  événements ne rapportent pas séparément l’identité serveur du modèle :
  `model_reported` reste `null`, sans substitution déduite ou revendiquée.
- Le contrôle local préalable renvoie HTTP 503 : une seule requête, aucun outil
  déclaré et paramètres attendus. C’est une preuve locale du client installé,
  pas un appel de modèle. Un premier contrôle local HTTP 400 est aussi conservé.
- Usage rapporté : 66 889 tokens d’entrée, dont 11 776 en cache ; 845 tokens de sortie, dont 403 de raisonnement. Les sous-comptes ne sont pas ajoutés deux fois. Somme des durées de processus : 29.889 s. Coût monétaire inconnu (`null`). Le cache et cet échantillon ne permettent pas une conclusion de coût ou de latence.

Originaux sous `evidence/m6-20261016/real-codex-02/` : `campaign.json`,
`receipts-index.json`, `answers.json`, `graded-results.json` et
`grading-receipt.json`. Chaque sous-dossier de cas conserve `reservation.json`,
`events.jsonl`, `stderr.txt`, `answer.json`, `response.schema.json`, `receipt.json`.
Le `model_effectiveness: UNPROVEN` du grader est maintenu : les reçus ajoutent
une provenance d’exécution, sans transformer quatre réponses en preuve générale.

### Incident du collecteur et correction vérifiée

Au premier appel, Codex émet un `item.error` de démarrage : Code Mode est
indisponible parce que son hôte est volontairement désactivé. Le tour termine
néanmoins avec sa réponse et son usage. Le collecteur initial comptait tout type
d’item autre que réponse/raisonnement comme une activité d’outil : il a produit
un faux échec local après un appel réussi.

La correction distingue diagnostics, outils, tours terminés et tours échoués,
sans condition sur le texte de l’erreur ni sur le cas. Trois tests nommés
(`runner-events-tests.json/.log`) vérifient le diagnostic conservé, les vrais
outils et types inconnus toujours visibles, et l’absence de succès d’un tour
absent/échoué. Les treize tests de grille passent aussi
(`grader-real-phase-tests.json/.log`).

Le premier `receipt.json` erroné n’est pas réécrit : un `assessment.json` séparé
lie ses empreintes aux événements et explique la conclusion corrigée. Le modèle
n’a pas été rappelé. L’avertissement Codex reste présent ; aucun outil n’a été
réactivé pour obtenir un statut vert.

### RETEX et suite bornée

M6.1 à M6.5 sont réalisés dans leur périmètre : préparation et contrôles locaux,
quatre évaluations autorisées, notation originale, diagnostic et présent bilan.
Cela ne clôt ni l’apprentissage procédural ni les exigences globales C05/C10/C11/C14.

Le signal observé est une meilleure conformité à cette grille avec rappel.
Sa portée métier est limitée par un champ `admission` qui mélange état actuel et
condition de reprise future. Avant une prochaine campagne, séparer ces deux
questions dans le contrat de réponse, préinscrire leurs critères et conserver
ce contre-exemple. Ne pas renoter les réponses présentes avec un barème changé.
Le README impose désormais cette préparation aux prochaines comparaisons.

La prochaine expérience utile doit aussi apporter une information de projet
vérifiée qui manque au contexte sans rappel ; celui-ci doit pouvoir signaler
ce manque plutôt que deviner. La qualité sera une décision mieux informée, pas
l’obligation de répéter une formule. Cette expérience reste à cadrer et nécessite
un nouveau budget si elle emploie un modèle. Aucune campagne supplémentaire,
aucune extraction automatique et aucune nouvelle politique produit ne sont lancées.
