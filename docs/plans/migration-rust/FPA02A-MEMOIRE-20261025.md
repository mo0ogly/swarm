# FPA-02a — pièces mémoire historiques et reprise SQLite

Statut : parcours vérifié et installé localement en **dev65**, aucune acceptation
globale de migration. Les droits M20 restent futurs.
Demande : reprendre le portage Rust avec APEX et GLM/SKYNET. Intégration Codex.
Référence : [plan actif](PLAN.md), FPA-01/02 de l'audit du 24 octobre.

Le consommateur `Work.memory` conserve les références SQLite, les remet comme
tableau JSON dans le prompt et les attribue au manifeste de la tentative.
Le garde ne retire pas les refus de sélection/admission de mémoire versionnée.
Les profils du guide, résumés et restaurations de conversation restent ouverts.
Ce parcours ne prouve ni fraîcheur des octets des pièces, ni confinement OS,
ni droit M20 ; les liens symboliques existants sont contrôlés, pas les copies
libres ou la récupération globale des connaissances déjà reçues.

| Contrat | Preuve attendue | État vérifié |
| --- | --- | --- |
| Référence historique utilisable | aperçu/manifest, réception par worker factice | PASS, recette finale |
| Refus des sources gérées | retrait historique, alias, refus avant processus | PASS, recette finale |
| Inventaire entier | ligne incohérente refuse, réparation reprend | PASS, recette finale |
| Politique SQLite initiale | échec original 0/173, correction et 3 tests | vérifié localement |
| Résultats gérés SCALING-S1 | deux copies, soumissions/inbox sans acceptation | PASS dev65 sur Root et sous-projet |
| Retrait d'autorisation concurrent | synchronisation, aperçu et lecture du rapport | 21 tests de supervision et parcours S1 PASS |
| Installation | candidat normal, source-review, serveur/ressources/Version | PASS, dev65 et 45 ressources |

La recette permanente est `memory-files-rust`. Le témoin Go conserve les corps
historiques du choix des sources avec un lecteur de données factice ; il ne
prouve pas le SQL, les droits ou le lancement Go. Les projections des fixtures
sont calculées par le vrai probe Rust, séparé du binaire livré sans fixture.
Les triggers restent actifs. Une ligne négative a du JSON valide et une révision
d'enveloppe incohérente ; elle ne contourne pas le refus de JSON du trigger.

Le candidat commun contient SCALING-S1 déjà en travail. Sa recette a révélé une
assertion `blocked` utilisée après soumission, puis un arrêt réel du serveur sous
contention SQLite. Le bootstrap imposait zéro avant de lire le délai Admin.
La correction consomme les réglages validés avant la première requête d'identité,
sans nouveau délai, tentative, droit ou contournement de l'identité physique.

Preuves originales : `evidence/20261025-memory-files-zjficbf1/`. Échecs de recette,
réponses GLM et limites conservés. Une revue GLM avait manqué la contradiction
de statut ; la recette l'a reproduite puis une revue ciblée a examiné la correction.
Avis de modèle, tests locaux et acceptation indépendante du moteur restent distincts.
Le [reçu installé courant](evidence/20261025-memory-files-zjficbf1/installed-complete-summary.json)
identifie dev65, PID162928, empreinte
`11414fafac47e3ca4db8fe9146b4b027b787d35bc39b50c0167a92bd518c0a8f`.
La revue initiale porte 12 entrées effectives depuis dev60, puis la
[revue complémentaire](evidence/20261025-memory-files-zjficbf1/REVIEW-COMPLETE.md)
porte 3 entrées de dev62 à 65 : 14 entrées distinctes au total. Aucune baseline
intermédiaire ne masque les changements S1.

Les 7 tests moteur restent réutilisables : les 259 entrées moteur du profil n'ont
pas changé entre dev62 et65, preuve `engine-proof-reuse-complete.json`.
Les 21 tests de supervision et 32 assertions S1 (16 noms répétés sur deux Roots)
passent sur dev65 dans `shared-s1-complete`, intégrité VERIFIED. Le parcours
mémoire est rejoué sur dev65 : 44 assertions, 38 noms distincts. La recette mémoire
complète de dev62 en compte 45/39 : elle ajoute le contrôle de stabilité des entrées
du témoin Go par son lanceur, conservé avec le probe de projection et son intégrité.
Les 55 assertions consignes et 36 navigateur
sont la non-régression dev62, distincte des nouveaux tests dev65. Rustdoc CLI
dev65 passe avec avertissements interdits ; la preuve moteur inchangée est conservée.
La fenêtre Version installée vérifie 14 assertions FR/EN, notes, commit et
modifications non commitées, sans POST. Les 45 ressources HTTP correspondent aux
octets du profil et les deux missions réelles sont inchangées et suspendues.

