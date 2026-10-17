# IA3c.1 — reçu durable d’émission humaine

17 octobre 2026 — **PASS ciblé privé ; admission et sécurité globales ouvertes.**

Le reçu de la décision humaine est désormais conservé dans la tête du registre,
son intention de récupération et le journal SQLite. Il lie l’autorité fournie,
la session sous empreinte et les dates au candidat exact. Il reste historique :
sa lecture ne renouvelle aucun droit et ne prouve pas un enrôlement réel.

## Besoin, décision et périmètre

IA3b-I04 : la preuve privée du permis disparaissait après redémarrage. Deux voies
comparées : nouvelle table/transaction, ou reçu inclus dans la tête déjà chaînée.
Seconde voie retenue pour réutiliser intention → COMMIT → finalisation sans
migration. Tête humaine format 2, reçu format 1, schéma SQLite toujours 37.
Le format 2 sans reçu et le format 1 avec reçu sont refusés ; les anciennes têtes
privées sans reçu restent lisibles dans une chaîne mixte.

Codex seul, HEAD de référence `4e58f7cbf8200a58f69f1f4d03d657c3313fb642`,
branche `codex/clear-product-method-names`, checkout partagé avec travail sale
préexistant. Aucun commit, déploiement, fournisseur réel/payant ou modification
de mission vivante. Six sources possédées, avec empreintes dans
[source-final.json](source-final.json) : deux nouveaux fichiers reçu/tests et
quatre fichiers existants émetteur/types/publication/lecture du registre.
Les sources et processus du chantier mémoire sont préservés.

## Résultats observés

| Contrôle | Résultat |
| --- | --- |
| Reproducer avant correction, vue stable | 1 FAIL attendu : reçu absent après réouverture |
| Lot final normal, watchdog global désactivé | 117 noms PASS, 0 FAIL, 0 SKIP |
| Même lot final sous `-race`, watchdog global désactivé | 117 noms PASS, 0 FAIL, 0 SKIP |
| Contrôles propres au reçu, inclus dans ces lots | 37 noms dans chaque mode |
| `go vet` moteur | PASS |
| Compilation de l’application | PASS, binaire conservé, non déployé |
| Entrées exactes comparées entre contrôles finaux | 1064 identiques, aucun changement |
| Schémas FR/EN | 4 rendus SVG/PNG réussis ; 2 images inspectées |

Ces nombres incluent les tests parents et sous-cas ; ils ne comptent pas des
attaques distinctes et ne constituent pas une comparaison scientifique.

Le reçu conserve le principal exact, projet/plan/révision Trust, référence
d’enrôlement, permissions/délégation, intervalle d’autorité, empreinte et expiration
de la session, émission et fin de validité égale au minimum des expirations.
Les listes sont possédées. La validation historique ne dépend pas de l’heure de
lecture. Les tests couvrent 29 variantes invalides, les bornes d’émission/expiration,
le retour d’horloge, la réouverture, l’altération du journal et la récupération.

Les deux reprises sont injectées après intention persistée et après COMMIT.
Elles avancent une chaîne format 1 → format 2, relisent exactement la même tête
et le même reçu après réouverture, conservent deux instantanés/deux événements
et refusent toute publication nouvelle avec la session expirée. La capacité
privée originale est conservée par la fixture : sa récupération hors processus
n’est pas implémentée. Aucun jeton brut n’est stocké dans le reçu.

## Preuves et reproductibilité

- [Échec préalable stable](receipt-before-02-run.json),
  [trace](receipt-before-02.jsonl), sources avant/candidat/final conservées.
- [Lot final normal](normal-contracts-02-run.json) et
  [lot final instrumenté](race-contracts-02-run.json) : mêmes 117 noms, empreintes
  des entrées et des binaires vérifiées avant/après.
- [Analyse statique](vet-01-run.json), [compilation](app-build-01-run.json),
  [binaire application](app-binary.json), [vérification des manifestes](finalize.py).
- [Vue des voisins mémoire](frozen-neighbors.json) : 16 fichiers figés pour le
  compilateur ; ressources web figées depuis IA3b (136 fichiers). L’overlay
  préserve les sources du checkout et déclare ses remplacements. Ces contrôles
  n’acceptent pas le dernier chantier mémoire/web en cours d’édition.
- [Schéma français](receipt-fr.svg), [schéma anglais](receipt-en.svg),
  [commandes de rendu](diagrams.json), liens locaux contrôlés.

La recette [run_checks.py](run_checks.py) conserve des identifiants de tentative
uniques et refuse l’écrasement d’un résultat. Les identifiants déjà exécutés ne
sont pas à relancer : une nouvelle tentative exige diagnostic et nom distinct.
Une reproduction exige la vue compilateur déclarée et les sources correspondant
aux empreintes ; les sources critiques et voisins figés sont conservés ici,
les ressources web exactes dans les preuves IA3b référencées.

