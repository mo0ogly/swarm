# Sauvegarde générale demandée le 17 octobre 2026

Demande explicite : « fait un commit general ». Le commit regroupe l'état
présent des chantiers mémoire, identité/confiance, reprise et revue du moteur,
interface, méthodes, documentation et preuves. Cette sauvegarde générale est
autorisée comme un ensemble ; elle ne déclare pas chaque chantier accepté.
Branche de départ : `codex/clear-product-method-names`, parent observé
`4e58f7cbf8200a58f69f1f4d03d657c3313fb642`.

## Contrôles de préparation

- Inventaire initial : 7 716 fichiers modifiés/nouveaux non ignorés, environ
  308 Mo avant compression Git, dont les preuves conservées dans les plans.
- Aucun chemin de runtime `.swarm`, fichier `.env`, clé privée ou base SQLite
  dans cet inventaire. Les exclusions Git existantes restent appliquées.
- Examen par signatures de jetons, clés privées, JWT et URL d'identification :
  43 occurrences correspondent à l'URL littérale d'un test négatif local
  `TestActionsConfianceT4RemoteInvalidOrigins` et à ses copies dans les preuves.
  Aucun identifiant réel détecté par ces règles ; ce contrôle ne constitue pas
  un audit de confidentialité complet, notamment des captures PNG.
- `python3 tools/agent-workflows/check.py` réussit : 12 méthodes partagées et
  cinq tests/sous-tests de contexte. `git diff --check` du diff suivi réussit.
- Les résultats des tests applicatifs restent ceux de chaque rapport, sur
  leurs sources et entrées propres. Pas de nouvelle suite applicative globale
  pour cette opération Git ; aucune modification du comportement produit.
- Identité Git existante utilisée, sans attribution ajoutée. Aucun push,
  déploiement ou redémarrage demandé par cette sauvegarde.

## Incident Git et préservation

La première tentative `git add -A` échoue car `.git/index.lock` existe déjà.
Le verrou est vide, âgé d'environ 68 heures ; aucun processus Git n'est observé.
Il n'a pas été supprimé. Le contrat partagé impose « Do not delete locks » ;
son âge seul ne donne pas l'autorisation de le retirer.

La préparation utilise un index Git isolé dans un répertoire temporaire,
sans modifier l'index partagé ni le verrou, pour rendre le contenu et les
contrôles du commit examinables. L'utilisateur autorise ensuite explicitement :
« Oui, archive le verrou et finalise le commit ». Après recontrôle de l'inode,
de l'absence de détenteur, du HEAD et de l'empreinte de l'index partagé, le
verrou est déplacé avec son inode conservé vers
`.git/retained-locks/index.lock.orphan-20261017`. Une copie de l'index partagé
initial est aussi conservée dans les reçus temporaires. La finalisation reprend les
opérations Git ordinaires sur la branche actuelle ; les changements intervenus
dans le checkout partagé sont examinés avant l'enregistrement.

Le premier déplacement vers `/tmp` a échoué avec `EXDEV` : volumes différents,
verrou inchangé. L'ajout Git suivant a été tenté trop tôt et a rencontré le
même verrou ; ces deux erreurs sont conservées dans les reçus. Reprise sur
une destination du même volume et vérification du code de sortie avant chaque
opération dépendante. Ne pas déduire la réussite d'une étape de son lancement.

## Conservation des octets des preuves

Le premier contrôle de l'index isolé signale 1 168 espaces ou fins de fichier :
1 166 dans 48 artefacts historiques sous les répertoires de vérification
confiance ou de preuves mémoire, et deux dans des recettes actives. Les archives
contiennent sorties originales, contexte de patch et préimages. Modifier ces
données invaliderait leurs empreintes. L'alternative
retenue étend les attributs Git déjà utilisés pour les preuves brutes à ces
deux racines d'archives précises. Les trois règles purement typographiques
`blank-at-eol`, `blank-at-eof` et `space-before-tab` ne s'appliquent pas aux
octets historiques ; sources applicatives et documents hors archives gardent
leurs contrôles. Aucun artefact original n'est réécrit. Le refus original de
Git reste conservé dans `staged-diff-check.out` des reçus de préparation.

Les deux blancs des recettes actives sont corrigés sans changement logique :
une ligne finale vide dans `tests/actions_confiance_effect_proof.py` et les
espaces d'une ligne vide dans `tests/actions_confiance_secret_data_ui.cjs`.
Comparaison AST Python identique, comparaison hors espaces identique et
`node --check` réussi ; aucune campagne fournisseur ou navigateur relancée
pour cette modification purement typographique. Originaux dans les reçus.

English: retained qualification archives contain exact stdout, patch context
and historical source bytes. Git whitespace attributes preserve these two
archive roots without altering their original hashes. Application sources
and documents outside these archives keep normal checks. This is a Git
representation rule, not a runtime behavior or a claim of global acceptance.

Reçus locaux de préparation : `/tmp/swarm-general-commit-_rfszjyx/`.
La documentation opérationnelle des fonctions n'est pas modifiée : le besoin
présent porte sur l'enregistrement Git, pas une nouvelle capacité produit.

## Complément demandé avant le commit : README mémoire et identité

L'utilisateur demande aussi d'expliquer leur apport à la gestion de workflows
agentiques. Les README FR/EN présentent désormais la mémoire de projet
disponible dans les sources, ses versions, ses conditions, sa sélection et la
conservation de l'histoire ; le noyau identité et ses permissions/délégations
sont distingués de son raccord public encore en cours. Propriété et auteur
attestés, droits d'usage mémoire et approbation humaine protégée ne sont pas
présentés comme déjà garantis. Les usages développement et la cible métier
sont reliés à la décision d'architecture existante.

Références vérifiées : `docs/PROJECT-MEMORY.md` et sa version anglaise,
`docs/architecture/AGENT-IDENTITY-CONTRACT.md` et sa version anglaise,
`internal/engine/agent_identity_authority.go`, `docs/PRODUCT-ARCHITECTURE.md`
et le point de reprise courant du chantier identité. La modification est
documentaire ; liens et diff sont vérifiés, sans nouveau test applicatif ou
prétention de déploiement. La recherche avait encore repris le chemin absent
`docs/REFERENCE.md` ; elle est corrigée par les chemins réellement inventoriés,
sans création d'un doublon.
