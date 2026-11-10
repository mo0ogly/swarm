# Premier lot mémoire : résultats et RETEX

Itération du 15 octobre 2026, autorisée par « démarre ». Réalisation par Codex
seul dans le checkout existant ; pas de mission native, de délégation, d’appel
LLM payant, de commit ni de déploiement. Les modifications étrangères du checkout
sont conservées. La recette utilise exclusivement des racines temporaires.

## Résultat et contrat livré

Le parcours RETEX vérifié → admission d’une expérience/procédure → sélection
explicite pour une tâche exécutante future → contexte reçu → remplacement/retrait
fonctionne dans la recette locale. La réception est observée dans l’entrée d’un
processus fournisseur Python déterministe, lancé par le moteur réel. Ce processus
capture le texte ; il ne démontre pas qu’un LLM applique correctement la procédure.
Le contrat courant et les exemples sont dans [la documentation FR](../../PROJECT-MEMORY.md)
et [EN](../../en/PROJECT-MEMORY.md).

Les versions et décisions résident dans le travail source SQLite, avec révisions,
événements, auteur local, motif et empreintes. Les sélections sont attribuées à la
tâche puis figées dans le contexte de tentative. Le catalogue et les aperçus de
correction exposent les tentatives actives détenant les versions concernées.
Import et déplacement ne transmettent pas l’admission locale. Migration de stockage
33 avec sauvegarde ; aucune migration de la base réelle du projet n’a été exécutée.

Le lot reste plus étroit que certaines options du cadrage : recherche littérale,
pertinence décidée manuellement, tâches exécutantes seulement. Pas de classement
sémantique, d’extraction automatique, d’évolution libre des consignes communes ou
de détection générale des contradictions. La garde contrôle le premier appel
fournisseur ; les conversations déjà démarrées conservent leur contexte et leur
responsable décide de leur poursuite. Le retrait conserve les sources historiques.

## Matrice de preuve

| Critère du lot | Résultat | Preuve et limite |
| --- | --- | --- |
| M3.1 Admission, remplacement, retrait, réouverture, rejeu et conflit concurrent | PASS local | `TestProjectMemoryReplacementPersistenceAndStaleSource`, `ConcurrentReplacement`, `PublicLifecycle` ; conflit concurrent aussi contrôlé avec `-race`. |
| M3.1 Migration et import sans autorité importée | PASS local | `TestProjectMemoryMigrationPreservesOriginals`, `ArchiveDoesNotImportAdmission`. Fixtures initialisées par les tests, sans acceptation métier réelle. |
| M3.2 Sélection atomique, contenu exact, conditions, provenance et attribution | PASS local | `InvalidInputsAndAtomicSelection`, `PublicLifecycle` et recette navigateur avec capture de l’entrée réelle du processus fournisseur. |
| M3.2 Source modifiée et retrait après préparation | PASS local | `ReplacementPersistenceAndStaleSource`, `LegacyGuideAndPreparedGuard`. Les contextes historiques ne sont pas réécrits. |
| M3.2 Anciennes voies guide/profil/pièces de reprise, chemins équivalents | PASS local | `LegacyGuideAndPreparedGuard`, `GuideReferenceNormalization` ; rôles et lecture pendant transaction couverts. Le texte copié librement ailleurs n’est pas effacé. |
| M3.3 UI/CLI/API, refus sans écriture partielle | PASS local | Recette publique, tests HTTP et validations Go communes. Source modifiée après aperçu : HTTP 409 visible et saisie conservée. |
| M4 FR/EN, deux thèmes, clavier/focus, largeurs et rendu | PASS local | `browser-verified-contract/checks.json` : 1440/1024/760/390/320 px, pas de débordement, contraste de texte contrôlé, aperçu/Escape/retour de focus, contenu HTML affiché comme texte. |
| M4 Compatibilité des profils et contrôles existants affectés | PASS local | Tests ciblés de profils, contexte, dialogue, RETEX et apprentissage de consignes ; contrôles nommés dans les sorties originales. |
| Utilité sémantique, comparaison avec/sans rappel et consommation LLM | NOT TESTED | Aucun vrai modèle ; délai de recette et code de sortie ne mesurent pas ces bénéfices. |
| Revue indépendante et acceptation utilisateur | NOT TESTED | Relecture par le même exécutant ; aucune acceptation indépendante ou globale n’est déclarée. |

Les attentes de `cas-evaluation.json` sont conservées. La couverture produit du lot
est séparée de leur proposition initiale : C04 et C08 vérifiés sur les expériences
versionnées ; C05/C06/C09 partiels selon les limites de la recette ; C11/C14 ne
prouvent pas l’apprentissage procédural ni une comparaison comportementale.
Les autres usages restent ouverts. Aucun taux global de mémoire réalisée n’est déduit.

## Originaux et reproductibilité

- Échec avant implémentation : `evidence/m1-20261015/swarm-m3-red.log` (route absente).
- Régression des alias reproduite avant correction : `swarm-m3-reference-red.log`.
  Après correction : `swarm-m3-reference-fixed.log`, 20 tests de premier niveau
  réussis, 1 export de fixture optionnel ignoré. Le test ignoré n’est pas un succès.
