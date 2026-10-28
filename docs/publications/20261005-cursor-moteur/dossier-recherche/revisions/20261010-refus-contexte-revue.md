# Deux refus de revue : transmission incomplète des sources — 10 octobre 2026

Statut : étude exploratoire après incident, sur une seule tâche réelle. Ce cas
prolonge l'[observation de vivacité](20261010-vivacite-supervision.md), mais concerne
une autre cause. Aucun résultat des campagnes de facturation ni verdict D1–D6
n'est modifié. Les [observations structurées](20261010-refus-contexte-observation.json)
conservent les identités, empreintes et décisions publiques.

## Question et résultat observé

Comment une correction supervisée peut-elle consommer la dernière tentative sans
rendre le résultat vérifiable ? Sur `plan-ca937edf4e-T1`, mission
`w-ca937edf4e0abd85f8d3d67b`, les deux revues achevées concluent
`changes_requested` : critères 1, 2 et 4 `pass`, critère 3 `unknown`.
Ce dernier verdict exprime une preuve insuffisante ; il n'établit pas que
l'analyse des sources est fausse ni que les citations sont falsifiées.

Le producteur devait comparer l'architecture à Cursor, en conservant janvier et
février, puis examiner les sources complémentaires avant décision. Il avait lu
`fiches-lecture.md` dans sa copie et cité ce fichier dans son bilan de livraison.
Le vérificateur indépendant, sans outil, recevait le diff, le rapport et les
reçus de contrôle. Le fichier cité était inchangé et absent des sources jointes.
Le contrôle mécanique annonçait explicitement « semantic review still required ».
Un contrôle réussi et une déclaration `pass` du producteur ne prouvaient donc pas
l'examen sémantique demandé.

## Chronologie et rôle du superviseur

Heures UTC, le 10 octobre 2026 :

| Événement | Observation |
| --- | --- |
| 17:29:47.604934808 → 17:32:34.000739990 | `review-849606d4679df263de116295`, candidat `38345bb1805ef56137087b3326d19f2c4d8cf204` ; 166,40 s, source complémentaire absente |
| 17:34:44.831099302 | Le responsable racine demande une correction ciblée : citations exactes dans le diff ou mise à jour du fichier source |
| 17:34:48.886596130 → 17:45:01.638089713 | Deuxième tentative ; citations et numéros de ligne ajoutés au rapport, fichier source complet toujours absent du contexte de revue |
| 17:45:20.996785809 → 17:47:31.909131270 | `review-c96e0e832eb54fc6b28db2bc`, candidat `6392cd32a2ce77a86b6b91c36098f635d4ffecf1` ; 130,91 s, même critère `unknown` |
| 17:48:36.818298678 | Le responsable constate 2/2 tentatives consommées et ne propose aucune opération ; les tâches dépendantes restent en attente |
| 17:51:53.558380294 | Réparation externe explicite remise par l'opération publique `planning revise-recovered-result` |
| 17:52:16.200443736 → 17:55:05.414020512 | `review-bc4a136c4713357aed3c56b9` ; 169,21 s, quatre critères `pass`, puis T1 accepté par le moteur |

Le superviseur a bien reçu les refus et décidé la première correction. Il a
ensuite respecté la limite ; il n'était ni absent ni interrompu à 90 secondes.
Sa première consigne corrigeait la présentation des citations, sans garantir
l'accès à l'original. Sa seconde décision ne mobilise pas la réparation externe.
Cette intervention reste donc manuelle : elle ne démontre pas une récupération
autonome par Swarm ni l'efficacité des sous-planificateurs, absents de ce cas.

## Cause établie et hypothèse de conception

Les deux contextes archivés ne contiennent aucune entrée `sources`. Leurs
empreintes sont vérifiées contre les événements publics. En revanche, le
mécanisme `managed_review_sources.go` sait déjà fournir les fichiers complets,
depuis le candidat Git testé, avec chemin, identité de blob, taille et SHA256.
Il nécessite une déclaration explicite :

```json
{"version":1,"files":["docs/publications/20261005-cursor-moteur/article-scientifique/fiches-lecture.md"]}
```

Cette déclaration se place dans `docs/plan-ca937edf4e-T1.review-context.json`.
Un chemin dans `delivery.json.criteria[].evidence` ne joint pas automatiquement
son contenu. Une copie de citation produite par l'auteur ne fournit pas non plus
une référence indépendante à son original.