## Replan : le délai de 90 secondes était arbitraire

Les 90 s provenaient d’un choix de Codex, repris d’une recette antérieure sans
contrat ni mesure le justifiant pour ce lot. Ce watchdog Go n’était pas une
politique du moteur. L’objection utilisateur a identifié un défaut de méthode :
découper les tests ne justifiait pas la borne initiale.

Le [lot registre original](race-registry-01-run.json) atteint 90 s : 22 noms
passés, 1 échec attribué au cas en cours par le timeout ; **lot incomplet**.
Les lots séparés réussissent, mais ne sont pas le critère final. La recette
courante désactive explicitement le watchdog global (`-test.timeout=0`) ;
l’[aide du Go installé](go-test-timeout-help.txt) vérifie cette sémantique.
Le lot complet instrumenté finit en 143.5 s selon le lanceur,
et le normal en 14.3 s, instrumentation comprise.
Le suivi porte sur les événements réels, avec diagnostic si la progression cesse.
Aucun délai contractuel, droit, budget ou réglage opérateur n’est supprimé.

La précision utilisateur étend la règle aux autres blocages de test tant que
les limites du code ne sont pas comprises : origine, portée, risque protégé,
progression et mesures avant ajout/durcissement ; distinguer lenteur, blocage
réel, limite fonctionnelle et limite de méthode. Tentatives, itérations, volume,
concurrence et conditions d’arrêt sont concernés, sans contournement des
protections existantes. Cette extension de consignes est documentaire ; les
entrées Go vérifiées restent inchangées, les contrôles réussis sont réutilisés.

Ce n’est pas une mesure de performance produit : la latence sous charge observée
auparavant reste ouverte. Les durées variables des fixtures/compilations ne
permettent pas d’isoler un gain causal. La compilation instrumentée a pris
292.5 s ; paquet volumineux et pression hôte
sont des facteurs possibles, conservés dans [l’observation](compile-observation.json).

Autres reprises : la première tentative de reproducer coïncidait avec deux
sources mémoire modifiées : elle reste une preuve instable, remplacée par une
vue voisine explicite ; un chemin Chromium supposé absent est corrigé par la
configuration vérifiée du projet. Aucun original n’est effacé ou requalifié.

## Documentation, transmission et RETEX de cet incrément

Contrat FR/EN et schéma, contrat IA2, plan, note scientifique et programme de
recherche sont mis à jour ensemble. La leçon du délai arbitraire est inscrite
dans les pages méthodes FR/EN, `AGENTS.md` projet et accords personnels globaux ;
la règle personnelle est reçue dans le nouveau contexte de cette conversation.
[Empreintes de transmission](instructions-persistence.json). Ces fichiers
persistants ne mettent pas à jour les processus déjà démarrés. Le contrat
embarqué n’a pas changé dans cet incrément.

RETEX local : une preuve privée éphémère ne suffit pas pour auditer une émission
après réouverture. Un reçu obligatoire dans un format explicite, lié au candidat
et au journal existant, conserve cette décision et sa reprise. Cela ne remplace
pas la source d’autorité fraîche. Côté méthode, un délai sans justification peut
transformer un cumul de tests valide en échec artificiel ; sa suppression doit
être accompagnée d’un suivi réel et d’une exécution complète, pas d’une promesse.

## Limites et reprise

IA3b-I04 est **partiellement traité** : reçu historique vérifié ; enrôlement,
persistance/révocation et témoin indépendants restent à réaliser. Le reçu n’est
pas une signature indépendante, un accord humain distinct ou une capacité
reconstructible depuis un import. Les tests utilisent une autorité explicite
et un fournisseur OIDC local signé, sans compte humain externe.

Codex porte la source d’enrôlement protégée et son contrôle de fraîcheur avant
raccord public. L’opérateur doit désigner le principal humain initial et la
frontière de contrôle indépendante des agents ; aucun Admin/jeton cockpit ou
premier appelant ne peut les remplacer implicitement. Le témoin doit couvrir
également le watermark du plan. Admission publique, délégation effectivement
reçue, création exacte, départ/consommation/réconciliation, droits mémoire,
F01–F07, IDA, IA4, IA5 et RETEX global restent ouverts. La latence sous charge,
les fixtures/paquets partagés et le défaut de review-retry antérieur gardent leur
suivi et leur propriétaire ; cet incrément ne les clôt pas. Aucune priorité
scientifique ni supériorité comparative n’est démontrée.
