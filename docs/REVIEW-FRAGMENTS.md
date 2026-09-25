# Revue de gros candidats : reprise bornée

## État du raccordement

Le parcours moteur utilise désormais les fragments lorsque les lots par tâche dépassent la limite de transport. La reprise publique d’un refus de taille sans appel préalable vérifie le dossier complet et conserve le candidat et la tentative. Chaque inspection, la sélection des pièces et la décision finale disposent d’une réservation durable distincte. La finalisation relit les réponses originales ; seule la transaction habituelle peut accepter le résultat après recontrôle des preuves, critères, fournisseur et modèle.

La recette utilise un fournisseur local déterministe : verdict favorable, refus final, reprise du précontrôle sans producteur et rejeu sans double appel, deuxième publication sur la base acceptée et invalidation si le journal final de cette base est altéré. **La reprise explicite des inspections interrompues conserve le même journal, les inspections réussies et tous les appels consommés.** Elle précontrôle le budget manquant et passe par une autorisation durable consommée sous verrou. Une simple réconciliation ne relance pas un appel interrompu. La sélection et la décision finales suivent la même règle : une nouvelle réservation exige une autorisation explicite liée à l’appel interrompu et le budget restant. Un verdict favorable déjà durable peut être finalisé sans nouvel appel, même lorsque le budget est épuisé. Le précontrôle en lecture seule conserve `executable:false` : il ne constitue pas une autorisation d’exécution.

Les sections suivantes retracent la construction des composants. Elles ne prouvent pas l’installation du protocole ni le succès d’une revue IA réelle.

## Pourquoi les lots actuels ne suffisent pas

Le découpage actuel répartit les tâches et leurs sources annexes mais conserve tout le diff dans chaque appel. Il ne peut donc examiner un diff qui dépasse à lui seul la limite de 192 Kio. Rejouer ce précontrôle ne produit aucune information nouvelle.

Deux voies ont été comparées :

- Réduire la remise : préférable avant production, mais retirer après coup du code de la version testée impose de refaire les vérifications correspondantes et une révision explicite de la remise. Les anciennes preuves ne suffisent pas.
- Examiner des fragments complets puis leurs interactions : conserve le candidat exact, mais exige un protocole durable de couverture et une décision indépendante finale. C’est la voie étudiée ici ; le moteur ne doit pas la simuler avec les anciens lots.

## Faisabilité en lecture seule

`python3 tools/review/fragment_preflight.py --git-dir REPOSITORY.git --base BASE_SHA --candidate CANDIDATE_SHA --calls-available N --reserve-final-calls M --output NEW_DIRECTORY`

Le programme résout les deux commits, vérifie leur ascendance et lit le diff sans filtre externe ni textconv. Il conserve les sections de fichiers entières, leur ordre, leurs octets UTF-8 et leurs empreintes. Chaque paquet contient le candidat, la base, l’empreinte du diff et de l’inventaire, les identités des sections affectées et leur contenu intégral. L’inventaire complet est conservé dans `plan.json` plutôt que répété dans chaque appel. Une reconstruction indépendante de tous les paquets doit reproduire le diff exact.

Chaque paquet JSON tient dans 192 Kio avec 24 Kio réservés pour les consignes et le schéma de réponse. Un fichier trop gros, un diff binaire, une couverture incomplète, un paquet altéré ou un budget insuffisant sont refusés. Le dossier de sortie doit être nouveau. Aucun fournisseur n’est appelé ; aucun Store n’est ouvert ; aucune limite n’est augmentée.

Cette estimation couvre **le diff uniquement**. Les rapports, sources complémentaires, contrats, reçus et preuves de la décision finale doivent encore être mesurés. Réserver des appels de synthèse ne prouve pas que leurs entrées tiendront ni que le vérificateur rendra un avis favorable.

## Protocole moteur requis avant activation

1. **Précontrôle total** : construire les fragments, les pièces complémentaires et les appels finaux ; vérifier chaque taille et le budget total avant le premier appel. Toute impossibilité reste un refus sans dépense.
2. **Identités figées** : candidat, base, tentative, contrat, politique, reçus, méthode, fournisseur, modèle et inventaire sont inclus dans l’empreinte du plan. Un changement invalide la reprise.
3. **Inspection indépendante** : chaque réponse désigne exactement son fragment et ses fichiers. Elle expose constats, références précises et incertitudes. Un fragment examiné ne constitue pas une tâche acceptée. Toute zone non examinable reste explicite.
4. **Preuves durables** : préserver les paquets et réponses brutes, leurs empreintes et les réservations d’appels. Après interruption, réutiliser seulement une réponse achevée, intacte et rattachée au même plan ; ne jamais rembourser un appel ambigu.
5. **Interactions** : la revue finale reçoit l’inventaire, les contrats, les contrôles et les preuves originales nécessaires aux interactions identifiées. Les résumés servent à s’orienter, jamais de substitut aux preuves. Une preuve nécessaire absente entraîne `unknown`.
6. **Critères complets** : la décision finale couvre chacun des critères de chaque tâche concernée, sur le même candidat. Une somme de fragments sans défaut détecté ne vaut pas avis favorable global.
7. **Publication** : le moteur recontrôle les identités et les preuves avant la transaction de publication. Ni manifeste, ni inspection partielle, ni précontrôle réussi ne peut publier le candidat.

