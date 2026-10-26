# Scaling agents — reprise APEX du portage Rust, 25 octobre 2026

[English](../../en/plans/migration-rust/SCALING-AGENTS-20261025.md) ·
[Contrat et schémas finaux](../../architecture/CURSOR-ENGINE-CONTRACT.md)

## Mise à jour S1 — remise au propriétaire et reprise qualifiées

La documentation S0 ci-dessous est historique ; S1 a ensuite été intégré et installé
en Rust dev65 (`11414fafac47e3ca4db8fe9146b4b027b787d35bc39b50c0167a92bd518c0a8f`).
Le présent incrément ajoute une recette publique maintenue sur ce profil natif
normal exact, sans modifier de source de production ni redémarrer le service.
Deux parcours Root isolés et le couple final témoin/refus en sous-projet passent.
Après refus sur le chemin du rapport, la ref Git reste conservée ; le même commit
et rapport sont remis avec la copie originale déplacée, sans worker supplémentaire,
publication du candidat ni acceptation. Cinq tests nommés couvrent l'injecteur,
la contre-pression des canaux, stdout invalide et les erreurs de reprise/restauration
conjointes. Intégrité de la recette finale `VERIFIED`
([preuves](evidence/20261025-scaling-recovery-tl7d1g71/recipe-closure/result.json)).

GLM a fourni la proposition d'injecteur ; erreur masquée et test invalide ont été
corrigés avant intégration. Une revue large épuisée ne produit aucun verdict.
La revue indépendante réduite repère la contre-pression du stderr non drainé,
reproduite avant correction ; sa reprise ciblée confirme le correctif. L'affirmation
ultérieure imposant de recréer la copie est rejetée par le vrai consommateur du
worker terminé et le parcours sans copie. Son risque lié de masquage au nettoyage
a été reproduit séparément et corrigé : restauration et erreur initiale sont toutes
deux conservées. Stdout brut reste retenu avant décodage JSON. Avis statiques GLM
et assertions exécutées restent distincts ([rapport et RETEX](evidence/20261025-scaling-recovery-tl7d1g71/RESULT.md)).

Ce contrôle provoque un refus de forme du chemin avant écriture finale ; il ne
prouve ni ENOSPC réel, ni échec pendant l'écriture, ni durabilité après coupure.
Ces cas disque restent ouverts sous la responsabilité S1. S2 contrôles et validateur
final managed, S3 fin globale et S4 mesures de la boucle complète restent ouverts.
Aucune nouvelle politique opérateur, aucun appel métier, commit ou push. Les
mentions de tranche documentaire seule ci-dessous concernent S0.

## Besoin et frontière

L'utilisateur demande APEX, relecture détaillée de Cursor avant les délégations
GLM SKYNET, puis mise à jour documentaire avec Mermaid en fin de page. Il confirme
le **validateur final indépendant** de Swarm. Les deux articles ont été relus ;
janvier, les étapes de février et nos propres garanties restent distincts.
Le besoin de migration demeure ouvert : documentation, réponse GLM et preuve Go
ne démontrent pas une boucle Rust livrée.

Une tranche documentaire est réalisée : principes, responsabilités, dépendances
et manques réels du portage, FR/EN. Aucun fichier moteur, politique, donnée de
mission, budget ou processus de service modifié dans cette tranche.
Les sources sont lues avec leurs producteurs, pas assimilées au binaire servi.
Le précédent constat local dev.59 reste une observation distincte.

## S2 en cours — construction et protocole de preuve

Le producteur `managed_repository_candidate.rs` fusionne résultat/candidat précédent
dans une copie possédée, crée un commit technique à parent unique et reset avant
contrôles. Trois cas Git unitaires réels passent initialement ; un quatrième test
scoped existant était sélectionné. Ce n'est ni une validation publique ni une
installation. La première commande omettait `--lib` :100exécutables d'intégration
compilés inutilement n'ont exécuté aucun test et ne comptent pas comme PASS.
Seuls ces caches nouveaux ont été retirés après conservation des empreintes
(11 452 343 912octets) ; sources/preuves, binaire unitaire et releases sont préservés.
AGENTS impose désormais `--lib` ou `--test TARGET` ; Claude lit ce fichier commun.

