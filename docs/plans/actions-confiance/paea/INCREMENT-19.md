# PAEA — incrément 19 : comparer deux actions depuis le menu

## Reprise et décision APEX

Le 26 octobre 2026, « reprends glm » poursuit PA-08/PA-01 après le commit
`444f74e` et la relance locale dev93. IA3 EP-01/04/05/07/10 et AC-02 ne sont
pas remis ; aucun rôle humain ni droit protégé n’est inventé. Codex racine garde
l’intégration. Une contribution GLM interne propose seulement la validation et
le rendu du rapport de comparaison ; elle ne voit ni dépôt ni outils.

Le comparateur pur et la CLI existent, mais leur parcours n’est pas accessible
depuis le menu. Livrer **Comparer deux actions** : deux documents de proposition
et contexte, champs modifiés et identités avant/après, indicateurs canonique/
intention/clé de tentative, export brut. Le rapport ne contient pas les valeurs
des champs. Une égalité ne valide aucun accord, identité, origine ou effet.

Options : dupliquer un contrôleur pour la troisième page, ou réutiliser le
contrôleur de fichiers/session et une opération explicite. La seconde option
préserve les refus UTF-8, erreurs, effacement et absence de retry déjà qualifiés.
Le noyau et le hash v1 restent inchangés.

## Séquence et critères observables

1. Conserver le témoin reçu valide et le refus 404 de la nouvelle route avant
   implémentation ; lire les vrais schémas/producteurs et leurs consommateurs.
2. Ajouter `POST /api/v1/protection/compare` derrière les gardes natives.
   Enveloppe fermée `{schema_version:1,before:String,after:String}` ; mêmes
   erreurs/borne de transport. Rapport strictement identique à la CLI par le
   comparateur commun, sans IO Root, politique ou autorité nouvelle.
3. Troisième page indépendante des UX métier, styles/contrôleur partagés,
   deux fichiers UTF-8 exacts, pas de conversion des nombres d’entrée.
   Rapport validé avant affichage/export, textes seuls, liens langue/thème,
   erreurs/reprises visibles ; égalité, requête seule et contenu distingués.
4. Recettes permanentes : schéma, API deux domaines, admission/refus/cause sûre/
   reprise/Root inchangé ; navigateur natif deux domaines, FR/EN, thèmes,
   1440/390/320, clavier/téléchargement, corruption client séparée, zéro
   diagnostic inattendu. Rejouer les deux vues existantes car contrôleur partagé.
5. Documentation FR/EN, revue des politiques/fonctions/erreurs liée aux octets,
   profil de production et livraison locale supportée ; Version/notes/commit/
   dirty/assets servis contrôlés. Pas de commit/push déduit de cette reprise.

Preuves locales privées : `evidence/20261026-compare19-5jvm093y/`. Leur présence
dans un clone n’est pas promise. La source historique et les résultats 18 restent
conservés. Une lecture initiale a supposé un nom de schéma inexistant ; l’inventaire
`rg --files` a fourni les vrais `http-receipt-check-v1` et `comparison-v1` avant
consommation technique. Aucun payload n’a utilisé le nom supposé.

État initial : tranche décidée, contribution GLM et témoin avant implémentation
en cours. Même UID et données non authentifiées ; aucune assurance protégée
réalisée par cette page. **0/17 invariants de bout en bout**, E2/E3 et RETEX final
du programme restent ouverts.


## Résultat et RETEX de la tranche 19

La page **Comparer deux actions** est réalisée par Codex et installée localement
le 26 octobre 2026. L'appel GLM interne a échoué avec HTTP 504 après 600,055 s ;
aucune réponse exploitable ni usage n'a été remis. Aucun code GLM n'est intégré,
aucun coût nul n'est déduit et aucun appel identique n'est relancé. Les originaux
sont conservés dans `glm-implementation-01/` et `glm-504-replan.json`. Le besoin
initial a été poursuivi directement après ce diagnostic.

