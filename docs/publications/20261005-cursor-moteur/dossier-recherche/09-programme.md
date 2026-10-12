# 9. Programme de thèse

Les chapitres précédents établissent un résultat sur un scénario. Ce chapitre identifie ce qui
sépare ce résultat d'une thèse, décrit les axes de travail correspondants et propose une
organisation.

## 9.1 Ce qui manque

1. **Généralité.** Un seul processus métier, une API simulée. Une thèse demande au moins un second
   domaine à effet irréversible (livraison, attribution de droits d'accès, publication) et une API
   réelle en bac à sable, avec ses délais, ses rejets et sa conservation limitée des clés.
2. **Agents réels à l'échelle.** Cinq essais par scénario et un modèle. Il faut plusieurs modèles
   et fournisseurs, davantage d'essais, et des fautes propres aux modèles qui soient réalistes.
3. **Coordination par des modèles (QR4).** Dans toutes les campagnes, le responsable de mission et
   la revue sont scriptés.
4. **Attribution des échecs (QR3).** Le protocole existe ; le corpus et l'étiquetage non.
5. **Fondements formels.** Les invariants sont énoncés et testés, pas prouvés.
6. **Validité.** Auteur unique, pas de second annotateur humain, pas de revue par les pairs.
7. **État de l'art.** Les travaux les plus proches sont lus et situés ; une thèse demande un état
   de l'art complet sur la médiation à l'exécution, les transactions distribuées et la sûreté des
   agents.

**Comparaison à ajouter avant une conclusion sur l’apport propre de Swarm.** Un workflow fixe
scripté doit subir la même grille de fautes, avec les mêmes contrôles, le même exécutant et le
même parallélisme que Swarm. Des ablations ciblées (fraîcheur des preuves, garde de lancement,
reprise et persistance) isoleront les contributions. B1 est une garde minimale ; le résultat
contre B1 ne démontre pas une supériorité sur tous les systèmes de politiques à l’exécution.
La question est : quelles garanties supplémentaires apporte un orchestrateur durable à un
workflow doté des mêmes contrôles, et à quel coût ? Ce travail reste à réaliser.

## 9.2 Axe 1 — Attribution des échecs (QR3)

Quand une tentative échoue ou qu'une tâche se bloque, le moteur doit attribuer la faute au bon
responsable ; une attribution fausse a un coût direct, puisqu'une indisponibilité prise pour une
faute de l'agent déclenche une régénération du livrable. Le protocole pré-enregistré
(annexe B) définit cinq classes de cause, dérivées de MAST et étendues au moteur et à la règle :
agent, environnement, règle, moteur, inconnu. L’injection fournit une perturbation connue,
pas automatiquement la cause de chaque blocage : les traces doivent établir que la perturbation
a eu lieu, identifier les mécanismes intermédiaires et distinguer une cause initiale d’une cause
aggravante. Les mesures prévues sont la précision et le
rappel du diagnostic du moteur contre cette référence, l'exactitude de l'annotateur humain, et
l'accord entre l'annotateur humain et un annotateur fondé sur un modèle différent, rapporté comme
tel et jamais comme un accord entre humains. Un constat est acquis d'avance : le moteur n'a pas de
catégorie pour la règle ni pour lui-même, et ne peut donc jamais attribuer un échec à l'une
d'elles.

## 9.3 Axe 2 — Coordination par des modèles (QR4)

L'étude S-collectif confie à des agents réels les trois rôles de décision : préparateur,
responsable de mission et vérificateur ; le règlement reste déterministe et seul porteur du jeton.
Le responsable y crée lui-même les tâches, ce qui expose le moteur aux erreurs de planification.
Un fait du moteur en conditionne la conception : un responsable ou un vérificateur fondé sur un
modèle est lancé sans aucun outil, si bien qu'il ne peut que proposer des opérations, toutes
validées par le moteur.

Une [observation de terrain du 10 octobre 2026](revisions/20261010-vivacite-supervision.md)
montre une revue et une décision de correction par des modèles réels dans la
mission d'évolution Cursor, hors de S-collectif. Elle ajoute une question de
vivacité : un contrôle de durée peut empêcher le superviseur de traiter le défaut
qu'il est chargé de diagnostiquer. L'essai comparatif proposé sépare durée totale,
silence, bail renouvelé et correction supervisée ; il doit être pré-enregistré
avant collecte. Cette observation ne remplace pas la campagne S-collectif.

Une [seconde observation sur le dossier de revue](revisions/20261010-refus-contexte-revue.md)
montre une correction supervisée qui ajoute des citations sans transmettre leur
source complète. Elle propose de comparer les modes de transmission des preuves,
à critères, fournisseur et budgets figés, en comptant séparément les inconnus
justifiés et les acceptations insuffisamment prouvées. Ce cas après incident
n'ajoute aucun résultat à S-collectif.

Deux conditions ne diffèrent que par la règle de prérequis du correctif D6 : avec la règle
(« garde »), le moteur retient le règlement si le responsable oublie la dépendance ; sans elle
(« libre »), seul l'exécutant l'arrête. Leur comparaison mesure ce que vaut la garantie face aux
erreurs d'un responsable réel. Trois fautes propres à la coordination s'ajoutent : une consigne
injectée dans un livrable remis, une revue favorable sur un contrôle en échec, et une exigence mal
rattachée par le responsable, observée sans être injectée. La question posée n'est pas « le modèle
s'est-il laissé manipuler ? », mais « la manipulation a-t-elle produit un effet ? ».

## 9.4 Axe 3 — Fondements formels

### Traçabilité comme exigence de développement

Le [contrat de traçabilité](../../../architecture/TRACEABILITY.md) relie objectif,
décision, tentative, candidat, exécution, revue et acceptation. Ses repères TR-1
à TR-6 précisent les lots existants du moteur : attribution des preuves, contexte
des superviseurs, remise au propriétaire, concurrence, reprise publique et mesures.
La [note de travail](revisions/20261010-tracabilite-developpement.md) sépare ces
exigences du constat intermédiaire T2 et des vérifications effectivement réalisées.
Tester l’utilisation de ces informations par les responsables et l’attribution
des manques ; une documentation ou un inventaire fourni ne prouve pas leur usage.
La [démarche autorécursive](revisions/20261010-demarche-autorecursive.md) ajoute
le contrôle du diagnostic et de la transmission eux-mêmes : une erreur de méthode
devient un problème lié, puis la correction revient à l’exigence initiale.

### Étude du parallélisme et des dépendances

Le [contrat de parallélisme des agents](../../../architecture/AGENT-PARALLELISM.md)
sépare sous-planification, production, vérifications locales et publication.
La [décision de plan](revisions/20261010-parallelisme-plan.md) conserve le graphe
séquentiel de la mission ; elle ne démontre pas la concurrence.

Comparer travaux indépendants, chaînes de résultats et écritures communes, à
objectif et contrôles identiques. Mesurer chevauchement d’activité utile, temps
total, attente centrale, résultats acceptés, conflits, reprises et coût connu
ou inconnu. Distinguer capacité et débit, décisions de modèles et interventions
humaines. Fixer conditions et répétitions avant collecte ; cette documentation
n’ajoute aucun gain ni résultat aux campagnes historiques.

Le [défaut de phases de publication](revisions/20261010-phases-plan-integration.md)
ajoute un cas d’attribution au moteur : un avis indépendant favorable ne rendait
pas une acceptation utilisable, car la preuve générée contredisait le plan.
La garde des dépendances bloquait correctement la suite. Étudier séparément
la qualité du livrable, sa preuve structurée et le diagnostic du responsable.

Les invariants I1 (effet unique par intention), I2 (pas de mutation sur un état périmé) et I4
(lancement atomique et exclusif) se prêtent à une vérification par modèle sur de petites instances,
en TLA+ ou Alloy : requêtes concurrentes, rejeu, révisions, bail périmé. L'objectif n'est pas de
prouver tout le moteur, mais de rendre vérifiable le noyau sur lequel repose la médiation, dans
l'esprit du moniteur de référence, qui doit être assez petit pour être vérifié.

## 9.5 Axe 4 — Généralité

Un second domaine et une API réelle en bac à sable éprouveraient deux hypothèses du modèle : que
l'effet se décrive par un candidat structuré avant exécution, et que l'API déduplique correctement
par clé. Le second point est connu pour varier d'un fournisseur à l'autre ; le modèle actuel ne
couvre pas la réconciliation qu'exige un fournisseur défaillant.

## 9.6 Axe 5 — Fautes propres aux modèles

La fraude au changement d'IBAN testée a été déjouée par le modèle lui-même. Il faut des fautes
plausibles, construites pour tromper un modèle compétent (lot cohérent mais faux, montant arrondi
de façon crédible, facture oubliée), et plusieurs modèles, dont certains dédiés à la revue, pour
évaluer des revues décorrélées.

## 9.6.1 Apprentissages et améliorations après recette

La [révision du 11 octobre](revisions/20261011-instruction-learning-retex.md)
ajoute deux objets à évaluer : l’apprentissage attribué dans les consignes des
tâches futures, et les propositions de quick wins issues d’une recette acceptée.
Comparer pertinence, réception effective, effet sur l’erreur récurrente et rejet
utilisateur. Distinguer le protocole vérifié avec fixtures de l’efficacité avec
modèles réels. Conserver les sources, les coûts cumulés et les interventions,
puis mesurer les incréments adoptés plutôt que compter les propositions.

## 9.7 Organisation proposée

| Chapitre de thèse | Contenu | Matériau disponible |
|---|---|---|
| 1. Introduction | problème, thèse, questions | chapitre 1 de ce dossier |
| 2. État de l'art | orchestration, contrôle à l'exécution, reprise, agents financiers | chapitre 2, à compléter |
| 3. Modèle et invariants | acteurs, état, fautes, I1 à I8, E1 à E3 | chapitre 3 |
| 4. Vérification formelle du noyau | spécification de I1, I2, I4 | à faire (axe 3) |
| 5. Réalisation | moteur et banc | chapitre 4 |
| 6. Évaluation sous fautes | méthode, campagnes scriptées et réelles | chapitres 5 et 6 |
| 7. Le moniteur mis à l'épreuve | défauts, correctifs, vérification | chapitre 7 |
| 8. Attribution des échecs | QR3 | à faire (axe 1) |
| 9. Coordination par des modèles | QR4 | à faire (axe 2) |
| 10. Généralisation | second domaine, API réelle, plusieurs modèles | à faire (axes 4 et 5) |
| 11. Discussion et conclusion | portée, limites, perspectives | chapitre 8, à étendre |

## 9.8 Étapes et risques

La première étape est la publication d'un préprint qui date les contributions acquises (modèle,
réalisation, évaluation, mise à l'épreuve du moniteur), après la fin de la campagne de
vérification et une relecture indépendante des analyses. Viennent ensuite, dans l'ordre où ils
dépendent les uns des autres : l'attribution des échecs, qui n'exige que le corpus scripté ; la
coordination par des modèles, qui exige les correctifs vérifiés ; la vérification formelle, menée
en parallèle ; enfin la généralisation.

Trois risques principaux pèsent sur ce programme. Le coût des campagnes avec agents réels croît
avec le nombre de rôles confiés à des modèles ; il doit être mesuré par des pilotes avant tout lot.
L'auteur unique de l'ensemble limite la validité ; une collaboration pour l'annotation et la
relecture est nécessaire. Enfin, le moteur évolue : chaque résultat doit rester attaché à un
commit figé, et les résultats historiques ne doivent jamais être recalculés sur un moteur corrigé.

# 10. Conclusion

Sur un processus de paiement simulé, séparer la proposition de l'action supprime, dans nos
conditions, les deux défauts les plus graves d'une chaîne d'agents : payer un candidat modifié ou
périmé, et déclarer réussi un règlement qui ne l'est pas. La clé d'idempotence reste nécessaire, et
le contrôle a un coût mesuré. Avec un agent réel, l'architecture où l'agent propose et des
composants déterministes contrôlent et exécutent ne produit aucun effet faux, sans que ce lot
distingue le moteur d'un workflow fixe. Enfin, la même méthode appliquée au moteur y a révélé des
défauts, dont les correctifs ont été vérifiés selon le même protocole. Ce dernier point est peut-être
le plus utile : un moniteur qui prétend garantir la sûreté d'agents doit accepter d'être évalué
avec la même exigence que les agents qu'il contrôle.