Les contributions GLM de segmentation/catalogue sont adaptées dans
`managed_review_citation_json.rs`, `managed_review_citations.rs` et
`managed_review_reply.rs` : guillemet perdu, références littérales erronées et
causes codec effacées corrigés. Les plages Go768octets/fin courte8 sont des règles
de représentation sans perte ; les limites de citation8/1000 deviennent les bornes
de revue enregistrées fournies au schéma et au parseur. Le consommateur Root et
le schéma fournisseur restent à raccorder : effet de configuration non prouvé.

Le parseur traite candidat exact et cohorte complète, doublons/exclusivité des
champs v2, empreintes et citations attribuées, fail/unknown et bornes de transport.
Il ne crée aucune autorité. Lots/fragments restent refusés. Le premier lot unitaire managed est vérifié ; revue finale liée aux octets, parcours public de
contrôles/reviewer, publication, profil normal et installation restent ouverts.
Le correctif Git S1 est installé en dev67. Assemblage du catalogue réel, propriété du candidat et contrôles
cumulatifs doivent être raccordés avant de retirer une garde publique.

Contrôle ciblé actuel : **31 tests unitaires managed PASS** (sources figées, sans feature fixture). Les empreintes brutes refusent les modifications masquées par assume-unchanged/skip-worktree et permettent la reprise après restauration. Le correctif partagé de conservation neutralise les callbacks Git locaux ; le contre-exemple natif dev65 est conservé. Revue GLM ciblée et témoins Git casse/priorité terminés, recette native du nouveau profil et installation encore ouvertes. Ces résultats ne raccordent pas S2 au Root.

[Preuves S2](evidence/20261025-scaling-s2-oi9dgo_e).

## Choix d'architecture

1. Porter tous les anciens fichiers Go avant de rechercher le parcours public :
   risque de doublonner les producteurs Rust existants et de multiplier les lots
   de gardes sans résultat utilisable. Non retenu comme ordre de livraison.
2. Raccorder une boucle utile à partir des producteurs existants : worker géré
   terminé → candidat immuable et retour attribué → décision du propriétaire →
   contrôles et validation finale → publication protégée → clôture sous preuves.
   Retenu. Les branches indépendantes progressent en parallèle ; une transition
   atomique ne justifie pas un verrou global pendant un appel modèle.

Le validateur final statue sur le candidat complet et les interactions. La racine,
en rôle de juge global, statue sur le besoin ; le moteur vérifie et applique.
Aucun rôle intégrateur central supplémentaire, aucune acceptation faible ou
suppression des refus actuels n'est décidé par ce plan.

## Séquence et critères observables

| Lot | Responsable et périmètre | Fin observable et dépendances |
| --- | --- | --- |
| S0 — principes et contrat | Principal, docs FR/EN ; revue GLM bornée | Les rôles et limites Rust sont explicites ; trois schémas finaux par langue. Réalisation documentaire uniquement. |
| S1 — résultat géré et retour | Principal intègre des propositions GLM ; G3/G4, producteurs/copie/attribution et relais au propriétaire | Un résultat immuable courant est attribué et reçu exactement une fois ; reprise conserve identité, base, preuves et dépenses. Un autre worker peut continuer. La publication protégée/acceptation ne sont jamais simulées par un simple rapport. |
| S2 — validateur final | G3/G4, dossiers/journaux/contrôles/revue et vrais consommateurs | Avis sur le candidat complet, réserves et interactions ; même candidat que les contrôles, refus/unknown/périmé et reprise/rejeu sans double dépense. Lots/fragments restent refusés avant port complet du contrat concerné. |
| S3 — fin globale | Racine et moteur, après S1/S2 | `end` autorisé seulement avec résultats et preuves actuels ; `end` prématuré refuse sans mutation. Le refus Rust présent n'est pas retiré seul. |
| S4 — mesure et RETEX | Principal, vérification distincte | Plusieurs workers, retour pendant travail parallèle, refus/correction/reprise ; calcul/attente/contrôle/revue/publication séparés. Fournisseurs factices et modèles réels ne sont pas confondus. |

