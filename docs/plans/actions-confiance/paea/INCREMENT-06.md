# PAEA — incrément 06 : raccord CLI Rust, qualification différée

[English](../../../en/plans/actions-confiance/paea/INCREMENT-06.md) · [Plan](../PAEA-APEX.md)

## Réalisation en sources

Reprise « reporends » : méthode APEX/replan, contribution GLM SKYNET autorisée.
La bibliothèque Rust de l’incrément 05 n’avait aucun consommateur public. Choix :
le raccorder au lecteur CLI existant via deux fichiers positionnels, plutôt que
créer un nouveau document enveloppe ou modifier le parseur global des options.

```sh
swarm-rust protection compare /CHEMIN/avant.json /CHEMIN/apres.json --json --lang fr
```

**Commande implémentée en sources, non compilée, non exécutée, non installée.**
La consigne utilisateur sans compilation ni redémarrage demeure prioritaire.
L’ancienne commande `canonicalize` reste présente ; aucun ancien résultat ne
qualifie le nouvel adaptateur. Le programme PA-00…13 reste ouvert.

`protection.rs` appelle `compare_documents` après deux lectures par le mécanisme
existant. Les chemins sont des fichiers réguliers ; `-` est refusé pour cette
commande, afin de ne pas partager implicitement stdin entre deux documents.
Pour un fichier nommé `-`, fournir son chemin `./-`. Les liens vers des fichiers
réguliers suivent le comportement existant, sans établir propriété ou confinement.
La borne transport existante de 16 Mio s’applique à chaque document ; aucune
nouvelle limite métier, tentative ou durée n’est ajoutée. Les lectures successives
ne constituent pas un instantané atomique : les empreintes identifient les octets
lus, pas la fraîcheur du système de fichiers.

`--input` non vide, root différent de `.`, serveur, output/task non vides et phase
autre que `delivery` refusent. Les valeurs historiques vides de ces options
conservent leur comportement ; aucune autorité n’en découle. Aucun Root, secret,
réseau, service d’effet ni configuration projet n’est ouvert. CLI seulement :
API/UX et autorité protégée restent à raccorder.

## Sortie et erreurs

L’enveloppe JSON porte `schema_version:1`, `scope:"offline-data-only"` et
`comparison`. Celle-ci contient les identités `before/after` (SHA256 des documents,
actions et intentions), les chemins fixes triés sans valeurs et les trois booléens
`canonical_content_changed`, `intent_changed`, `transport_request_changed`.
`authority`, `effect`, `approval_validity` restent `not-established`.
Un contenu identique ne prouve aucun accord encore valide ; un changement doit
être présenté au contrôle protégé futur. Digests et changements restent des
métadonnées soumises aux droits de leurs destinataires.

Le schéma descriptif `schemas/paea/comparison-v1.schema.json` documente cette
sortie ; son accord avec un binaire n’est pas vérifié. Le schéma d’entrée v1
historique et ses formats restent inchangés. Aucun nouveau composant Python.

Les erreurs lecture/inspection/fermeture gardent `CommandInputError` et son code ;
les erreurs de document/canonicalisation gardent `Protection(Error)`. Aucun rapport
n’est préparé si un couple est invalide. La sérialisation précède l’écriture ;
`write_output` conserve les causes d’écriture/flush. Une publication partielle
reste possible : une sortie non nulle ne prouve pas absence d’octets publiés.
Aucun effet métier à réconcilier n’est créé par ce diagnostic. Une reprise est
explicite après correction de l’entrée ou du canal, sans retry automatique.

## Vérification différée et revue

Quatre tests CLI ajoutés, non exécutés : deux domaines FR/EN avec entrées inchangées ;
faute JSON/fichier absent sur chaque côté avec témoin/reprise ; options runtime
refusées ; échec `/dev/full` puis reprise. Le target CLI `protection` compte huit
tests sur Linux, dont les quatre historiques. Trois tests de bibliothèque de
l’incrément 05 restent écrits/non exécutés. La recette permanente `paea-contract`
est adaptée à ces cibles et compteurs réels, **non lancée**. Elle reste une recette
Linux comprenant ses vérifications historiques, sans watchdog global ajouté.

La revue statique conserve fonctions, politiques et erreurs liées aux octets dans
`source-review.json`. Les lectures ont corrigé deux chemins supposés inexistants :
parseur/aide sont dans `swarm-cli/src/lib.rs`, erreurs dans ce même parent. Aucune
relance sur une interface inventée. Dans ce fichier partagé, seules les deux
chaînes d’aide Protection sont modifiées ; les travaux étrangers sont préservés.
La migration SCALING-S2 reste en cours selon son plan lu. Pas de commit/push,
publication, installation, reprise de mission ou interruption de processus.

## Reprise restante

Codex doit qualifier les targets exacts `comparison` et `protection`, le socle
concerné et Rustdoc dès levée de la consigne sans compilation. Le raccord API/UX
ne doit pas transformer ce diagnostic en approbation. PA-02/03 attendent les
remises protégées IA3/AC-02 ; le commanditaire doit désigner évaluateur et équipe
E3. Les preuves Go et les avis internes ne les remplacent pas. Les lots et le
RETEX global restent ouverts.

## Revue GLM et décisions vérifiées par lecture

Appel interne terminé `stop` : 16 825 jetons entrée, 22 302 sortie, 39 127 total
déclarés, 138,412 secondes. Aucun plafond client ou effort imposé. Identifiant
API demandé/déclaré `glm-5.2`, sans attestation du backend. Aucune exécution par
GLM ni acceptation indépendante ; ses originaux et sources reçues sont conservés.

Décision utile intégrée : renommer `action_content_changed` en
`canonical_content_changed`, car le contexte fait partie des octets comparés.
Renommage du code Rust, tests et schéma, avant toute livraison ; aucun alias
historique runtime à préserver. Les documents des preuves antérieures restent
historiques. La version d’enveloppe utilise désormais la constante commune.
Les tests ajoutent le champ inconnu privé (JSON syntaxiquement valide), les deux
positions `-`, un argument excédentaire et le code ENOSPC sûr. Toujours non exécutés.

Deux assertions de GLM sont réfutées par les producteurs réels : les chemins de
fixture, résolus depuis le fichier ou `CARGO_MANIFEST_DIR`, pointent tous deux
sur le même original ; nix 0.30.1 `close<Fd: IntoRawFd>` consomme le `File` avant
`libc::close`, sans double drop propriétaire. Le projet ne gagne aucune preuve
compilateur par cette lecture. `git check-ref-format refs/heads/@` retourne 0
avec Git 2.51.0 : l’affirmation interdisant cette branche n’est pas intégrée.

L’alerte sur les causes serde est examinée dans `safe_causes` et
`native_error_links` : seuls catégorie JSON/labels sûrs sont publiés, aucun message
serde privé. Le test du champ inconnu protège ce consommateur lors de sa future
exécution ; la lecture n’est pas une preuve runtime. La sortie compacte PAEA
historique est conservée : le nouveau rapport ne contient que métadonnées/champs
ASCII ; le helper public n’ajouterait aucune propriété d’autorité. Le comportement
sans `--json` et les options explicitement vides suivent l’existant, sans politique
nouvelle. Seul le profil Linux est prévu pour cette qualification ; non-Unix
n’est pas revendiqué. Aucun échec ne devient conformité globale.
