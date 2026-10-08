# Résumé

Les agents fondés sur des grands modèles de langage ne se contentent plus de produire du texte :
ils préparent des actions qui modifient le monde, dont certaines sont irréversibles, comme un
virement. Les organisations d'agents décrites par l'industrie acceptent une part d'erreur et la
corrigent après coup ; cette tolérance convient au code, qu'un commit suivant répare, mais pas à
un paiement, qu'aucune revue finale ne rattrape. Ce dossier défend une thèse simple : sur un
processus à effet irréversible, les agents doivent seulement proposer, un moteur déterministe doit
médiatiser chaque lancement, chaque reprise et chaque acceptation, et un exécutant déterministe
doit seul produire l'effet. Nous formalisons cette séparation par huit invariants du moteur et
trois propriétés de l'exécutant, la mettons en œuvre dans Swarm, moteur ouvert écrit en Go, et
l'évaluons sur un banc de facturation simulé soumis à des fautes injectées. Sur 9 000 exécutions
avec des agents scriptés, le moteur ne produit aucun paiement inexact ni aucun faux succès, là où
une chaîne sans moteur déclare réussi un règlement démenti par le grand livre dans 1 900 cas sur
3 000 ; la clé d'idempotence reste indispensable contre les doublons, et le contrôle coûte un
facteur 8 à 30 en durée. Avec un agent réel, aucune exécution ne produit d'effet faux, mais un
workflow fixe reprenant les mêmes contrôles fait jeu égal avec le moteur. L'évaluation a enfin
retourné la méthode contre le moteur lui-même : elle y a révélé cinq défauts, corrigés puis
soumis à une campagne de vérification. Le dossier se termine par le programme de recherche qui
sépare ce résultat d'une thèse : généralité, coordination par des modèles, attribution des
échecs, fondements formels.

# 1. Introduction

## 1.1 Des agents qui agissent

Pendant plusieurs années, un modèle de langage a été un outil de rédaction : il proposait un
texte, un humain le relisait, puis décidait. Les agents ont déplacé cette frontière. Un agent
enchaîne des appels d'outils, lit des fichiers, interroge des services et produit des actions ;
plusieurs agents coordonnés forment un essaim capable de mener seul un travail de plusieurs
jours. Les rapports d'ingénierie de Cursor sur les bases de code autonomes en donnent une
illustration frappante : des centaines d'exécutants, organisés en planificateurs et
sous-planificateurs, font évoluer un logiciel avec une intervention humaine minimale [Lin 2026a].

Ces organisations reposent sur un compromis assumé. Un intégrateur central, chargé de tout
vérifier avant chaque modification, est devenu un goulot d'étranglement ; exiger une correction
complète avant chaque commit sérialisait le travail. Le système accepte donc un taux d'erreur
faible mais non nul, en comptant sur des corrections ultérieures [Lin 2026a]. Ce compromis est
raisonnable pour du code : une erreur introduite aujourd'hui se corrige demain, et son coût se
limite au temps perdu.

## 1.2 L'effet irréversible change la question

Le même raisonnement ne tient plus lorsque l'action produit un effet qu'on ne peut pas annuler.
Un virement émis ne se rappelle pas ; une facture payée deux fois se récupère, au mieux, par une
procédure longue et incertaine ; un paiement adressé à un mauvais bénéficiaire peut être perdu.
Pour ces processus, l'erreur « faible mais non nulle » n'est plus un coût de productivité : c'est
une perte, parfois une fraude réussie.

Or les protections habituelles des systèmes d'agents agissent soit trop tôt, soit trop tard. Trop
tôt, lorsqu'elles portent sur la qualité de la réponse du modèle : un bon prompt, un modèle plus
fort ou une consigne de prudence réduisent la probabilité d'erreur sans l'annuler. Trop tard,
lorsqu'elles prennent la forme d'une revue finale : la revue découvre le doublon une fois le
second virement parti. Entre les deux, un ensemble de fautes ordinaires de tout système distribué,
comme une réponse perdue, un processus tué au mauvais moment ou deux lanceurs concurrents,
produit des effets doubles ou inexacts que ni le modèle ni la revue ne voient.