Onze appels API GLM sont conservés, route et modèle déclaré `glm-5.2` : dix
réponses exploitables et une sortie épuisée sans texte final. Un assemblage de
contexte manquant a été refusé avant appel API ; il ne compte pas comme appel.
Les workers et planificateurs des recettes sont des programmes locaux factices.
Le coût GLM n'est pas établi ; aucune économie ni acceptation indépendante n'est
inventée. Les archives vérifiées des profils et parcours, dont les trois échecs
S1 originaux, sont inventoriées dans `archives/inventory.json` et
`archives/fixtures-inventory.json` sous les preuves.

## RETEX et suite conservée

Reprise après installation dev62 : la répétition avec l'inventaire de sources
complété révèle trois courses d'autorisation propres (preview, lecture du rapport,
synchronisation à nouvelle révision) traitées à tort comme fatales au serveur.
Les échecs `shared-s1-bound`, `shared-s1-pause`, `shared-s1-report` restent originaux.
Les candidats 63/64 n'ont pas été activés. Après ces échecs au même point public,
la portée de vérification est réduite à une matrice déterministe des trois formes
et des refus de nettoyage/écriture/incertitude ; ses 3 tests passent. Dev65 passe
ensuite les 21 tests de supervision et le parcours public complet, avant activation.
Les reçus dev62 restent historiques, sans effacer les courses découvertes ensuite.

Le correctif reconnaît uniquement les formes typées effectivement reproduites :
aperçu après file commise et clôture propre ; observation du rapport en lecture
seule ; conflit de révision synchronisée accompagné d'un refus d'autorisation
indépendant, pour le même Work et sans préparation. Un conflit seul ne suffit
pas. Le watch conserve les effets et causes, termine sa seule session avec
`permission-withdrawn` et draine ses rôles possédés. Il ne signale pas les workers
indépendants et ne rejoue ni admission ni décision. Nettoyage échoué, écriture
incertaine, mauvais Work ou publication de sortie échouée restent des erreurs.
Ce nouveau motif Rust ne constitue pas une parité de format Go démontrée.

Le besoin initial est préservé : restaurer les capacités Rust par parcours publics,
sans supprimer les gardes manquants. La mémoire permanente versionnée n'est pas
clôturée par ce lot de références historiques. Prochain propriétaire G8/Codex :
raccorder les sources RETEX et leurs preuves fraîches, puis catalogue/aperçu,
admission/remplacement/retrait, sélection et effet observé au consommateur.
Reprise : contrat Go pertinent retenu et consommateurs Rust présents, puis
témoin valide, refus/reprise et revue sur le même candidat. Aucun champ historique
ne doit être vidé pour obtenir un départ.

L'erreur réelle la plus importante était une politique cachée avant l'identité SQL.
Le témoin avant correction échoue0/173 ; le consommateur de la valeur persistée
est maintenant exercé, y compris refus zéro explicite et reprise. Une contention
plus longue que le délai configuré peut toujours refuser ; aucun retry nouveau
ni maintien de service garanti dans toutes les pannes n'est revendiqué.

Défauts de méthode conservés : enveloppe `work` mal lue, attente d'un événement
HTTP inexistant après refus pré-Root, empreinte comparée avant un changement de
révision des réglages, fixture SQL sans UDF, source Go non canonique dans le
témoin. Ils ont été diagnostiqués avant reprise ; leurs sorties restent originales.
La recette distingue maintenant phases/empreintes, utilise le vrai probe et
vérifie l'ordre de ses réponses. Une revue LLM n'a pas remplacé ces reproductions.
Les fonctions/tests et documents maintenus portent ces apprentissages ; cela ne
prouve pas leur lecture par des agents ou contextes déjà démarrés. Autres défauts
de méthode retenus : chemins et contextes de patch supposés, sorties trop larges,
assemblage de contexte suivi à tort d'une étape dépendante et ancien chemin du
candidat consommé après son déplacement par l'installateur. La reprise utilise
les chemins observés, des étapes dépendantes séparées, les compteurs vérifiés et
le binaire installé contrôlé par `/proc` ; aucune recette verte antérieure n'a
été réécrite pour masquer ces erreurs.

La qualification de défauts disque sur le nouveau chemin S1 n'est pas établie ici.
Conserver cette suite au propriétaire SCALING-S1 : faute de publication sur une
fixture valide, refus avec cause/effet incertain, restauration puis remise unique.
Les preuves du stockage atomique existant ne suffisent pas à annoncer ce parcours.
