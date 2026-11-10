# Premier incrément mémoire : une expérience réutilisable entre missions

Statut au 15 octobre 2026 : premier lot réalisé après « démarre », recette locale
et RETEX dans `RESULTATS-MEMOIRE-01.md`. Contrat courant :
[`../../PROJECT-MEMORY.md`](../../PROJECT-MEMORY.md). Exigences parentes :
`CADRAGE-MEMOIRE.md`.

Le texte ci-dessous conserve la proposition M2 qui a précédé la réalisation.
Les tableaux de contrats cibles ne décrivent pas à eux seuls le produit livré ;
le bilan précise les écarts, les preuves et les limites, notamment la sélection
manuelle et la garde avant le premier appel fournisseur.

Complément après la question sur la mémoire procédurale : ce parcours reste un
candidat. Le cadrage compare désormais rappel d’expériences et apprentissage de
consignes avant d’en arrêter la priorité ; ce document ne vaut pas couverture
complète des types de mémoire ni choix définitif de réalisation.

## Résultat attendu

Une mission A produit une leçon technique vérifiée. Une tâche pertinente de la
mission B, dans le même projet, peut retrouver cette leçon et recevoir son texte,
ses conditions et ses preuves. Après retrait ou remplacement, un nouveau départ
de tâche ne reçoit plus l’ancienne version comme connaissance active. Les
contextes historiques restent consultables et attribués.

Le premier incrément porte sur les expériences issues de RETEX. Les faits
généraux, préférences personnelles et extractions automatiques restent dans le
cadrage global ; cette livraison ne suffira pas à les déclarer réalisés.

## Point de départ vérifié dans Swarm

| Contrat actuel | Source | Conséquence pour le lot |
| --- | --- | --- |
| Un RETEX peut être enregistré, qualifié, relié à une tâche et vérifié. | `internal/engine/retex.go`, `retexAction`, `verifiedRetex` | Réutiliser la source, la distinction fait/hypothèse et les preuves d’acceptation ; ne pas reconstruire un extracteur. |
| La publication exige un fait sourcé et une tâche acceptée avec des preuves fraîches. | `verifiedRetex`, `retex-guide-preview`, `retex-guide` | Réutiliser la vérification au moment de l’admission. Une ancienne acceptation ne certifie pas la validité dans B. |
| Une leçon publiée bloque `retex-qualify` et `retex-status` ; une note différente ne peut pas écraser la note existante. | `retexAction`, `retexGuide` | Il manque un parcours public de révision/retrait de la connaissance publiée. Changer directement le statut du RETEX ne le remplace pas. |
| Le lancement peut charger l’index du guide ; les profils projet peuvent charger des textes par rôle. | `agents_store.go`, `project_profile.go` | Auditer ces deux routes lors du retrait : un nouveau rappel filtré ne suffit pas si un ancien chargement réintroduit la version retirée. |
| Les mutations de travail contrôlent révision et rejeu d’événement. | `store.go`, `mutateWithHookOnce` | Réutiliser la transaction et les refus de conflit, avec contrôle des références dans la même vue cohérente. |
| La recherche de dialogue concerne un travail précis. | `dialogue_context.go`, `dialogueHits`, `resolveDialogue` | Ajouter le rappel intermissions ; ne pas élargir implicitement les références de dialogue existantes. |

Ces observations sont statiques pour cette reprise. Les tests historiques de M1
et M2 gardent leurs périmètres ; aucun nouveau résultat d’exécution n’est déduit
de cette lecture.

## Choix proposés pour une première réalisation bornée

1. **Source et admission.** Partir d’un RETEX vérifié et de sa leçon, avec
   identifiants du travail et du RETEX, texte exact, empreintes des originaux,
   déclencheur, préconditions et périmètre de réemploi. L’opérateur du projet
   décide de l’admission, de la portée et des corrections. Un agent peut proposer
   une leçon ; ce texte ne lui accorde pas le droit de l’admettre. L’admission
   automatique par un superviseur serait une évolution de droits distincte.
2. **Persistance.** Préférer des versions et décisions conservées dans le
   stockage Swarm du projet, rattachées au travail source, avec les événements
   existants. Éviter une seconde autorité dans des fichiers modifiables : le
   Markdown publié reste une source ou un export. Les noms de champs et la
   migration seront définis avec les tests de stockage, avant modification du
   modèle ; aucun nouveau champ JSON n’est supposé déjà pris en charge.
3. **Recherche de référence.** Proposer, à la préparation d’une tâche, les leçons
   actives du même projet dont la portée déclarée correspond au rôle et au
   périmètre de la tâche. Commencer par des critères explicites et une recherche
   textuelle déterministe ; montrer les correspondances. La sélection finale
   est explicite dans l’aperçu de contexte. Mesurer ensuite si une sélection
   automatique ou sémantique apporte un bénéfice. Aucun score lexical ne vaut
   preuve d’applicabilité.