Le code G3/G4 sera risque A : témoin valide avant injection, sources figées,
revue des politiques/fonctions/erreurs/documentation liée aux octets, contrôles
ciblés et une recette complète de tranche. Réglages opérateur persistés avec
Admin/API/CLI/consommateur. Vérifier le binaire réellement installé, ressources et
notes FR/EN après livraison autorisée par le lanceur pris en charge.
Aucune reprise d'une mission réelle ni appel fournisseur métier n'est créé ici.

## Délégations et décisions du principal

La route SKYNET a changé de précondition par rapport au blocage DNS historique :
DNS résout et `/models` annonce `glm-5.2`, limite déclarée 524288. La version réelle
GLM 5.3 indiquée par l'utilisateur reste distincte du nom API ; aucun nouveau test
de remplissage ni de tarif. Deux appels bornés, sans relance automatique, effort
low ; résultats et entrées hachées dans le dossier de preuve existant de tranche.
Ils n'ont ni outils ni accès au dépôt : leurs constats sont vérifiés avant usage.

- Revue des principes : correction retenue de l'ambiguïté juge/validateur/racine,
  clarification du périmètre Go historique et de la présentation de 53,149 s,
  sans modification de la mesure d'origine. Suggestion de seuil/refus sur contention
  non adoptée : il faut d'abord mesurer, pas ajouter une politique arbitraire.
- Proposition de parcours Rust : utile pour nommer les types à fournir avant code,
  mais ses affirmations « non portés » sur RootStore, LocalDirectory, fences et
  l'observation managed sont rejetées : les sources fournies les appellent déjà.
  Contexte non fourni n'est pas absence d'implémentation. Refaire cette observation
  comme premier lot ne constituerait pas un progrès produit. Aucun patch moteur
  intégré et aucune acceptation déduite de cette réponse.

Prochaine contribution GLM : une interface réellement manquante de S1, avec
producteur, consommateur, types et gardes complets, plutôt qu'une nouvelle vague
identique. Le principal reste responsable de l'intégration et des vérifications.
Les travaux Trust, mémoire, budget et consignes des autres tranches sont préservés.

## Vérification et transmission de la tranche documentaire

Documentation explicite des rôles et limites, liens locaux et absence de défauts
de diff à vérifier sur les fichiers modifiés. Le rendu local Mermaid échoue d'abord
sur un Chrome attendu absent par la version de Puppeteer du CLI. Le Chrome système
existant est ensuite sélectionné explicitement ; originaux de l'échec conservés,
aucune installation de dépendance, aucun changement de sécurité du produit.
Le contrôle positif précède les cinq rendus restants. Les images et leur lecture
visuelle qualifient les schémas, pas le moteur.

Pas de compilation/installation moteur requise pour ces deux pages documentaires :
aucun contrat exécuté ni ressource applicative n'est modifié. Ce texte n'est pas
injecté automatiquement dans un agent déjà démarré. La prochaine délégation doit
recevoir explicitement les rôles et le périmètre nécessaires. Les deux requêtes GLM
présentes conservent les contextes effectivement reçus ; elles ne prouvent pas une
transmission complète aux futurs rôles natifs Swarm.

