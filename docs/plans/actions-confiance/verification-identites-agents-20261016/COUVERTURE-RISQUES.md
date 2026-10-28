# Identité — contrôle de couverture après IA1

16 octobre 2026. Question utilisateur : « tu as rien oublié de l'identité ? tous
les scénarios de risques précédents sont traités ? » Méthodes : `replan`, puis
`spec-audit`. Audit de couverture documentaire, sans nouveau test d'application,
modification du code ou changement des résultats historiques.

**Verdict : les demandes initiales sont tracées ; les risques ne sont pas tous
traités et la spécification doit encore être précisée avant IA3.** Les dix-huit
scénarios ne sont pas une démonstration d'exhaustivité. Il faut distinguer la
présence au plan, la reproduction d'un défaut, sa correction et sa qualification.

## Demandes utilisateur et état réel

Le passage de relais et les quatre échanges précédents du chat « Comparer Deep
Agents à Swarm » ont été relus. Les demandes humaines retrouvées concernent le
propriétaire, l'auteur, les droits d'usage, l'impossibilité pour un pirate de
présenter un agent comme légitime et pour un agent légitime de créer un enfant
non autorisé, avec explication et décision humaine pour une exception hors plan.
La demande de scénarios complémentaires est également conservée dans ce chat.

| Demande initiale | Contrat et scénarios existants | État après IA1 |
| --- | --- | --- |
| Empêcher l'usurpation d'un agent | Invariant 1, IDA-01/02/14/15 | Défauts F01/F02/F04/F07 ; aucune correction applicative. |
| Empêcher un enfant non autorisé, même demandé par un agent légitime | Invariants 2/3, IDA-03/05/06/09/12 | F06 reproduit une création par un processus déjà admis ; identité authentifiée et délégation restant à construire. |
| Expliquer l'exception hors plan et obtenir une décision humaine | Invariants 4/5/6, IDA-03/07/08/16 | Contrat écrit ; parcours de décision humaine non implémenté ni qualifié par IA1. |
| Distinguer propriétaire du cadrage et auteur | M20 et invariants 1/9 | Plan mémoire existant, attribution métier effective non livrée par IA1. |
| Contrôler consultation, résumé et transmission | Invariant 9, IDA-13, IA4 | Dépendance IA2/IA3 ; aucun contrôle d'usage nouveau livré. |

La matrice du `RAPPORT-IA1.md` contient exactement **4 FAIL, 4 PARTIAL et
10 NOT TESTED**, sur 18 scénarios. Aucun n'est entièrement PASS. Les sept
contre-exemples sont des observations regroupées dans ces scénarios ; ils ne
représentent ni sept scénarios entièrement couverts ni sept corrections.
Les treize noms de tests Go verts concernent des contrôles existants.

## Risques historiques R01/R13 à réintégrer au consommateur agent

Sources relues : `identites-entreprise.md`, `identites-r13-coverage.md`,
`identity-context-replan.md`, `group-mapping-replan.md`, le plan d'identité des
agents et le rapport IA1. Les mentions PASS ci-dessous sont celles des rapports
historiques dans leur périmètre ; elles n'ont pas été requalifiées par cette
lecture ni transmises automatiquement à l'admission des agents.

| Risque déjà recensé | Couverture historique déclarée | Preuve encore nécessaire pour les agents |
| --- | --- | --- |
| Mauvais issuer, signature, audience, nonce, PKCE ou dates | Parcours humain OIDC sur émetteur isolé | Refus au consommateur de création et d'approbation ; aucune identité déduite d'un champ client. |
| Confusion humain/service/local ou assertion client/access token | Contexte commun, workload et mapping | Un principal de service ou local ne devient pas une personne approbatrice ; types de jeton non interchangeables. |
| Groupes signés, cumul de groupes et extension au-delà des rôles locaux | Mapping restrictif versionné | Les droits d'agent et de création respectent aussi ce plafond et sa révision ; aucun rôle déclaré ne crée un droit. |
| Expiration, suspension, logout et réactivation | Cycle de vie et contexte commun | F03 montre le raccord absent au lancement ; réactivation sans restauration d'anciennes autorisations à vérifier. |
| Rotation de clés | Recettes humaines et services déclarées | Effet au point d'usage agent ; clé remplacée légitimement et clé révoquée/compromise traitées distinctement. |
| Révocation humaine distante et connexion en vol | Logout signé déclaré | Refus frais au lancement et à l'approbation ; pas de session devenue valide après révocation en vol. |
| Révocation distante d'un access token de service | Introspection sur émetteur isolé déclarée | Même refus au consommateur agent, y compris indisponibilité/malformation et reprise du serveur. Aucune révocation intersystèmes instantanée supposée. |
| Délégation, séparation et authentification renforcée exacte | Mandats et step-up déclarés | Les accords de création doivent contrôler les rôles effectifs, la personne habilitée et le contenu exact ; les anciens accords métier ne valent pas accord de création. |
| Expiration pendant une attente de verrou | Défaut et correction du temps sous verrou documentés | Revalider sous la frontière d'engagement, pas seulement avant l'attente, puis au départ du processus. |
| Secrets et preuves | Contexte sans jeton brut, ancrage et périmètres propres aux helpers | F07 : protéger le registre et les clés contre le fournisseur général ; les garanties d'un helper ne couvrent pas tous les processus agents. |

## Six précisions nécessaires, sans masquer les exigences existantes

