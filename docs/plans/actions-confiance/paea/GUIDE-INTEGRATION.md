# PAEA — intégration du guide opérationnel

[English](../../../en/plans/actions-confiance/paea/GUIDE-INTEGRATION.md) ·
[Plan adopté](../PAEA-APEX.md) · [Guide conservé](../../../security/PAEA_v0.2_Guide_Operationnel.docx)

## Statut et décision

L’utilisateur fournit ce guide pour comprendre PAEA et guider son implémentation.
Il devient une **référence opérationnelle de travail**, confrontée aux sources,
aux preuves et aux décisions existantes. Ses prescriptions sont des exigences
proposées du document : elles ne donnent aucun droit d’exécution, de délégation,
de paiement ou de déploiement supplémentaire. Le DOCX original est conservé à
l’identique, hash `e621efa0953ff056cf12b318236849764feeff16f9e2d2f3c5b4016d75774298`.
Le plan PA-00…13 et les deux pilotes règlement fictif/promotion Git restent en place.
IAM et industrie demeurent des exemples de généralité, hors exécution autorisée.

Lecture : 288 blocs textuels/tables, les deux figures, sans redlines ni notes
annexes ; rendu de 34 pages produit, pages pertinentes examinées. Le guide
présente une architecture de recherche et ses recettes, pas un déploiement déjà
réalisé. Les sources externes de son annexe E ne sont pas réévaluées ici.

Le premier écran **Vérifications PAEA** reste un contrôle de correspondance de
fichiers. Il ne couvre pas le processus opérationnel décrit : criticité, autorités,
accord humain, engagement protégé, oracle indépendant et décision de déploiement.
La suite doit avancer vers un pilote complet, sans accumuler des tests de forme
comme s’ils démontraient une protection du service d’effet.

## Correspondance et état réel

| Guide | Lots et propriétaire | État observé et sortie requise |
|---|---|---|
| §§1–3, annexe A : effet/criticité/utilité | PA-00/01, responsable métier/risque + Codex | Besoin général retenu ; fiche par effet à renseigner avec auteur habilité, conséquences, oracle et décision. Aucune classification C3 déduite du montant. |
| §4 : menace/TCB | PA-00/02/04/09, identité et exploitation | Architecture cible connue ; comptes, privilèges, secrets, FD, sockets, chemins et versions effectifs à inventorier pour le pilote. Les noms de composants ne prouvent pas leur isolation. |
| §5 : trois gardiens/point de non-retour | PA-02/03, autorité et services d’effet | Interfaces protégées non démontrées. Contrôle courant au point où le sink peut encore refuser ; aucun ALLOW ni reçu cohérent n’est un effet. |
| §6 : provenance/mémoire/confidentialité | PA-05/06, mémoire/identité | Forme de provenance présente dans le contrat ; attribution et conservation à lecture, résumé, compaction, retrait et transmission à démontrer. |
| §7 : identité/délégation/clés | PA-02/06/07, identité + contrôle protégé | Dépendances IA3 maintenues. Résoudre principal/instance/mandat côté contrôle, protéger les clés, atténuer les droits et vérifier l’audience ; voir ERR-01 ci-dessous. |
| §8 : intention/accord exact | PA-01/03/08, contrat et services métier | Canonicalisation et rapports testés localement sur dev77 ; contexte encore fourni comme données. Objet d’approbation représenté mais non authentifié. Résolution fournisseur/facture/compte et capture d’interface restent ouvertes. |
| §9 : politique/révocation/ordre | PA-02/03/07, contrôle + sink | Contrat spécifié ; preuve au sink à produire. Réglages opérateur persistés et exposés via Admin/API/CLI, aucune constante issue d’un exemple. |
| §10 : médiation/egress | PA-04/05, AC-02 | Qualification Rust/AC-02 préalable aux protections critiques. Témoin puis contournements dans les droits réels des agents/descendants, chaque chemin déclaré. eBPF d’observation n’est pas une barrière. |
| §11 : unicité/UNKNOWN | PA-03/09, sink + vérification | Cycle de données testé ; aucun registre PAEA protégé ou rapprochement d’effet démontré. Conserver tentative, intention, reçu et consommation ; pas de nouvelle clé pour sortir d’UNKNOWN. |
| §12 : F-12 de bout en bout | PA-00…10, Codex intégration | Adapter les étapes A…L au pilote fictif puis à Git ; ne pas inscrire « démontré » sur la base des scénarios racontés. |
| §13 : IAM/sûreté industrielle | Hors périmètre actuel, commanditaire | Conserver les enseignements. Aucun connecteur IAM/industriel réel, règle de sûreté ou mandat nouveau adopté par ce guide. |
| §14 et annexe B : 17 invariants | Tous lots, propriétaires du plan | Registre existant conservé ; **0/17 démontré de bout en bout**. Chaque propriété doit relier responsable, frontière, oracle, contre-exemple et preuve. |
| §15 et annexe C : recettes falsifiables | PA-03/09/10/12, vérification | Les 18 tests CLI/973 appels et 174 assertions par pont restent des preuves locales distinctes. Construire les trois recettes initiales sur l’état indépendant du sink, avec témoin avant chaque faute. |
| §16 : B2/ablations/statistiques | PA-11/12, évaluateur + commanditaire | Protocole E3 existant préparé, pas préenregistré. Comparateur B2 réaliste, conditions comparables, indépendance des essais, faux refus, utilité, coûts, latences et inconnus à mesurer. Aucun cumul d’assertions n’est un nombre d’essais indépendants. |
| §17 : incidents/requalification/humain | PA-08/09/10/13, exploitation + superviseurs | Procédures à relier aux opérations publiques réelles et à l’effet observé. Revue IA distincte d’un accord humain ; retrait de mémoire et suppression d’un secret n’effacent pas leurs copies. |
| §18 : dossiers/statuts | PA-00/09/10/11, responsable du dossier | Séparer conception, réalisation identifiée et qualification ; conserver NON EXAMINÉ, SPÉCIFIÉ, IMPLÉMENTÉ, TESTÉ LOCALEMENT, ÉVALUÉ INDÉPENDAMMENT, RÉFUTÉ et HORS PÉRIMÈTRE. Aucun E2/E3 déclaré ici. |
| §§19–20 : jalons/ateliers | Plan complet, Codex + responsables habilités | J1 → PA-00/01/02/03/09 ; J2 → PA-04/05 ; J3 → PA-06/07 ; J4 → PA-11/12 ; J5 → PA-11/12/13. Les cinq ateliers sont une méthode de travail, sans calendrier ni surveillance créée. |