La cause immédiate établie est l'absence du blob source dans le dossier transmis.
Une hypothèse de conception est le manque de visibilité du mécanisme dans les
consignes : le guide [de reprise](../../../../ATTEMPT-RECOVERY.md#donner-au-vérificateur-les-sources-du-candidat)
le décrit, mais les recherches dans `managed_delivery_contract.go`,
`agents_store.go`, le contrat commun et les méthodes APEX/replan ne trouvent pas
`review-context.json`. Ce constat de texte ne prouve pas à lui seul la cause du
choix du modèle. Il justifie un essai sur la construction des dossiers et les
consignes du responsable, plutôt qu'une attribution automatique au producteur.

## Réparation retenue et preuve de transmission

Deux voies étaient possibles : modifier artificiellement le fichier original
pour le faire entrer dans le diff, ou déclarer sa transmission avec le mécanisme
existant. La seconde conserve ses octets et indique clairement son rôle de preuve.
Elle évite de confondre une correction du résultat avec une correction du dossier.

Codex a modifié la copie du producteur terminé, puis remis un arbre examiné par
l'opération publique de réparation. Le rapport désigne explicitement cette
intervention externe. Une erreur factuelle eBPF a également été corrigée : la
valeur 2 de `unprivileged_bpf_disabled` reste modifiable par l'administrateur ; la
valeur 1 est irréversible sur le noyau courant, selon le
[guide officiel Linux](https://docs.kernel.org/admin-guide/sysctl/kernel.html#unprivileged-bpf-disabled).
Aucun réglage noyau ni privilège n'a été changé. Ce second changement empêche
une attribution expérimentale strictement isolée à l'ajout du manifeste.

La source jointe mesure 23 612 octets, blob
`fe87c05dbbd02b528b30412bb6277468b0fcfe44`, SHA256
`1c7e84b87fc2e7baa53383be0c494658f6d0c1615962daf40511d99c7301934a`.
Ses octets sont identiques à ceux du candidat refusé. Le nouveau contexte contient
son texte complet dans `sources[]`, sans troncature. Les sept contrôles du moteur
ont réussi sur le candidat `6da19fd7700cfc540fc0e078b94f9c56453ca3f5` ; la
revue indépendante fraîche a ensuite jugé les quatre critères favorables.
Le commit de résultat externe `20c2961aecf9bd9332a01e3abdd52c38a1244bec` et le
candidat composé soumis à la revue ont des identités distinctes.

Les deux tentatives restent consommées, les limites restent 2 tentatives et
80 appels d'outil par tentative, et les appels de revue continuent d'être comptés.
Les avis antérieurs restent conservés dans les événements et les preuves.
Aucune acceptation opérateur ni modification directe de base n'a servi à la reprise.

## Portée et évolution à vérifier

Le dossier incomplet a entraîné deux refus et une tentative corrective qui n'a
pas résolu la condition de vérification. Sans dossier contenant l'original, nous
ne pouvons pas qualifier ces refus de faux négatifs. Nous pouvons établir le coût
de la coordination supplémentaire, sans chiffrer une économie hypothétique.
Cette observation n'établit aucune supériorité de Swarm sur Cursor ou un autre
système, ni une conformité globale au contrat JAN/FEB.

Pour T5, proposer une consigne de remise explicite et un diagnostic du dossier
qui montre quelles sources sont réellement jointes au vérificateur. Une source
présente ne garantit jamais un verdict favorable. Un contrôle préalable doit
refuser une source manquante ou trop volumineuse de façon expliquée, sans
substituer une revue favorable à un critère inconnu. Les règles d'acceptation et
les politiques de reprise doivent rester distinctes.

## Comparaison à pré-enregistrer

Sur des tâches et budgets identiques, comparer : simple chemin de preuve,
citations incorporées, manifeste explicite et manifeste accompagné d'un contrôle
de complétude avant appel. Figer fournisseur, modèle, critères, version du moteur
et textes ; varier uniquement la transmission. Inclure citations exactes et
fausses, source contradictoire, fichier absent, source trop volumineuse, nouvelle
révision et conflits de publication.

Mesurer séparément sources réellement reçues, inconnus justifiés, acceptations
sans preuve suffisante, diagnostics corrects, nombre de reprises, coût et durée.
Un ajout de citations ne compte comme correction que si la précondition signalée
par la revue change. Pré-enregistrer le protocole avant les répétitions ; cette
étude a posteriori sur une tâche n'en est pas le résultat.

## Résultat de la reprise

Observation ajoutée après revue le 10 octobre 2026 :
`review-bc4a136c4713357aed3c56b9` termine à 17:55:05.414020512 UTC avec les quatre
critères `pass`. Sa justification indique que les citations ont été comparées à
`fiches-lecture.md` fourni dans le contexte, avec empreinte identique. Le moteur
accepte T1. À la révision publique 36, les deux tentatives restent consommées,
les limites restent inchangées et les appels de revue sont à 4/30. T2 est encore
`todo` et le responsable racine a repris la planification de la suite.

Ce résultat corrobore le diagnostic de preuve manquante sur ce cas. Il ne prouve
pas une amélioration générale du taux d'acceptation ni une économie de coût.
La réparation reste manuelle, la revue indépendante et l'acceptation restent
celles du moteur. Les deux refus ne sont ni effacés ni réécrits.

## Correctif global ultérieur du 10 octobre 2026

Les constats sur le chemin de preuve décrivent le moteur au moment des deux
refus. Le [correctif global](20261010-correction-globale-preuves.md) joint désormais
les fichiers déclarés dans la livraison, même inchangés, et remet l'inventaire
des sources aux responsables racine/enfants. Cette évolution ne réécrit pas les
contextes historiques et ne transforme pas la réparation manuelle T1 en reprise
autonome démontrée. Voir les schémas du [contrat d'architecture](../../../../architecture/REVIEW-EVIDENCE.md).