Ces points sont des constats de spécification ou des variantes de test à
ajouter au travail IA2/IA3/IA5. Ils ne sont pas six nouvelles vulnérabilités
runtime démontrées. Propriétaire : Codex. Aucun nouvel endpoint, protocole,
plafond ou droit n'est créé par ce document.

| ID / sévérité | Lacune ou précision à apporter | Critère discriminant proposé / rattachement |
| --- | --- | --- |
| COV-ID-01 — major | IDA-01 décrit surtout les jetons invalides ; IDA-02 la copie d'identifiants. Le vol d'un **jeton valide** ou l'usage d'une autorisation par un autre processus n'a pas de cas explicite. | Copier le moyen d'authentification ou l'autorisation d'une instance vers une autre tâche/processus/mission ne doit pas suffire à usurper cette instance. Définir l'ancrage réellement vérifié et son modèle de menace ; tester aussi l'usage d'un service privilégié comme intermédiaire pour une demande interdite. IA2/IA3, variantes IDA-01/02/05/09/14. |
| COV-ID-02 — major | Le plan renvoie au contexte Trust mais ne déroule pas son héritage de fédération, type de jeton, mapping, rotation, révocation distante et temps sous verrou jusqu'à l'admission. | Ajouter les refus du tableau précédent aux vrais consommateurs, avec absence de lancement et de réservation de travail sur refus. Les PASS des producteurs restent distincts des preuves de raccord. IA2/IA3/IA5, variantes IDA-01/07/09/10/12/16. |
| COV-ID-03 — major | L'identité logique/instance est requise par l'invariant 1 ; ses collisions de domaine et son remplacement ne sont pas déclinés en recettes. | Même nom ou sujet externe chez deux émetteurs, homonymes entre projets, clone/reconnexion d'une instance et remplacement d'un agent ne doivent pas transférer de droits. L'ancienne instance reste historique et ne récupère pas une autorisation du remplaçant. IA2/IA5, variantes IDA-02/09/15. |
| COV-ID-04 — blocker pour le contrat de révocation | L'invariant 7 mentionne déjà les agents actifs et descendants, mais dit seulement de définir leur traitement. Le résultat attendu n'est pas encore arrêté. | Définir ce qui doit être bloqué, arrêté ou réconcilié pour un agent actif, ses descendants, outils et effets déjà engagés. Vérifier l'arrêt/refus réel et conserver un effet incertain comme tel ; un statut affiché ne prouve rien. IA2/IA3, variantes IDA-10/11/16 ; aucune politique de délai cachée. |
| COV-ID-05 — major | IA1 a inventorié les responsables, revues, tours de préparation, précontrôles et métadonnées ; IDA-12 ne les nomme pas tous explicitement. | Exercer l'admission avant tout exécutable fournisseur concerné, y compris avant une sonde ou des métadonnées lorsque la demande est interdite. Une activation sans ligne `Agent` ne doit pas contourner l'autorité. IA3/IA5, variantes IDA-03/06/12/14/17. |
| COV-ID-06 — major | Bootstrap et intégrité du journal existent dans les invariants 5/10 et IDA-18 ; la récupération après compromission et la restauration d'un ancien état authentique restent implicites. | Définir l'autorité de récupération, traiter une clé compromise et une ancienne sauvegarde/ancre sans réactiver droits ou accords révoqués. Détecter l'état périmé selon l'ancre et la menace retenues ; ne pas promettre de résister à un administrateur hôte tout-puissant sans frontière extérieure démontrée. IA2/IA3/IA5, variantes IDA-09/10/15/18. |

Une autre formulation est devenue périmée : la matrice du plan présente encore
chaque ligne comme NON TESTÉE alors que le rapport IA1 conserve quatre défauts
et quatre couvertures partielles. La matrice reste un tableau des résultats
**attendus** ; son état observé fait foi dans le rapport IA1. Cette divergence
documentaire ne doit ni effacer les échecs ni devenir un PASS implicite.

Après cet audit en lecture seule, la reprise `replan` a corrigé cette formulation
dans le plan et y a rattaché COV-ID-01–06 aux scénarios existants. C'est une mise
en cohérence documentaire, sans résolution des décisions ouvertes, sans
nouvelle autorisation et sans correction applicative.

## Décision et condition de fermeture

Comparer deux voies : conserver les dix-huit titres génériques et supposer que
les tests Trust couvrent leur intégration, ou conserver leurs identifiants et
décliner explicitement les variantes précédentes au consommateur. La seconde
est nécessaire : F02/F03 démontrent précisément qu'un producteur correct peut
ne pas gouverner le lancement.

IA1 est un audit initial livré, pas la fin de la sécurité des identités. Avant
de présenter IA2/IA3 comme prêt, résoudre COV-ID-04, définir les liaisons et les
critères COV-ID-01/02/03/05/06, puis réaliser et vérifier les corrections dans
les droits convenus. Préserver IDA-01–18 et les originaux F01–F07 ; les variantes
sont des compléments, sans remise à zéro de l'historique.

La fermeture exige les résultats effectifs de création/exception, les refus
sans effet, la réception des droits, leur révocation, le confinement annoncé,
les autres consommateurs, les droits mémoire, la persistance/reprise, les
preuves d'intégrité et le RETEX. Une qualification peut être partielle seulement
si son périmètre et toutes les limites restantes sont explicites. Aucune
garantie d'absence absolue d'autres risques ne découle de cet inventaire.