- Autres contrôles ciblés : `swarm-m3-final-contracts`, `swarm-m3-receipts`,
  `swarm-m3-input-validation`, `swarm-m3-race` ; chaque paire `.json/.log` conserve
  commande, dates, code de sortie et empreinte du journal. Les tests se recouvrent :
  leurs nombres ne s’additionnent pas en une campagne indépendante.
- Recette publique : `tests/project_memory_ui.cjs`, paramètres et résultat dans
  `swarm-m3-ui-verified-contract.json/.log`, captures et assertions dans
  `evidence/m3-20261015/browser-verified-contract/`. Les annulations de navigation du cockpit
  initial sont isolées ; aucune requête de la page mémoire n’est ignorée.
- Sources : `evidence/m3-20261015/baseline.json`, `preexisting.patch` et manifestes
  du candidat. Branche initiale `codex/clear-product-method-names`, base
  `4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, avec travail non commité. Le binaire
  de recette est temporaire ; construire ne remplace pas le serveur utilisateur.

Contrôles complémentaires : `swarm-m3-vet-release` (analyse statique),
`swarm-m3-i18n-final` (catalogues et génération sans mutation),
`swarm-m3-methods-verified` (méthodes et capacités), et `git diff --check`.
Le manifeste `candidate-inputs-verified-before.json` couvre les sources Go et
les ressources réellement embarquées ; le contrôle après construction confirme
916 entrées inchangées et conserve le SHA-256 du binaire dans
`candidate-inputs-verified-after.json`.

## Incidents, décisions et prévention

| Observation originale | Cause et décision appliquée | Effet vérifié et prévention |
| --- | --- | --- |
| Compilation momentanément impossible sur `trustDataCapabilities` | Fichier d’un autre chantier en cours d’écriture. Inspection de la définition apparue ; aucune modification de ce chantier. | Compilation suivante et analyse statique passent. Conserver la frontière du candidat et ne pas corriger des sources étrangères sans diagnostic. |
| Fixture de profil refusée | Le producteur exige tous les rôles, le test n’en déclarait qu’un. Fixture alignée sur `projectRoles`. | Tous les rôles et un appel de planification sous transaction sont contrôlés. Lire le producteur avant de supposer son interface. |
| Admission refusée après import | Les preuves archivées n’établissent pas les fichiers actuels de la racine destinataire. | Fixture connue matérialisée après comparaison d’empreinte puis contrôlée par `evaluate --phase delivery`. Pas de modification des décisions d’acceptation historiques. |
| Tentative de nouveau gate sur une tâche acceptée refusée | Mauvaise opération de recette ; le refus moteur était correct. | Recette corrigée pour évaluer les preuves actuelles, sans rouvrir artificiellement la tâche ni désactiver de garde. Originaux `swarm-m3-browser*` conservés. |
| Assertion finale affectée par des requêtes abandonnées du cockpit initial | Navigation volontaire vers la page mémoire avant la fin des requêtes du cockpit. | Origine de chaque requête enregistrée ; seules ces annulations précises sont classées séparément. Erreurs et requêtes échouées de la mémoire restent bloquantes. |
| Liens `./leçon.md#section` et pièce `./chemin` contournaient les contrôles introduits | Comparaison textuelle au lieu d’une résolution de référence. | Deux tests rouges conservés, normalisation commune puis tests verts. Les variantes encodées et un lien voisin à conserver sont couverts. |
| L’ajout de prévention ne tient pas dans le contexte de planification | 32 204 octets au premier contrôle, puis 32 041 après une première condensation, pour une capacité existante de 32 000. | Originaux `swarm-m3-workflow-release` et `swarm-m3-workflow-compact` conservés. Répétitions condensées sans retirer d’exigence ni relever la capacité ; `swarm-m3-workflow-verified` passe pour les quatre rôles et vérifie la réception exécutante. |

Apprentissage réutilisable : un retrait doit être vérifié au consommateur par
toutes ses voies d’entrée, notamment les voies héritées et les références
équivalentes. Cette obligation figure désormais dans le contrat partagé
`tools/agent-workflows/CONTRACT.md`, lu par les méthodes et embarqué dans les
contextes des rôles. Les instructions seules ne prouvent pas l’effet : les tests
du moteur et la recette de réception restent nécessaires. Une reconstruction
est requise pour diffuser ce contrat dans de nouveaux processus Swarm. Le candidat
reconstruit est contrôlé par `swarm-m3-workflow-verified` : les quatre rôles
passent ; le processus exécutant reçoit le contexte et le conserve après
réouverture. Ce contrôle ne garantit pas la qualité de tous les plans ultérieurs.

## Suite proposée après ce lot

Propriétaire du choix : utilisateur ; réalisation : Codex dans le périmètre adopté.
La prochaine expérience utile est une comparaison bornée avec/sans rappel sur une
procédure et une variante nouvelle, avec modèle, entrées, résultat attendu et budget
identifiés. Elle doit mesurer l’action correcte et l’absence de régression, pas
seulement la présence du texte. C’est la preuve manquante de C05/C10/C11/C14.

La reprise de conversation après arrêt, la projection de séquences d’outils, le
résumé progressif et la suppression restent des lots distincts (C15–C18). Le
présent RETEX clôt l’itération locale ; il ne les transforme pas en capacités
livrées et ne lance aucune expérience payante ou mission supplémentaire.