## Clarifications avant consommation technique

**PAEA-ERR-01, provenance étendue au §7.1 du guide (page rendue 10).** Une capacité
à usage unique liée au contenu ne remplace pas le contrôle de révocation courante.
Contre-exemple : jeton émis, révocation enregistrée, première consommation encore
inutilisée ; un sink vérifiant seulement contenu et unicité peut l’accepter.
Le contrat retenu reste celui de PA-02/03/12 et du §9.2 : vérifier l’epoch/état
courant dans l’ordre d’engagement reconnu par le sink, ou déclarer la propagation
et refuser le périmètre exigeant une immédiateté non garantie. T05/T17 doit tester
spécifiquement la **première** consommation après révocation. Propriétaire :
contrôle d’effet ; état : exigence clarifiée, preuve distribuée toujours ouverte.
L’original du guide et les preuves historiques ne sont pas réécrits.

**PAEA-GUIDE-02, exemples de représentation.** Le §8 qualifie son JSON de
pédagogique, non stabilisé ; il ne remplace pas `contract-v1.schema.json`.
La deuxième figure contient notamment `montant: 4200.00`, tandis que le texte
et notre profil utilisent `amount_minor: "420000"`. Ne pas copier la figure
comme payload, politique, logo produit ou schéma de conformité. Le contrat actuel
refuse le montant numérique, ne revendique pas JCS et conserve ses vecteurs v1.
Propriétaire : contrat/documentation ; état : règle de consommation documentée,
aucun changement de schéma ou test de sécurité déduit de l’illustration.

**PAEA-GUIDE-03, identité de la dette.** Le guide illustre
`payment:S01:F-12` ; le profil actuel dérive l’intention de `project_id+invoice_id`.
Cela est compatible seulement si le producteur protégé résout `invoice_id` vers
un objet de dette unique dans le projet, et non une référence de facture brute
qui pourrait être réutilisée par deux fournisseurs. Avant PA-03, exiger ce contrat
du registre fournisseur/facture et tester collisions, alias et corrections.
Conserver v1 maintenant ; si le registre ne fournit pas cette identité, prévoir
un schéma versionné et une migration explicite, pas un ajout implicite au hash.
Propriétaire : service métier/PA-03 ; état : précondition sémantique ouverte,
aucune unicité métier protégée revendiquée par les hashes actuels.

## Première tranche opérationnelle et parcours UX cible