## Recette exigée

- Reconstitution exacte, UTF-8, renommage, suppression, changement de mode, fichier sans retour final ; refus binaire et fichier indivisible trop gros.
- Fragment manquant, doublon, ordre changé, contenu changé, candidat/contrat/politique/fournisseur modifié.
- Budget insuffisant avant toute réservation ; arrêt avant/après réservation et réponse ; reprise sans double dépense.
- Réponse liée à un autre fragment ; incertitude ; défaut local ; défaut reliant deux fragments ; preuve finale absente.
- Aucun passage à `accepted` sans couverture complète **et** avis final indépendant favorable **et** contrôles frais du même SHA.
- Anciennes revues et plans de lots inchangés, y compris leurs empreintes et leur reprise.

## Conservation du plan existant

E1–E5 restent enregistrées avec leurs preuves historiques. E6 demeure la tâche de recette réelle, avec ses tentatives consommées et son candidat conservé. E7 et E8 gardent leurs identifiants et dépendances. Le travail sur le protocole relève de la correction du moteur ; ce document n’applique aucune modification de statut ou de budget dans la mission.

## Précontrôle du candidat conservé dans le moteur

`swarm planning fragment-preview WORK --input request.json`, avec `{"task_id":"TASK"}`, ou GET `/api/v1/planning?work=WORK&task=TASK&action=fragment-preview`. La route utilise l’authentification locale existante.

Le moteur vérifie la tentative arrêtée, le résultat récupéré, le candidat conservé, le contrat et les reçus avant de construire les paquets. Ceux-ci couvrent tout le diff, les sources annexes, les rapports, livraisons, contrats, contrôles et la base de revue éventuelle. Les identités et empreintes permettent de refuser une couverture altérée. Le budget restant est lu dans la mission ; deux appels sont réservés pour une étape finale encore à implémenter.

Le résultat porte toujours `executable:false`. Il ne réserve pas d’appel et ne déclenche pas de fournisseur. La reprise existante reste refusée tant que le protocole d’exécution durable et la décision finale ne sont pas implémentés. Le précontrôle ne certifie pas le volume futur de cette décision finale.

## Journal transactionnel (non activé)

Le Store lie le plan et chaque version du journal à leurs empreintes. Réserver un fragment consomme un appel dans la même transaction que l'enregistrement de la réservation ; deux connexions concurrentes ne peuvent pas réserver le même état. Un refus de budget ou une pause conserve les compteurs et le journal précédents.

Une réponse peut être enregistrée après réouverture du Store, sans nouvelle dépense, uniquement pour sa réservation et les mêmes candidat, tentative, méthode, fournisseur et configuration de modèle. Les réponses antérieures restent immuables. Une inspection réussie ne permet pas de publier : la décision finale indépendante et son raccordement restent à implémenter.

## Entrée de décision finale (construction seulement)

Le constructeur de décision finale conserve les contrats, rapports et contrôles, distingue les extraits originaux des opinions d'inspection et peut joindre des pièces entières demandées par leur index et empreinte. Les références inconnues, doublons, contenus périmés et dépassements de capacité sont refusés. Aucun fichier demandé n'est tronqué.

Le parseur reçoit uniquement les preuves originales effectivement incluses dans cet appel : une citation qui existe dans le diff complet mais pas dans le message final est refusée. Le constructeur ne déclenche aucun appel et ne lève pas l'interdiction de publication. La réservation des appels finaux, leur exécution, leur reprise et leur ancrage restent nécessaires avant activation.

## Capacité de la décision avant dépense

Chaque raison et extrait d'inspection est limité à 96 octets JSON sérialisés hors guillemets, échappements et UTF-8 compris. Le plafond total de réponse reste 16 Kio. Une réponse trop longue est refusée, jamais raccourcie. Cette limite permet de calculer une borne supérieure de l'entrée finale avec les identités, noms, contrats et consignes réels avant tout appel.

La capacité restante concerne le JSON complet des pièces supplémentaires, pas seulement leur texte. Elle ne garantit pas qu'une demande future de preuve pourra tenir. Le moteur doit vérifier cette demande exacte et refuser sans troncature si nécessaire ; un précontrôle de taille ne prouve ni la suffisance des pièces ni un verdict favorable. Ce calcul n'est pas encore raccordé à l'exécution.

## Sélection indépendante avant décision

Un premier appel final propose les pièces originales nécessaires, avec les index et empreintes de l'inventaire. Son état `ready` autorise seulement l'examen de cette sélection ; `unknown` conserve une insuffisance de preuves et n'autorise pas la décision. Les coûts JSON par pièce, les références et l'enveloppe restante sont indiqués. Le moteur remesure le message complet et refuse toute sélection dupliquée, périmée ou trop grande.