Les trois Mermaid de hiérarchie, validation et manques Rust sont placés à la fin
du [contrat canonique](../../architecture/CURSOR-ENGINE-CONTRACT.md#schémas-mermaid--cible-et-limites-de-la-migration),
avec les mêmes frontières dans sa version anglaise.

Correctif Git partagé livré dev67 : add et write-tree protégés ensemble, recette native forcée PASS/VERIFIED, profil normal424entrées et14assertions UI FR/EN. [Résultat/RETEX](evidence/20261025-scaling-s2-oi9dgo_e/S1-GIT-RESULT.md). S2 reste ouvert.

Reprise GLM S2 : défaut clean-filter du vérificateur reproduit puis supprimé ; trois tests réels Git PASS et recette managed-candidate-custody PASS/VERIFIED. Original du checkpoint, refus et restauration conservés. Aucun raccordement Root/validation finale revendiqué. Preuves : `evidence/20261025-scaling-s2-oi9dgo_e/resume-glm-candidate/`.

S2 : préflight de la cohorte de contrôle consommé avant création du candidat. Une politique absente dans une tâche acceptée produisait auparavant un candidat (contre-exemple Git conservé) ; le refus et la reprise sont maintenant couverts. Original/normalisé par tâche, ordre conservé, IDs de transport distincts, aucun plafond agrégé. Le digest métier ne suffit pas au futur job : politiques normalisées, réglages et candidat physique devront être liés. Documentation FR/EN et quatrième Mermaid explicitent tâche/agent/job. Preuves et statut de livraison : `evidence/20261025-scaling-s2-oi9dgo_e/native-controls-glm/`.

S2/job : le calcul natif acceptait un résultat portant un contrôle étranger dans une fixture de données (contre-exemple avant correction conservé, aucune exploitation sur mission réelle démontrée). Mapping politique/créneau/résultat maintenant vérifié avant conversion publique, original et rejeu. Les résultats absents/infructueux non marqués en échec refusent ; les vrais échecs/non-tentatives restent bloqués et conservés. La cohorte S2, ses témoins physiques et le validateur final restent à raccorder. Preuves/recette/installation : `evidence/20261025-scaling-s2-oi9dgo_e/native-job-binding/`.

S2/origine : nouvelle liaison du journal natif, pas encore le candidat géré. Origin scalaire et politique effective sont enregistrés au claim ; empreinte de domaine avant overlay mission, mêmes lecteurs de permission/publication/revue. Reçu privé v2 exige les deux champs, v1 les omet sans réencodage ni backfill. Le témoin public dev70 conservé manque cette origine ; nouveaux tests vérifient préimage et absence de downgrade implicite. GLM a trouvé la préimage de contrôles manquante dans la première proposition : corrigée avec effective_policy. Preuves/recette/livraison : `evidence/20261025-scaling-s2-oi9dgo_e/native-source-origin/`.

S2/préimage candidat : défaut reproduit — une modification du délai par défaut conservait le même candidat tout en changeant les contrôles normalisés. Checkpoint privé v2 avec plan cumulatif complet et refus sans remplacement ; v1 non lié conservé/refusé. La recette historique de mapping natif attend désormais les trois tests réellement présents (ancienne preuve dev70 inchangée). Vérification et livraison suivies dans `evidence/20261025-scaling-s2-oi9dgo_e/candidate-controls-rbro0sp6/`. S2 natif/final reste ouvert.

Complément S2/préimage : GLM a proposé une liaison au checkpoint. Intégration principale renforcée : empreinte de domaine sur le checkpoint complet, y compris observations physiques et sources suivies, sans autorité implicite ; test du producteur/lecteur et des liaisons absentes/altérées. La proposition omettait ces observations et ses assertions ignoraient l’enveloppe d’erreur réelle : non reprises telles quelles. Preuves finales dans le sous-dossier `full-binding/` de la tranche ci-dessus ; les preuves dev72 initiales sont conservées séparément. Le candidat complet est préparé, pas encore exécuté via Root.

S2/garde physique : `pin_candidate` conserve les vrais descripteurs copie/.git/workspace après vérification ; lane Git libérée avant retour. Préparation/lecture historiques passent par la même garde et ferment explicitement. Propriété hors wrappers génériques pour conserver les causes de fermeture après erreur d’ancêtre. Les tests couvrent commande réelle, verrou disponible pendant la garde, substitution/refus/restauration et échec de duplication EMFILE dans un sous-processus isolé. Aucun job géré ni validateur final activé. Preuves et statut exact : `evidence/20261025-scaling-s2-oi9dgo_e/candidate-witness-djwzsrgp/`.

S2/journal cumulatif : lecteur privé v3 avec candidat complet, plan de toutes les tâches et double préimage. Mapping par transport, résultats publics de la seule tâche courante, mais échec antérieur bloquant. Reçus v1/v2 inchangés ; aucun producteur géré activé. Contre-exemples conservés : hash candidat opaque admis en v2 et arguments vides concordants classés positifs. Origine SQL empêche une cohorte partielle réencodée de remplacer l’original ; le parcours de source ordinaire refuse tout candidat implicite. Contribution GLM intégrée avec API de hash réelle et cohérence original/normalisé renforcée. Preuves/recette/livraison : `evidence/20261025-scaling-s2-oi9dgo_e/native-cohort-71bzc4jf/`. S2/permissions/validateur final restent ouverts.

### S2 — raccordement Root du candidat, préparation explicite

Incrément suivant dev75 : `task validation-candidate` prend une observation de la vraie soumission native et des politiques cumulatives, libère les verrous Root/réglages avant Git, conserve et épingle le checkpoint candidat puis recontrôle l’état Root avant résultat. Les aperçus restent sans écriture et le lancement/reviewer géré reste refusé. Recette : `native-managed-candidate` ; rapport et preuves : `evidence/20261025-scaling-s2-oi9dgo_e/native-admission-wj7wdoab/`. Préparation effective ne vaut ni job ni exécution des contrôles : S2 global non clos, S3/S4 et RETEX final ouverts.

Résultat vérifié de cet incrément : préparation native installée en **v0.1.1-dev.77**, recette maintenue PASS/intégrité VERIFIED (26 tests Rust ciblés, contrôles ordinaires et reprise, vrais workers gérés puis 8 assertions de préparation/refus/récupération dans `app`). Aperçu ordinaire dev75/dev77 identique, empreinte source comprise. Vérification de l’exécutable réel, ressources protégées et 14 assertions du dialogue FR/EN ; Works conservés. [Rapport, limites et RETEX local](evidence/20261025-scaling-s2-oi9dgo_e/native-admission-wj7wdoab/RESULT.md). Aucun job cumulatif ou validateur final géré lancé.


### S2 — exécution native des contrôles sur le candidat (dev78)

Le vrai candidat préparé alimente désormais le claim Root : tous les créneaux,
origine SQL liée au plan complet/candidat et instance réelle Root/SQLite, bail
éventuel, permissions et commandes dans le sous-projet détenu. Les deux propriétaires
conservent séparément leurs descripteurs ; Git reste hors des verrous Root.
Producteur v3, publication des résultats, réconciliation et rejeu sans candidat
sont raccordés. Le validateur final géré reste explicitement refusé ; aucun reçu
ni test n'accepte la tâche ou ne publie le pointeur candidat.

Preuves : [rapport et RETEX de l'incrément](evidence/20261025-scaling-s2-oi9dgo_e/native-launch-3w5nsudd/RESULT.md),
`recipe-complete-inputs/` PASS / intégrité VERIFIED : 26 tests Rust ciblés,
compatibilité ordinaire et vrais workers gérés puis trois contrôles par job,
trois jobs enregistrés/un abandonné, bail automatique réel, commande2 en échec
suivie de commande3, refus de publication puis réconciliation sans réexécution,
rejeu terminal avec candidat/cadre d'admission déplacés. La régression de
préparation conserve ses huit assertions PASS. Binaire normal sans fixture,
empreinte de build identique à `/proc/PID/exe`, version **v0.1.1-dev.78**,
14 assertions du dialogue FR/EN et ressources protégées PASS ; Works inchangés
depuis le témoin dev77. L'activation est observée ; aucun second redémarrage
n'a été ajouté par cette reprise. Les autres développements sales ne sont pas
qualifiés par cette recette.

Revue GLM en contexte séparé : aucun défaut d'autorité démontré dans le premier
paquet ; complément reçu : lecteurs v3, abandon et sources brutes examinés sans défaut
démontré dans leur portée.
Consulter le rapport pour l'avis effectivement reçu, ses hashes et ses limites.
Ce n'est ni l'acceptation métier indépendante, ni la qualification globale Rust.
La preuve native porte sur la tâche courante ; une cohorte avec antécédent accepté
reste une preuve de composant, sans faux état accepté injecté.

**Incident de reprise B1 ouvert, propriétaire : contrôleur natif/superviseur racine.**
Une mort avant le reçu global, ou une écriture de créneau en échec, peut laisser
un job actif sans original global. Rejeu, réconciliation et abandon refusent alors ;
l'absence du propriétaire n'établit pas le résultat/wait d'un enfant. Cette limite
existe dans le lot ordinaire aussi. Options : fabriquer un reçu/débloquer par
absence du PID (rejeté : preuve insuffisante) ; conserver l'exclusivité et porter
une récupération explicite fondée sur les originaux/créneaux réellement connus
(retenu). Condition de reprise : distinguer lot complet privé mais reçu global
absent, créneaux jamais admis et issue inconnue ; reproduire chacun après témoin
valide et prouver le règlement sans réexécution/acceptation indue. Ni le nouveau
chemin géré ni l'avis GLM ne corrigent cette limite. S2 global, S3/S4 et RETEX
final du projet restent ouverts.


Complément de revue : GLM suspectait des nombres négatifs admis au calcul des
scores. Hypothèse réfutée sur le vrai dev78, puis par le corps de
`gate_json::Document::number` (fini et >=0). Recette maintenue `gate-nonnegative` :
16 assertions des méthodes2/3, témoin valide/échec légitime/négatifs/zéro/reprise,
réglages et preuve inchangés. Aucun correctif produit ajouté. Apprentissage :
transmettre les accesseurs/décodeurs effectifs, pas seulement leur appelant, avant
de conclure à une validation absente ; un avis reçu n'est pas une preuve de défaut.


### B1 — reprise des lots complets sans reçu global

Reprise APEX du26octobre : le défaut est reproduit sur le vrai dev80 après un
témoin valide, avec trois reçus de commande attendus mais aucun reçu global.
Le candidat récupère cette même requête sans commande supplémentaire ; refus
source modifiée/créneau manquant/wait inconnu, restauration et abandon vérifiés
sur cette fixture. Recette finale PASS / intégrité VERIFIED, revue GLM sans défaut démontré dans
sa portée et installation dev82 vérifiée : voir le
[rapport de l'incrément](evidence/20261025-scaling-s2-oi9dgo_e/native-partial-_vzckkq_/RESULT.md).
La reconstruction exige origine SQL, source actuelle identique et tous les
originaux connus. Elle ne règle pas les lots aux observations absentes/inconnues.
Le reçu de reconstruction conserve la provenance et l'heure d'assemblage ; aucun
exit/wait n'est reconstruit. Le validateur final géré reste une dépendance S2 ouverte.


### Inspection complète avant le validateur final — dev84

Le parcours public `task review-preview` / `task review-inputs` inspecte maintenant
le candidat Git retenu et la cohorte des contrôles réellement réussis : sources
suivies complètes, rapports canoniques, critères, empreintes et cibles littérales
des liens. Les fichiers non UTF-8 sont refusés ; aucune troncature ni lecture des
cibles des liens. Les quatre limites persistées du reviewer sont appliquées avec
refus et reprise vérifiés. Aucune nouvelle politique opérateur codée en dur.

Le contre-exemple dev82 est conservé. La recette maintenue passe ses quatre
parcours publics (ordinaire, reviewer ordinaire de fixture, candidat géré texte,
refus binaire), avec douze assertions texte et quatre assertions binaire. Le
parcours sans contrôles refuse aussi. Revue GLM séparée, Rustdoc, diagrammes
Mermaid FR/EN et installation du vrai binaire dev84 sont vérifiés dans le
[rapport d’incrément](evidence/20261025-scaling-s2-oi9dgo_e/native-review-context-ekf9ibrv/RESULT.md).

Ce contexte est explicitement `inspection_only` : aucun appel au reviewer géré,
aucun verdict final, aucune preuve de validité des verdicts des antécédents
acceptés, aucune acceptation ni publication du pointeur candidat. Le raccordement
suivant appartient au contrôleur/reviewer natif : lier ces preuves au cycle réel
réservation, permission, appel, réponse, reprise et application autorisée. S2
global, S3/S4 et RETEX final restent ouverts ; le RETEX de cet incrément ne les
clôt pas.


### Protocole complet de citations — sources et schéma du validateur

`task review-inputs` prépare maintenant le contexte typé et son empreinte, le
catalogue complet des originaux citables et son empreinte, ainsi que le schéma
de réponse v2 de toute la cohorte. Limites de citations persistées et parseur
partagent les mêmes producteurs ; sans référence admissible, fail/unknown restent
possibles sans preuve pass inventée. Les sources ordinaires gardent leur JSON.

Revue GLM : promotion de métadonnées moteur en sources reproduite après témoin
valide, puis corrigée sur le rôle des matériaux. Leurs octets restent dans les
matériaux/reçus, sans devenir des preuves d’implémentation. Sept tests du protocole
et un test de cette frontière passent, dont le vrai aller-retour JSON avec reçu
opaque. Le schéma impose une liste vide explicite aux tâches sans critère ; le
parseur accepte aussi son absence. Le minimum normalisé ne vise que le repli
Markdown, pas les citations littérales exactes.

Recette publique, profil livré, installation et limites : [rapport](evidence/20261025-scaling-s2-oi9dgo_e/native-review-protocol-2vxq_rki/RESULT.md).
Le premier refus sous plafond131072 et les défauts de recette restent conservés.
Pour les fixtures, le quota de prompt est explicitement mis à0 par l’API publique ;
aucun défaut produit ni plafond opérateur ne disparaît par une constante locale.
Le protocole demeure inspection_only : invocation gérée, réponse finale applicable,
preuve des verdicts antérieurs, acceptation/publication et S2 global restent ouverts.
Prochain propriétaire : contrôleur/reviewer natif ; raccorder ce protocole à toute
la chaîne de réservation/permission/appel/réponse/reprise sans retirer les gardes
sur la seule base d’un catalogue lisible.

Livraison locale vérifiée : dev91, huit tests Rust ciblés, quatre parcours de
recette/intégrité vérifiée et12 assertions d’inspection+4 de protocole revérifiées
sur le binaire installé. Quatorze contrôles Version FR/EN, Works logiques et formes
numériques conservés. Dev90->dev91 ne change que notes de version/métadonnées ;
les preuves ordinaires inchangées sont explicitement réutilisées. Apprentissage
pour le prochain contexte : fournir dès le premier paquet review_quote.rs
(citation exacte/repli Markdown) et model_wire.rs (GoRaw/sérialisation canonique),
avec producteur et vrai parseur ; leur absence avait créé un constat conditionnel
superflu. Le refus partagé sur diff commençant par une ligne vide reste dormant,
propriétaire du futur support diff ; le contexte natif courant n’invente aucun diff.


### Contrôle public de sortie fournie — dépendance du lancement natif

`task review-check` reconstruit le protocole sous témoins du candidat, exige
l’empreinte du prompt original et vérifie le JSONL fourni par le vrai transport
sans outils puis le parseur de cohorte complète. Réglages Admin persistés, noms
canoniques uniques, indices et citations sont appliqués. Résultat : données
d’opinion uniquement ; aucun fournisseur lancé, débit, verdict enregistré ni
acceptation. Le lancement natif géré reste protégé.

Cinq tests unitaires ciblés passent, dont deux tâches, erreur de transport et
conservation des originaux refusés. Le premier parcours CLI réel passe neuf
assertions de sortie, en plus des douze assertions d’inspection et quatre du
protocole ; la recette consolidée et l’installation sont suivies dans le
[rapport d’incrément](evidence/20261025-scaling-s2-oi9dgo_e/native-review-check-vtinqwfv/RESULT.md).
L’appel de proposition GLM initial a échoué HTTP504 après600.063s sans code ;
l’implémentation a été reprise directement et une revue distincte, plus courte,
est demandée à GLM. Ne pas compter cet échec comme une contribution intégrée
ni comme une consommation nulle. S2 global et le RETEX final restent ouverts.

Clôture locale de ce parcours : dev94 installée, trois parcours consolidés PASS
et intégrité vérifiée ; cinq assertions CLI sur le vrai exécutable actif et
quatorze contrôles Version FR/EN passent. La revue GLM réduite répond après496.183s :
aucun défaut direct dans les deux nouveaux modules, avec des limites explicites
sur les corps de fonctions inchangés non transmis. Les preuves locales associées
sont reliées dans le rapport ; cet avis n’est pas une acceptation indépendante
globale. Les deux appels larges restent des HTTP504 conservés, consommation inconnue.
Des notes et deux lignes d’aide des préparations ajoutées en parallèle après
livraison ne sont pas déclarées installées par ce lot. Le code du parcours
review-check livré et testé est inchangé.


### Reviewer natif géré — appel, journal et reprise de cohorte

`task review` raccorde désormais le candidat retenu hors verrous à la réservation,
au contrôle de l’enfant, au transport sans outils et à la publication de toute
la cohorte. Le journal conserve protocole, limites et réponse d’origine ; contexte
public adressé par contenu, avis et événement sont liés aux octets contrôlés.
Un avis n’accepte aucune tâche. La lecture historique gérée est raccordée ;
le consommateur d’autorité d’acceptation/reprise de production et la preuve de
baseline acceptée restent explicitement protégés, donc S2 global demeure ouvert.

Contre-exemple réel : après un appel terminé, la publication comparait le
protocole typé à une projection JSON dont l’ordre des clés différait. Correction
générale par comparaison de deux modèles typés canoniques, sans retirer les
contrôles de contexte, limites et prompt. Le même original a ensuite été appliqué
par `review-reconcile`, sans nouvel appel ni débit. Les sources running utilisent
le record interne exact, jamais une permission fournie par JSON.

Six parcours sont vérifiés : reviewer ordinaire, nouvel appel géré, cohorte
partielle refusée, publication refusée puis reprise, source modifiée pendant
l’appel, et compatibilité review-check. Trois résultats fonctionnels conservés
sont réutilisés, trois parcours complémentaires passent. Le premier wrapper de
recette reste FAIL : son seuil arbitraire de cinq assertions rejetait un cas
complet à quatre assertions ; la recette maintenue vérifie les exigences nommées.
Deux tests du journal, neuf de capture et la régression canonique passent.
GLM a contribué au design de l’emprunt et revu les frontières fournies : aucun
défaut direct confirmé, limites des extraits conservées. Rapport, état réel de
l’installation et RETEX : [preuves](evidence/20261025-scaling-s2-oi9dgo_e/native-review-launch-xxnx8icj/RESULT.md).

Apprentissages transmis : cibler le constructeur exact lors d’un changement de
`cfg(test)`, toujours qualifier le profil produit ; comparer des modèles de même
représentation avant d’inférer une dérive ; préserver et reprendre le même original
après refus de publication ; vérifier des obligations nommées par cas. Aucun
commit/push ni redémarrage de la mission suspendue n’est implicite.

Livraison de cet incrément vérifiée : **dev.101** active par le lanceur pris en charge ; empreinte réelle conforme au profil normal sans fixture, ressources protégées conformes, 14 assertions UI FR/EN et travaux conservés. Un nouveau parcours natif complet sur ce même binaire passe cinq assertions nommées, dont refus/restauration du contexte et rejeu sans appel/débit supplémentaire. Prestataire inerte isolé ; aucune acceptation de tâche ni clôture S2 globale. Détails et RETEX dans le rapport lié ci-dessus.

### S2 — entrée publique de prépublication vérifiée, dev106

`task publication-preview` vérifie l’original de l’avis natif, les originaux des contrôles et la cohorte du candidat actuellement retenu, sans écrire ni relancer de fournisseur. Les blocages de publication atomique et de baseline acceptée restent explicites ; une observation vérifiée n’est jamais `publication_ready`. Le contrat Go d’intégration confirme qu’accepter seulement un statut laisserait le pointeur candidat précédent consommé par les prochaines copies : la proposition GLM limitée au statut est écartée. Contrat d’acceptation encore ouvert, pas de suppression de garde. Sources, refus original dev101, recette maintenue, revue et livraison suivis dans `evidence/20261025-scaling-s2-oi9dgo_e/native-review-accept-e1j5v2ic/`.

Livraison constatée en dev106 : original natif favorable et avis demandant des corrections observés sur le binaire réellement servi ; aucun appel, débit ou changement de Work. Douze cas du parcours favorable et quatre assertions complémentaires passent ; la recette initiale et le complément sont conservés et leur intégrité vérifiée séparément. Le refus réel sur tâche bloquée a été reproduit puis corrigé pour la seule lecture `RecordedOpinion`. Quatorze assertions UI FR/EN, ressources protégées, profil normal sans fixture et conservation exacte des travaux depuis dev101 vérifiés. Revue GLM ciblée sans défaut confirmé dans les frontières fournies ; aucune acceptation ni conformité S2 globale. [Rapport/RETEX](evidence/20261025-scaling-s2-oi9dgo_e/native-review-accept-e1j5v2ic/RESULT.md).

Instance courante recontrôlée après une autre livraison locale : **dev107**, mêmes octets Rust de cet incrément, profil qualifié, deux branches publiques et14 assertions UI FR/EN de nouveau vérifiées. La preuve dev106 reste historique et intacte ; voir `activation2/` dans le rapport.
