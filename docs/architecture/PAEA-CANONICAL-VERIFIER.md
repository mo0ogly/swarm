# Vérifier la correspondance d’un export canonique PAEA

[English](../en/PAEA-CANONICAL-VERIFIER.md) · [Contrat](PAEA-CONTRACT.md)

L’outil maintenu `tools/paea/verify_canonical.py` compare un document d’origine
`{proposal, context}` au résultat hors ligne de `protection canonicalize`.
Il constitue une première brique locale PA-01/PA-09 ; il ne met pas en œuvre la
commande réservée `protection verify` ni le vérificateur de preuves d’effet.

```sh
python3 tools/paea/verify_canonical.py --document document.json --result export.json
```

Les deux fichiers doivent être réguliers. L’outil utilise le schéma versionné du
dépôt et nécessite un environnement POSIX, Python 3 et `jsonschema >= 4`. Il n’ouvre ni Root, service, compte,
secret, réseau ou politique. Il lit les fichiers en mémoire, sans plafond produit,
retry ni watchdog global. Les liens symboliques vers un fichier régulier sont
admis ; ils ne prouvent aucune identité ou propriété du fichier. Les imports des
helpers locaux ne créent pas de bytecode. Aucune entrée n’est modifiée.

## Vérifications et résultat

1. Lire les octets réellement consommés sur des descripteurs détenus ; refuser les
   répertoires, FIFO et périphériques, sans attente sur une FIFO.
2. Refuser doublons de noms JSON décodés, représentations flottantes/exponentielles,
   constantes non JSON, UTF-8/Unicode invalides et contenu après le document.
3. Vérifier la forme du document avec le JSON Schema v1. Les limites techniques
   de profondeur/entier du Python utilisé peuvent également entraîner un refus.
4. Contrôler le profil v1 du schéma : identifiant attendu, références locales
   seulement, aucune base imbriquée permettant une résolution externe ; une
   corruption du schéma reçoit un diagnostic distinct des entrées utilisateur.
5. Recalculer les octets canoniques, le digest d’action et l’intention métier avec
   l’oracle Python existant. Les champs du contexte sont exigés indépendamment du schéma ;
   aucune valeur par défaut de fixture ne peut compléter un contexte incomplet.
6. Exiger exactement les champs et valeurs de l’enveloppe hors ligne, y compris
   `authority=not-established` et `scope=offline-data-only`. Un booléen ne remplace
   pas la version entière. Le texte canonique doit correspondre octet pour octet.

Exit0 produit un JSON comprenant `status=canonical-correspondence-verified`,
`validation=schema-and-canonical-correspondence-only`, les SHA256 des deux fichiers
consommés et du schéma, son identifiant, le digest d’action et l’intention. `authority` et `effect` restent
`not-established`. Les champs de l’enveloppe exportée peuvent être réordonnés et
espacés sans changer sa valeur ; ses propres octets consommés sont néanmoins liés
par `result_sha256`. Le `request_id` reste exclu du contenu métier et les labels
restent un ensemble trié, conformément au contrat.

**Un couple document/export forgé mais cohérent peut réussir.** Le schéma seul
n’applique pas toutes les règles sémantiques d’admission Rust : montant i64,
références Git, relations entre sources, existence des ressources et politique
courante restent hors de ce contrôle. La recette conserve explicitement un couple
cohérent avec un montant hors i64 pour empêcher de confondre correspondance et
admission. L’outil ne prouve ni authenticité, autorisation, signature, effet réel,
absence d’omission, indépendance ou niveau E2/E3. L’oracle Python partagé avec les
vecteurs reste une dépendance commune déclarée.

## Refus, sortie et reprise

Exit2 conserve un diagnostic JSON statique sur stderr, sans document ni chemin :

| Code | Cause et reprise |
|---|---|
| `canonical-json-duplicate-key` | Nom dupliqué, même après échappement Unicode ; fournir un original sans ambiguïté. |
| `canonical-json-number-invalid` | Flottant, exposant, NaN ou Infinity ; utiliser les représentations exactes du contrat. |
| `canonical-json-invalid` | Syntaxe, Unicode ou limite technique du parseur ; corriger l’original. |
| `canonical-document-shape-invalid` | Forme inconnue/incomplète ; comparer au schéma versionné. |
| `canonical-correspondence-mismatch` | Export substitué, incomplet ou incompatible ; retrouver l’original correspondant. |
| `canonical-regular-file-required` | Type de fichier non admis ; fournir une copie régulière autorisée. |
| `canonical-io-unavailable` | Échec lecture/fermeture/écriture ; traiter la destination/source puis reprendre. |
| `canonical-verifier-schema-invalid` | Schéma local invalide ; réparer le candidat, sans accepter les entrées. |

Les arguments manquants suivent le diagnostic d’usage argparse. Le résultat est
écrit directement sur le descripteur stdout standard, en traitant les écritures
partielles ; zéro progression ou erreur donne un échec. Il n’y a pas de reflush
bufferisé du rapport à la sortie Python. Un échec peut avoir publié des octets
partiels ; seul exit0 avec un JSON complet permet de consommer le rapport. Une
panne stderr supplémentaire conserve exit2 sans inventer un diagnostic publié.
Aucune sortie n’est une preuve d’absence d’effet dans un autre système.

## Recette maintenue

`tools/verification/recipes/paea-canonical-verifier.json` requiert les mêmes
variables explicites d’identité du binaire et de transcript neuf que la
[recette publique](PAEA-CONTRACT.md#recette-publique-sans-compilation).
Elle utilise un binaire déjà construit et le vérificateur Python, dans des racines
isolées. Neuf tests nommés couvrent les 14 exports attendus, altérations, ambiguïtés,
équivalences et limites déclarées, lectures refusées, `/dev/full` réel et écritures
partielles simulées, schémas locaux corrompus/affaiblis et absence de cache
bytecode lors de la génération des vecteurs. Le digest du schéma décrit les octets consommés sans les authentifier ; modifier
le schéma modifie le candidat. Une exhaustion mémoire ou un arrêt externe peut
interrompre le processus hors de ses diagnostics normaux. Les originaux des refus et les reprises restent conservés.
Ni compilation, installation ou modèle n’est appelé par la recette.

[État, revue et preuves de l’incrément 04](../plans/actions-confiance/paea/INCREMENT-04.md).