4. **Transmission.** Joindre le contenu de la version sélectionnée, ses sources,
   les conditions à revérifier et la décision de sélection. Enregistrer son
   empreinte dans le contexte de la tentative. Recontrôler activité, droits,
   références et empreintes avant l’envoi ; une modification depuis l’aperçu
   impose un nouvel aperçu. Absence de résultat et échec de lecture restent
   deux résultats distincts.
5. **Correction.** Le retrait conserve version, auteur et motif, mais rend la
   version indisponible aux nouveaux rappels. Le remplacement admet une nouvelle
   version sourcée et relie explicitement l’ancienne à la nouvelle. Une
   correction concurrente fondée sur une révision périmée est refusée ; un
   rejeu identique n’ajoute pas de version. Deux affirmations contradictoires
   sans décision de remplacement restent un conflit à exposer, pas à résoudre
   par l’horodatage. La détection sémantique générale de contradictions reste
   hors de ce premier lot.

Le retrait ne réécrit pas les messages déjà reçus par un agent en cours. Il doit
signaler les tentatives concernées au responsable ; leur poursuite ou reprise
relève du parcours public approprié. La garantie couvre les voies de chargement
gérées par Swarm et les références connues ; elle ne prétend pas effacer toutes
les copies libres de texte présentes dans un dépôt.

## Contrats publics à ajouter et vérifier ensemble

Les opérations ci-dessous sont des capacités cibles, sans nom de commande ou
d’endpoint préjugé. Leur API et leur CLI doivent appeler la même validation ;
l’UI expose les mêmes décisions et leurs refus.

| Capacité cible | Entrée nécessaire | Résultat et refus essentiels |
| --- | --- | --- |
| Admettre | Source RETEX vérifiée, contenu et empreintes, conditions, portée, révision attendue, événement | Version attribuée et événement ; refus de source inconnue/non vérifiée, portée interdite, empreinte ou révision périmée. |
| Chercher et préparer | Identité contrôlée du projet, travail/tâche/rôle destinataires, critères de recherche | Candidats, motifs de correspondance et exclusions accessibles ; aucun contenu d’un autre projet. La lecture ne décide pas l’admission. |
| Livrer | Sélection de versions et empreinte de l’aperçu | Texte réellement reçu et manifeste persistant ; refus avant appel fournisseur si une version choisie a changé ou a été retirée. |
| Retirer ou remplacer | Version attendue, motif, décision attribuée ; nouvelle source vérifiée si remplacement | Historique et relation conservés, nouveaux rappels actualisés ; aucun écrasement ni mutation partielle sur refus. |

Les bornes techniques existantes doivent être vérifiées dans le chemin retenu,
sans recopier leurs valeurs en nouvelles politiques. Toute nouvelle politique de
volume, durée ou sélection exige persistance, Admin, API, CLI et validation Go
commune. Aucun service différé ni appel LLM d’extraction n’est nécessaire à cette
référence initiale.

## Ordre des travaux et preuves de sortie

| Lot | Livrable concret | Dépendance et critère de sortie |
| --- | --- | --- |
| M3.1 Admission et cycle de vie | Schéma versionné, opérations communes, migration et tests de stockage | Choix de réalisation ; enregistrer, rouvrir, remplacer et retirer sans perdre l’original ; conflits et rejeux contrôlés. |
| M3.2 Rappel et contexte | Recherche intermissions, sélection et garde avant lancement | M3.1 ; B reçoit le texte exact, la provenance et les conditions ; version retirée/modifiée refusée, routes guide/profil couvertes. |
| M3.3 Parcours opérateur | UI et CLI pour admission, rappel, remplacement et retrait | M3.1/M3.2 ; mêmes validations, raisons visibles, recettes FR/EN et deux thèmes selon les surfaces modifiées. |
| M4 Recette comparative | Parcours A → B → correction/retrait, preuves et documentation FR/EN | M3.1–3.3 ; contenu reçu et résultat contrôlés, isolation entre projets, source manquante, concurrence et redémarrage. |
| M5 RETEX | Bénéfices observés, limites et proposition d’itération suivante | M4 ; aucune qualité LLM ni économie annoncée sans mesure. |

Pour la recette, réutiliser `MEM-C04` à `MEM-C10` de `cas-evaluation.json` dans
le périmètre des expériences. `MEM-C01` à `MEM-C03` restent des attentes du
chantier global, pas des capacités acquises par ce lot. Une fixture déterministe
peut vérifier le transport et la correction ; comparer le comportement d’agents
réels demande une configuration et un budget identifiés. Tous ces nouveaux cas
restent `NOT TESTED` pour le produit.

## Conditions restantes avant modification du moteur

Le choix ci-dessus est une recommandation technique à examiner, pas une adoption
implicite. La prochaine décision porte sur sa réalisation. Le premier travail
d’implémentation doit fixer le format versionné, la migration et la correspondance
avec les contrôles d’autorité réellement présents, puis écrire les tests de
contrat avant le code. La spécification des capacités ne crée aucun droit.