La prochaine tranche fonctionnelle regroupe une **fiche d’effet**, son propriétaire,
le modèle de menace, la carte d’accès, l’intention, la politique, le point
d’engagement et les trois recettes avec oracle. Elle prépare PA-02/03 ; elle ne
saute ni leur autorité protégée ni les prérequis Rust/AC-02. Le règlement reste
fictif ; Git est le deuxième domaine déjà adopté. Les personnes habilitées et
les services effectifs doivent être identifiés, jamais inventés par le modèle.

| Recette initiale | Témoin et injection | Observation requise au service d’effet |
|---|---|---|
| T04 substitution | Action approuvée valide puis changement pertinent après accord ; ordre JSON équivalent accepté selon contrat | Aucun effet sur le contenu changé ; ancien accord et effet antérieur conservés. |
| T06/T21 réponse perdue | Effet légitime engagé puis perte de réponse/crash ; nouvelle clé d’essai proposée | Une seule opération métier ; UNKNOWN puis rapprochement ; absence d’idempotence/statut stable refuse la garantie. |
| T05/T17 révocation concurrente | Capacité valide, pause avant engagement, révocation ordonnée puis première consommation ; ordre inverse aussi testé | Révocation avant engagement refuse ; engagement antérieur reste historique ; ordre du sink conservé, pas comparaison de deux horloges clientes. |

Pour l’UX Projets et l’UX Processus, séparables, le menu PAEA devra permettre de :

1. Ouvrir le dossier de l’action : conséquence, propriétaire, sources, criticité
   justifiée et périmètre de risque ; les valeurs manquantes restent visibles.
2. Examiner l’action exacte et ses changements, le mandat et la politique observés ;
   demander l’accord à l’identité habilitée lorsque la politique l’exige.
3. Suivre l’engagement réel et ses refus ; afficher UNKNOWN et la réconciliation
   sans action de réémission libre, ni état « payé » déduit d’un rapport cohérent.
4. Lire incidents, procédures de reprise et dossiers de conception/réalisation/
   qualification, avec versions, preuves, limites et décision attribuée.
5. Accéder aux campagnes de **laboratoire** autorisées et à leurs résultats côté
   oracle, distinctes de l’exécution d’une action métier et des tests développeur.

**Cible non livrée.** L’écran actuel conserve son périmètre « correspondance » ;
aucune route métier ou acceptation n’est inventée pour remplir ce parcours.
Les métriques et politiques futures restent configurables selon le contrat commun,
pas fixées par les exemples F-12, cinq minutes, 200 actions/heure ou une classe C3
illustrative. Le changement de titre/menu ne doit jamais suffire à prétendre
que ce parcours opérationnel est implanté.

## Première préparation exécutée

[Dossier des pilotes](PILOTE-OPERATIONNEL.md) et [incrément 14](INCREMENT-14.md) :
effets SQLite/Git isolés, trois familles de panne dans les deux domaines,
processus oracle séparé et contre-exemple des IDs bruts de dette. Sept tests
PASS et intégrité VERIFIED ; le garde Git affaibli est détecté par contre-épreuve.
Autorité simulée/même UID, aucun menu de campagne ni protection native livré.
Ces preuves préparent PA-02/03 ; **0/17 invariants de bout en bout** reste inchangé.

### Suite GUIDE-03 — incrément 16

Le résolveur pur et la CLI `protection resolve-payment` traitent désormais un
mapping explicite fournisseur/référence → objet de dette stable. Collisions et
versions périmées refusent ; deux fournisseurs peuvent partager une référence.
[Contrat](../../../architecture/PAEA-CONTRACT.md) et [incrément 16](INCREMENT-16.md).
Le producteur protégé, la continuité des IDs entre snapshots et leur fraîcheur
au point d’engagement restent ouverts ; GUIDE-03 n’est pas clos globalement.

### Parcours opérateur — incrément 17

Le menu Vérifications PAEA donne désormais accès à la résolution de référence
de facture par registre/demande, via le même résolveur API/CLI. Montant et
révision exacts, marqueurs non autorisants et export brut ; [preuve et limites](INCREMENT-17.md).
Cela n’active ni registre protégé ni campagnes de laboratoire dans l’UX.


### Essais guidés — incrément 20

Les trois vues proposent des paires de fichiers fictifs versionnés, à télécharger
puis sélectionner explicitement. L’aide précise le service/état du reçu et les
résultats attendus ; [preuves et limites](INCREMENT-20.md). Aucune campagne de
laboratoire, donnée de projet ou protection native n’est déclenchée par le lien.
