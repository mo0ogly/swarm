# Revue de gros candidats : reprise bornée

État : faisabilité du transport du diff démontrée. Protocole d’exécution **non implémenté**. Cette page n’autorise pas une acceptation et ne modifie pas le plan enregistré.

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