| Critère | Résultat et preuve réelle |
|---|---|
| Contre-épreuve avant raccord | Binaire normal dev93, Root isolé : reçu valide 200 puis comparaison 404 ; arrêt propre. `baseline-public-result.json`. |
| Schéma et contrat HTTP | `http-recipe-01` PASS : deux tests de schéma et sept tests HTTP nommés, deux domaines, exact i64, contrôles/refus/reprises et Root inchangé. Deux tests de comparaison ajoutés. |
| Parcours navigateur natif | `native-recipe-01` PASS : **1 310 assertions**, **12 configurations par vue, trois vues**, **62 refus natifs attendus**, zéro diagnostic inattendu. CLI/API et téléchargements identiques ; onze injections client séparées, avec témoins natifs et reprise. |
| Supervision des erreurs du navigateur | Deuxième étape native : perte de script injectée après contrôles valides, sortie 1 et cause attendues. Aucun timeout global. |
| Profil livré | Collecteur source `rust-69peqv1h` : `VERIFIED_DEFAULT_NATIVE_PROFILE`, features des quatre composants sélectionnés vides, aucune fixture. Sept fichiers compilés PAEA comparés à l'archive effective, octets identiques. `candidate-dev95.json`. |
| Qualité et documentation | Clippy du lib web avec `-D warnings` PASS ; Rustdoc privé PASS à la deuxième tentative. Deux avertissements moteur étrangers sont conservés et ne sont pas annoncés résolus. Revue politiques/fonctions/erreurs liée à 27 fichiers dans `source-review.json`. |
| Installation effective | `./swarm.sh restart --no-open` PASS, dev95, PID **447031** observé, SHA256 `ba404609f9f5ddd3060b3b6baa2060922b62ab740c6d02e33ab2775e5c01476c`, commit compilé `fcda062f61ffc7a6ff2867d5ff945402df0d6547`, modifications non commitées déclarées. |
| Instance locale | `live-recipe-01` PASS : **34 assertions en lecture seule**, **11 ressources protégées** identiques aux sources, entrées de comparaison/résolution vides, Version et notes exactes FR/EN, lien du commit, dirty, focus, zéro écriture/diagnostic inattendu. Échec de script séparé attendu. |
| Intégrité | `recipe.py verify` donne VERIFIED/PASS pour HTTP, navigateur natif et instance locale. Revue visuelle des résultats FR/clair/390 et EN/sombre/320, absence de débordement et interactions réelles. |

Les premières compilations ont rencontré des modifications moteur concurrentes :
module manquant puis `preparation_plan::answer_questions` absent (E0583/E0425).
Le premier build local a refusé une observation Git modifiée par l'apparition
au statut d'un plan de migration ; le second a refusé le profil non compilable.
Les refus restent originaux. Le candidat dev95 ultérieur n'est réutilisé qu'après
qualification normale et comparaison des octets PAEA archivés. Aucune garde ni
source étrangère n'a été modifiée pour forcer la livraison. L'instance observée
avant notre relance était entre-temps dev94 ; la relance qualifiée a activé dev95.

L'exécution HTTP utilise un instantané figé de 910 entrées déclarées, précédé
d'un inventaire des includes sans manquant. Les modules PAEA, tests, schémas et
interface sont identiques aux sources finales ; seule la liste étrangère des
capacités de préparation diffère dans `service.rs` (ajout `answer-questions`).
Les gardes et la route PAEA sont inchangées. Les tests HTTP sur cet instantané,
le profil effectif dev95 et les parcours CLI/navigateur sur son binaire restent
des preuves distinctes ; les documents ajoutés ensuite ne changent aucun octet
exécutable. `http-proof-current-input-comparison.json` conserve cette portée.

Le diff des fichiers de cette tranche est propre. Le contrôle global conserve
deux blancs EOF étrangers déjà présents, dans `DELIVERY-PREPARATIONS-20261025.md`
et `task_validation/managed_receipt.rs` ; aucune conformité globale n'est déduite.
L'index Git a évolué dans les travaux parallèles ; aucun stage, reset, commit ou
push n'a été effectué par cette reprise. Les 37 traductions et la note FR/EN de
cette tranche sont inventoriées séparément pour préserver les passages étrangers.

Apprentissages vérifiés : comparer les octets d'un candidat qualifié existant
permet de réutiliser sa preuve sans relancer une construction identique ; un
contrôle Git peut refuser aussi sur un changement de statut documentaire. Un
inventaire Git nul-délimité exige un lecteur compatible : une première lecture
par lignes a produit un diagnostic démesuré, corrigé par `git -z`, sans écriture
d'index. Ces limites et les preuves sont transmises par le plan canonique et ce
suivi, sans prétendre modifier les contextes d'agents déjà démarrés.

L'amélioration de progression streaming SKYNET déjà retenue en incrément 18
reste ouverte : mainteneur SKYNET, bénéfice attendu = distinguer progression,
interruption et erreur avec preuve SSE/annulation/usage ; aucune implémentation
ni réparation du 504 n'est revendiquée. Reprise conditionnée à une route corrigée
ou un mode streaming qualifié, dans les droits/budgets disponibles.

Cette tranche de données est livrée et vérifiée localement, revue dans la même
session. IA3 EP-01/04/05/07/10, AC-02 et propriétaires humains restent les
préconditions de la suite protégée : fournir ces remises et autorités avant de
qualifier l'admission/accord/engagement réels. **0/17 invariants de bout en bout** ;
E2/E3, acceptation indépendante et RETEX final du programme restent ouverts.


## Commit demandé et continuation directe

Après la vérification dev95, l'utilisateur demande « commit puis fais en direct
sans glm ». Le commit de cette tranche est autorisé ; la suite est réalisée par
Codex directement, sans appel GLM ni autre délégation. L'isolation du commit garde
les 108 chemins déjà préparés et les passages étrangers des fichiers partagés.
Les preuves de livraison ci-dessus conservent leur commit compilé historique ;
un commit Git seul ne reconstruit pas le binaire. Aucun push n'est demandé ici.