## 1.3 Thèse défendue

Ce dossier défend la thèse suivante.

> Sur un processus à effet irréversible, la sûreté d'un système d'agents ne peut pas reposer sur
> la qualité des réponses du modèle ni sur une revue finale. Les agents doivent seulement
> proposer ; un moteur déterministe doit médiatiser chaque lancement, chaque reprise et chaque
> acceptation ; un exécutant déterministe doit seul produire l'effet, et seulement sur le
> candidat validé. Le coût de ce contrôle doit être mesuré, pas supposé.

Cette thèse n'est pas neuve dans son principe. Elle reprend l'idée du moniteur de référence
formulée en 1972 : une médiation complète, que les sujets contrôlés ne peuvent pas contourner, et
assez petite pour être vérifiée [Anderson 1972]. Elle relève aussi du motif « planifier puis
exécuter » recensé pour la sécurité des agents [Beurer-Kellner 2025]. Sa contribution tient à
son application précise au cycle de vie d'un travail d'agents (tentatives, preuves, reprises,
acceptation) et à son évaluation sous fautes, avec et sans moteur.

## 1.4 Questions de recherche

Cinq questions organisent le travail. Les trois premières viennent du plan initial ; les deux
dernières sont apparues pendant l'évaluation.

| Question | Énoncé | État au 7 octobre 2026 |
|---|---|---|
| QR1, sûreté | Quels contrôles d'exécution empêchent les effets doubles, les acceptations sans preuve valide et les reprises sans cause corrigée ? | Répondue sur un scénario (chapitre 6) |
| QR2, coût | Quel surcoût ces contrôles imposent-ils ? | Durée mesurée ; blocages à tort non mesurés |
| QR3, attribution | Peut-on distinguer automatiquement une faute de l'agent, de l'environnement, de la règle et du moteur ? | Protocole pré-enregistré ; corpus non constitué |
| QR4, coordination | Les garanties tiennent-elles quand la coordination elle-même est confiée à des modèles ? | Étude conçue (chapitre 9) ; non menée |
| QR5, correction du moniteur | Comment un moniteur se met-il lui-même en défaut, et comment vérifier sa correction ? | Six constats ; campagne de vérification (chapitre 7) |

## 1.5 Contributions

1. **Un modèle** du travail d'agents médiatisé : acteurs et base de confiance, état, modèle de
   fautes, huit invariants du moteur (I1 à I8) et trois propriétés de l'exécutant (E1 à E3),
   avec la correspondance entre fautes et propriétés censées les arrêter (chapitre 3).
2. **Une réalisation ouverte** : le moteur Swarm et un banc de facturation à injection de fautes,
   en bibliothèque standard Python, publiés dans le même dépôt (chapitre 4).
3. **Une méthode et des résultats** : un protocole pré-enregistré, daté dans l'historique du dépôt,
   avec contrôles positifs, et ses résultats sur 9 000 exécutions scriptées et 90 exécutions avec
   un agent réel (chapitres 5 et 6).
4. **Le moniteur mis à l'épreuve** : six constats sur le moteur lui-même, leurs correctifs et une
   campagne de vérification pré-enregistrée de ces correctifs (chapitre 7).
5. **Un programme de recherche** qui situe ce qui reste à établir pour une thèse (chapitre 9).

La contribution d'attribution des échecs (QR3) n'est pas revendiquée : seul son protocole existe.

## 1.6 Organisation du document

Le chapitre 2 situe ce travail par rapport à l'orchestration d'agents, au contrôle à l'exécution,
à la reprise et aux agents financiers. Le chapitre 3 présente le modèle et ses invariants. Le
chapitre 4 décrit le moteur et le banc. Le chapitre 5 expose la méthode expérimentale, le
chapitre 6 les résultats, le chapitre 7 la mise à l'épreuve du moteur lui-même. Le chapitre 8
discute la portée et les limites des résultats. Le chapitre 9 propose le programme de thèse.
