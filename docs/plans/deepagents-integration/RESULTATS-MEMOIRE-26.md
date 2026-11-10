# M30 — captures cohérentes et confirmation de restauration

17 octobre 2026, reprise « next ». Codex seul, replan/audit-pdca avec corrections
autorisées ; auto-revue du candidat. Branche `codex/clear-product-method-names`,
HEAD `f32b9f8fa6e99f75845f57872a7f8a56a0ef9dc6`, sources initiales incluant M29
non commité. Baseline exacte dans `evidence/m30-20261017/baseline.json` ; les
travaux identité et les preuves M29 restent préservés. Aucun déploiement, service
système, fournisseur réel, délégation, agent natif, commit ou redémarrage.

**Effet :** une capture incohérente ou étrangère ne fournit plus d'aperçu de
restauration. Une confirmation est liée aux octets observés, revérifiés sous
réservation SQL avant effet. Le refus conserve la corbeille et les autres données.
Ce lot contribue à MEM-P1, qui reste ouvert pour les autres voies de diagnostic.

## Contrat réalisé

- `snapshotMission` relit le Work par le noyau M28 puis contrôle la capture
  complète avant mise en corbeille. Un échec retourne une capture absente.
- Le validateur `lifecycle_snapshot.go` impose le schéma externe 1, un seul Work
  concordant avec clé/révision positive/titre de la corbeille et l'enveloppe
  UTF-8/JSON de schéma 1. Il refuse tables ou colonnes dupliquées, noms de colonne
  invalides, cellules indécodables et tables inconnues, même vides. Les colonnes
  réellement insérables restent aussi contrôlées par SQLite à l'application.
- Les 38 tables du producteur ont une règle de périmètre : mission directe,
  agent/préparation capturés, demande/programme d'automation capturés. Le test de
  couverture lie ces règles à `lifecycleSpecs`. Les réservations de préparation
  conservent leur cas nul/vide lié à une préparation capturée ; un type incorrect
  ou un identifiant de mission étrangère n'est pas converti en absence.
- `snapshot_sha256`, optionnel dans aperçu et reçu, lie les octets exacts de la
  capture. Le jeton change même si tailles et comptages restent égaux. La
  transaction relit l'empreinte et revalide le contenu avant insertion, retrait
  de corbeille ou reçu. Un changement depuis l'aperçu exige une nouvelle
  confirmation. Le changement dans la transaction est aussi refusé.
- La restauration d'un marquage d'archive garde son parcours distinct et relit
  le Work cohérent dans la transaction. Un aperçu de corbeille ne peut pas servir
  à désarchiver une mission devenue active. Le rejeu exact conserve le reçu
  historique sans nouvel effet ; aucun digest rétrospectif n'est inventé.
- La recherche de corbeille n'est tentée que pour un Work SQL absent. Les causes
  de panne SQL restent présentes. Le décodeur de cellules commun à la restauration
  et à l'import automation conserve les erreurs via Unwrap, avec un message public
  qui ne recopie pas la valeur privée d'une cellule.

Les anciennes captures valides privées de tables optionnelles vides restent
restaurables. Il n'y a ni nouvelle migration ni seuil/politique ajouté. Les
contrôles de périmètre SQL n'attestent ni auteur, ni propriétaire métier, ni
permission d'agent. Ils ne revalident pas tous les champs JSON des objets liés.
Les tables d'identité protégée ne deviennent pas une autorité par restauration.

## Reproduction, correction et vérification

Preuves : [`evidence/m30-20261017/`](evidence/m30-20261017/).
Sources rouges conservées, racines temporaires isolées, aucun trigger désactivé.

| Série | Observation | Portée |
| --- | --- | --- |
| `red` | 19 cas précis FAIL, 20 noms en comptant le parent agrégateur ; un cas ancien valide PASS ; entrées stables | Quinze captures fournissaient un aperçu incohérent ; changement de taille égale, substitution dans la transaction, panne masquée et producteur incohérent reproduits. |
| `green` | Erreur Go de déclaration courte, zéro test collecté | Original conservé ; correction de compilation, aucune réussite comportementale déduite. |
| `green-current` | 33 noms PASS, 10 parents et 23 sous-cas | Refus, reprise valide, compatibilité, collisions atomiques et automation liée. |
| `delivery` / `race` | 128 noms normaux PASS, puis 35 sous race, entrées stables | Versions avant correction de confidentialité ; reçus conservés dans leur périmètre. |
| `red-private` | Deux cas précis FAIL et leur parent ; entrées stables | Les conversions int/float conservaient la cause mais recopiaient la valeur factice privée dans Error. |
| `final` | 131 noms PASS : 53 parents et 78 sous-cas ; vet/build PASS, entrées stables | Candidat avec diagnostic public fixe et causes errors.Is/As conservées. |
| `final-race` | 38 noms PASS : 15 parents et 23 sous-cas ; entrées stables | Nouveaux refus, reçu HTTP/rejeu, concurrence de confirmation, agent réservé après aperçu et restauration automation. |
| `cli-final` | Quatre assertions PASS, onze commandes sur le binaire compilé final | Corbeille réelle de fixture, refus sans stdout/mutation, substitution à taille égale refusée, nouveau reçu puis rejeu exact et lecture réouverte. Zéro appel modèle. |
| Schémas | Quatre rendus Mermaid FR/EN PASS | Deux diagrammes par langue ; syntaxe/rendu vérifiés, aucune recette navigateur d'UI modifiée prétendue. |