Le précontrôle vérifie les consignes et schémas des inspections ainsi que les messages maximaux de sélection et de décision avant la première dépense. Le transport tabulaire conserve intégralement les extraits et opinions ; seule la répétition des clés JSON disparaît. Ces fonctions ne lancent pas encore le fournisseur et ne publient aucun résultat.

### Capacité de réponse avant appel

Les nouveaux lots doivent aussi permettre une réponse complète : le moteur mesure un résultat `inspected` par pièce avec raison et extrait de 96 octets chacun, identités comprises, et réserve 2 Kio pour le formatage dans la limite de 16 Kio. Le découpage conserve toutes les pièces et recalcule le nombre d’appels requis. Un ancien lot trop chargé est refusé avant appel ; ses journaux restent lisibles et ses consommations sont conservées. Ce contrôle ne garantit ni la durée de l’analyse réelle ni la place pour toutes les demandes de preuves possibles.

### Reprise d’un lot ancien trop chargé

`planning retry-review` recalcule le découpage lorsque la capacité de réponse de l’ancien plan est dépassée, uniquement pour une revue en erreur dont toutes les réservations sont interrompues et qui ne possède aucun résultat ni décision finale. L’ancien plan et le journal restent ancrés sous `replanned_from` et sont vérifiés à chaque lecture. Les compteurs restent inchangés ; le budget doit couvrir le nouveau plan avant son enregistrement. Une seule replanification est prise en charge ; les reprises avec résultats existants conservent leur plan. `planning fragment-preview` examine aussi cette revue interrompue sans appel ni mutation.

### Diagnostic des appels interrompus

Une erreur de processus indique désormais les octets reçus, le nombre d’événements JSON analysés, le dernier type reconnu et la présence d’un événement final. Ces compteurs ne prouvent ni progression utile ni validation. Les événements sont analysés dans la limite de lecture existante ; les octets supplémentaires sont drainés et comptés. Aucun contenu de message ni identifiant n’est ajouté à ces compteurs. Un délai peut ainsi être distingué d’une absence totale de sortie, sans attribuer arbitrairement sa cause au fournisseur. Les délais, budgets et règles de reprise restent inchangés.

Lors d’une reprise explicite, le délai est relu dans la configuration actuelle du vérificateur et enregistré sur la revue reprise. Les preuves, réservations et appels consommés sont conservés. Modifier ce délai ne relance rien et ne garantit pas que le fournisseur terminera.

### Préparer une reprise depuis le moteur

`swarm planning recovery-preview WORK --input demande.json`, avec `{"task_id":"ID"}`, expose une décision en lecture seule. Même contrat HTTP : `GET /api/v1/planning?work=WORK&action=recovery-preview&task=ID`. Le moteur vérifie candidat, preuves et journal, compte les inspections réutilisables et calcule les appels encore requis (inspections et réserve finale), disponibles et manquants, ainsi que le plafond minimal. Le refus de `retry-review` utilise le même calcul. Aucun appel ni augmentation de plafond ne résulte du précontrôle.

La décision distingue une autorisation budgétaire nécessaire et une correction de précondition nécessaire. Un budget suffisant ne démontre pas que la cause du dernier échec a disparu. Une revue finale déjà engagée, un ancien plan trop volumineux ou une preuve incohérente sont refusés explicitement : ce précontrôle couvre les inspections interrompues, pas toutes les reprises possibles.

Le diagnostic distingue également les messages assistant, les événements système et les événements `system/api_retry` déclarés par le fournisseur. Seuls des sous-types prédéfinis sont conservés ; les valeurs inconnues deviennent `other`. Un événement système ne démontre pas une analyse en cours. Les anciens incidents sans ces compteurs ne peuvent pas être requalifiés rétrospectivement.

## Réserves de contexte entre fragments

Une inspection `unknown` achevée reste un avis durable : elle ne vaut pas conformité. Elle est réutilisée sans refaire payer le même lot, et les lots suivants sont examinés. Un défaut `fail` reste un arrêt. La sélection finale reçoit toutes les demandes `needs`, identifiées par lot, pièce et index ; elle doit chercher les originaux nécessaires dans l’inventaire complet.

En présence de réserves, la réponse finale contient `review` et `resolutions`. Chaque réserve exige exactement une résolution indépendante, avec justification et, pour `resolved`, une citation originale visible. Une réserve oubliée, dupliquée ou une citation inventée invalide la réponse. Une réserve `unknown` ou `fail` interdit l’acceptation même si tous les critères de tâche annoncent `pass`. Une inspection sans demande explicite, une preuve altérée ou un contexte trop grand reste bloquant. Les tailles réelles sont recontrôlées, sans troncature.

La journalisation conserve les avis initiaux `unknown` sans les réécrire. À la publication, le moteur reconstruit les messages et vérifie à nouveau toutes les résolutions. Les anciens parcours sans réserve conservent leur format. Le moteur vérifie identités, couverture et citations ; la pertinence sémantique de la réponse appartient au vérificateur indépendant.