La série race recouvre la série normale : **131 noms distincts au total**,
53 parents et 78 sous-cas. Les volumes ne sont pas additionnés.

Les tests HTTP traversent `/api/v1/action`, avec session/origine/CSRF : refus
sans aperçu, restauration valide, empreinte du reçu, rejeu identique sans mutation
et réouverture du reçu persistant. Les contre-exemples incluent des lignes
d'agent, de préparation, d'occurrence et de journal de programme réellement
produites pour une autre mission dans la même fixture. Les comparaisons de toutes
les tables vérifient l'absence d'effet interdit, pas seulement le code de retour.

Les invocations conservent commande, horaires, sortie originale, code de retour
et empreintes avant/après. Aucun watchdog global justifié : Go `-timeout=0`,
subprocess sans timeout ; progression suivie. Les limites produit ne changent pas.
La recette CLI finale refuse l'écrasement de ses reçus ; le script de la première
recette reste conservé séparément, avec son ancien binaire.

## Critères

| ID | État | Preuve / limite |
| --- | --- | --- |
| M30-01 | PASS local | `red` et `red-private`, sources et entrées stables. |
| M30-02 | PASS sur le contrat de capture | Work unique, schéma/cellules, métadonnées et périmètres contrôlés ; 38 règles liées au producteur ; compatibilité optionnelle vérifiée. Pas de validation exhaustive des JSON métier liés. |
| M30-03 | PASS local | Changement de taille égale via public apply, relecture dans la transaction, reçus et rejeu HTTP/CLI. |
| M30-04 | PASS local | Toutes les tables comparées après refus ; reprise valide, corbeille intacte, collision atomique, export/import automation et réouverture. |
| M30-05 | PASS local | Normal/race/vet/build/CLI/rendus acquis ; noms attendus et empreintes vérifiés dans verification-summary.json. Aucune revue indépendante. |

## Suites préservées de MEM-P1

| Origine / suite | Propriétaire | Critère concret de reprise |
| --- | --- | --- |
| M29 : boucles `missionOwnedCycle` / `missionCycleObserved` | Codex mémoire/Store | Reproduire l'erreur d'inventaire, suivre diagnostic reçu, réconciliation sans nouvel effet puis reprise. Lecture du code confirmée en M30 ; sources de mission inchangées. |
| M29 : callbacks de fraîcheur, fallbacks et vues complémentaires | Codex mémoire/Store | Tracer la cause jusqu'au consommateur et au résultat ; distinguer enrichissement facultatif et succès incomplet avant correction. |
| M29 : incohérence après get, pendant la sonde de lancement | Codex mémoire/Store | Barrière déterministe locale, refus avant agent/événement puis reprise valide. Le garde transactionnel M29 existe ; cette injection spécifique reste à produire. |
| Objets JSON liés de la capture, dont `decodeAgentRow` | Codex mémoire/Store pour la représentation ; chantier identité pour son autorité protégée | Le validateur M30 suit le rattachement SQL, pas l'égalité de chaque corps Agent avec ses colonnes. Qualifier séparément les producteurs/lecteurs avant toute conclusion sur cette enveloppe ; aucun exploit supplémentaire prétendu. |
| Attribution et droits P2–P5 / M20 | Chantier mémoire et dépendance identité, décisions opérateur pour bootstrap/activation | Aucun critère M20 clos par la cohérence ou l'empreinte d'une capture. Préparations indépendantes conservées, pas d'activation publique implicite. |

## RETEX

La cause générale était un contrôle de format SQL sans contrôle du contenu
restauré et de sa portée. Des tailles/comptages ne lient pas un aperçu à ce que
l'opérateur a examiné. Valider dès l'aperçu et à l'engagement donne une explication
avant confirmation, puis empêche la substitution après cette confirmation. Le
même validateur au producteur évite de fabriquer une capture incohérente.

La nouvelle lecture d'aperçu a aussi rendu visible un défaut des messages de
conversion : conserver la cause ne demande pas de recopier la valeur privée dans
son texte public. Une erreur enveloppante sépare ces besoins sans masquer la
cause originale. Les deux tests int/float restent comme gardes de régression.

La documentation Work, mémoire et référence CLI est actualisée en FR/EN dans
le même incrément, avec diagrammes rendus et suivi persistant. Aucun contexte
agent, asset web ou politique n'est embarqué par cette modification ; pas de
génération correspondante requise. La page mémoire et le README gardent leurs
affirmations actuelles ; aucune retouche de présentation n'est nécessaire.

Limites : tests/HTTP/CLI sur fixtures, auto-revue seulement, aucun fournisseur
réel ou serveur actif qualifié. Le navigateur et les thèmes ne sont pas exercés
dans ce lot sans changement frontend. Ce RETEX clôt uniquement l'incrément local
avec ses preuves finales collationnées ; il ne ferme pas MEM-P1 ni le besoin
initial de propriété et de droits d'usage.
