# Custodie d'entrée finie (CD01-FINITE-STDIN-17)

Assistant autonome de la bibliothèque standard Python pour la mise en
custodie des octets bruts d'un descripteur d'entrée finie loué par
l'appelant, dans un répertoire de sortie privé exclusif. Il est séparé
du wrapper rustc existant et de Cargo : rien ne le câble dans un build,
aucun fichier existant n'est modifié par ce paquet, et les sources du
wrapper et du collecteur restent inchangées.

- Assistant : `tools/verification/finite_input_custody.py`
- Régressions unitaires : `tools/verification/test_finite_input_custody.py`
- Recette : `tools/verification/recipes/finite-input-custody-inert.json`
- Contrat français : `FINITE-INPUT-CUSTODY.md` (ce fichier)
- Contrat anglais : `FINITE-INPUT-CUSTODY.en.md`

## Limites de périmètre

- Aucune exécution de compilateur, de sous-processus ou de service ;
  aucun câblage Cargo.
- Aucune lecture d'environnement, aucune éligibilité par argv, aucune
  hypothèse sur le descripteur 0 ou sur `sys.stdin`.
- Aucun délai imparti, plafond de taille, retry, repli, suppression ni
  nettoyage.
- Aucune reconstruction historique de stdin et aucune qualification de
  build, de profil ou de graphe complet ; rien ici n'admet, n'installe,
  ne fusionne ni n'accepte quoi que ce soit, et aucun verdict PASS
  n'est émis ni sous-entendu.

## Préconditions de l'appelant (déclarées, jamais prouvées)

`capture(input_fd, output_dir, allocation_id)` attend :

- `input_fd` : un descripteur détenu par l'appelant sous un bail de
  fixture finie coopérative à un seul écrivain et un seul lecteur, dont
  le producteur se terminera. Le descripteur est emprunté : il est lu,
  jamais fermé, remplacé ni dupliqué pour le retour.
- `output_dir` : un répertoire exclusif, neuf et privé, détenu par
  l'uid courant sans bit de permission groupe/autres, créé par
  l'appelant ; l'assistant ne crée jamais le parent de la sortie.
  L'appelant détient aussi, en dehors de l'appel, les descripteurs de
  montage et de réserve du volume sous-jacent.
- `allocation_id` : une chaîne de provenance non vide. Ce n'est jamais
  une authentification et cela n'accorde aucune éligibilité.

L'assistant valide les types d'arguments, le répertoire (vrai
répertoire, sans lien symbolique, détenu par l'uid courant, mode
privé), l'absence des quatre noms de custodie et la statabilité du
descripteur d'entrée avant de consommer le moindre octet d'entrée.

## Ce que fait une capture réussie

1. Crée `input.partial` exclusivement (`O_CREAT|O_EXCL`, mode 0600).
2. Lit `input_fd` jusqu'à l'EOF observé ; seule une lecture de longueur
   nulle compte comme EOF. Chaque bloc passe par une boucle d'écriture
   complète qui refuse les écritures sans progrès et boucle sur les
   écritures courtes.
3. fsync du fichier de staging puis fermeture unique de l'écrivain.
4. Hardlink de `input.partial` vers `input.bin` sans écrasement ; une
   collision échoue et laisse l'original intact.
5. Réouverture de `input.bin` en `O_RDONLY` avec vérification de
   l'offset 0.
6. Écriture exclusive de `manifest.partial`, fsync puis fermeture.
7. Publication de `manifest.json` exclusivement par hardlink sans
   écrasement depuis `manifest.partial`. Les octets bruts et le JSON ne
   partagent jamais un fichier.
8. fsync du répertoire de sortie, fermeture, puis seulement le retour
   `(read_only_fd, manifest)` : un descripteur lecture seule détenu
   par l'appelant, positionné à l'offset 0, accompagné du dictionnaire
   de manifeste publié.

SHA-256 et le nombre d'octets décrivent uniquement les octets en
custodie dans `input.bin`. Après un succès, les quatre noms
`input.partial`, `input.bin`, `manifest.partial` et `manifest.json`
existent ; les noms de staging sont conservés, jamais supprimés.

## Validation en lecture seule d'un enregistrement en custodie (validate_staged)

`validate_staged(output_dir, allocation_id)` est un consommateur strict
en lecture seule d'un enregistrement de custodie déjà publié, ajouté
sans aucun changement de `capture`. Il n'écrit rien, ne crée aucun
lien, ne renomme, ne supprime ni ne change aucune permission,
n'exécute rien, ne lit aucune variable d'environnement, ne renvoie
aucun descripteur et ne mute jamais un enregistrement, même en cas
d'échec. Il revérifie les préconditions appelant vérifiables depuis le
système de fichiers : `allocation_id` une chaîne de provenance non
vide (jamais une authentification ni un confinement), `output_dir` un
vrai répertoire existant (lstat, sans lien symbolique), détenu par
l'uid courant, sans bit de permission groupe/autres. Il exige ensuite
exactement les quatre noms de custodie `input.partial`, `input.bin`,
`manifest.partial` et `manifest.json`, et rien d'autre dans le
répertoire ; chacun doit être un fichier régulier (lstat, donc les
liens symboliques sont refusés), détenu par l'uid courant et en mode
privé ; `input.partial`/`input.bin` et
`manifest.partial`/`manifest.json` doivent former actuellement des couples de
hardlinks (même périphérique et même inode), sans preuve de création historique. Seul
FileNotFoundError établit un nom absent ; toute autre erreur, y compris
une erreur de permission, conserve sa cause originale et n'est jamais
traitée comme une absence.

Le validateur lit ensuite `manifest.partial` et `manifest.json`, exige
des octets identiques, analyse le JSON en refusant tout membre
dupliqué à toute profondeur, et valide le schéma exact du producteur :
ensemble complet des membres sans membre manquant ni membre étranger,
noms de staging et de publication et jetons de publication fixes,
`allocation_id` attendu (égalité avec la chaîne de provenance attendue
par l'appelant), `eof_observed` et
`staged_copy_complete_of_observed_stream` exactement le booléen vrai,
`byte_count` un int exact non négatif (un bool est refusé), `sha256`
exactement 64 chiffres hexadécimaux minuscules, et le bloc
`not_claimed` complet. Enfin il recalcule le nombre d'octets et le
SHA-256 de `input.bin` en le lisant exactement une fois, en lecture
seule, et échoue si l'un ou l'autre ne correspond pas au manifeste :
c'est ce recalcul, jamais les seuls noms, étiquettes ou JSON, qui fait
preuve du hachage. En succès il renvoie le dictionnaire de manifeste
validé.

Les échecs lèvent `FiniteInputCustodyError` avec le même message
statique et les mêmes champs d'état typés, aux étapes `validate`,
`read-manifest`, `parse-manifest`, `schema-manifest` ou `verify-input`,
avec les causes originales conservées non affichées ;
`ready_fd_close_attempted` reste toujours faux pour les échecs du
validateur. Une première erreur de fermeture est primaire ; les erreurs de fermeture au nettoyage sont secondaires. Aucun retry, repli, délai
imparti, plafond de taille, lecture d'environnement ni nettoyage ; et
une validation réussie ne prouve ni qu'un récepteur court, consomme
des octets ou réussit, ni aucun fait de compilateur, de lignée ou de
graphe complet, ni aucune entrée historique ; l'intention amont reste
inconnue même à EOF observé.

## Ce que le manifeste affirme, et refuse d'affirmer

Le manifeste enregistre `eof_observed`, `byte_count`, `sha256`, les
noms de staging et de publication, et un bloc `not_claimed` explicite :
l'intégrité de l'intention amont reste inconnue même si l'EOF a été
observé ; la disponibilité du descripteur et le retour de l'API sont
des faits de l'appel, pas du fichier ; la consommation par le
récepteur, l'intention de délégation et le succès du consommateur
restent inconnues. Le manifeste décrit uniquement l'entrée en custodie
complète, jamais le succès du retour d'API, du récepteur ou de la
délégation.

## Changements FIFO vers fichier régulier (déclarés, sans équivalence)

Quand le descripteur loué est un FIFO ou un tube, la mise en custodie
change le calendrier et l'adressabilité. Calendrier : le producteur
doit courir jusqu'à l'EOF avant que la capture ne revienne
(pré-lecture/spoule) ; ensuite les lectures du récepteur ne bloquent
plus sur le rythme du producteur et ne le concurrencent plus.
Adressabilité : la copie en custodie est un fichier régulier adressable
et relisible avec une taille visible, alors qu'un FIFO se consomme en
une passe et n'est pas adressable. Le récepteur consomme le fichier en
custodie, jamais le flux original ; c'est une transformation déclarée,
pas une équivalence.

## Erreurs

Tout échec de validation, de lecture, d'écriture (y compris écriture
sans progrès), de fsync, de fermeture, de hardlink, de réouverture ou
de publication du manifeste lève `FiniteInputCustodyError` : un
message statique avec des champs d'état typés `stage`, `primary` et
`secondary` contenant des jetons bornés op/nom/errno (pas d'octets
d'entrée, pas de valeurs d'environnement). Chaque descripteur est
fermé au plus une fois ; les échecs de fermeture du chemin d'erreur
deviennent des causes secondaires ; un descripteur déjà rouvert sur
`input.bin` peut être refermé par le chemin d'échec
(`ready_fd_close_attempted`). Les fichiers partiels sont conservés ; rien
n'est supprimé ; aucun rollback n'est déduit ; l'entrée n'est jamais
relue après un échec ; aucun fichier de rapport d'échec n'est
garanti. Il n'y a pas de retour en succès après un échec.

## Tests et recette

`python3 -B tools/verification/test_finite_input_custody.py -v` (la
recette utilise le gabarit `{root}`) lance 29 régressions unittest avec
des fixtures fraîches sous le TMPDIR choisi par l'appelant. Chaque test
à faute injectée exécute d'abord une capture témoin normale, puis une
capture de récupération fraîche après la faute. Couverture : le
contrat témoin, les flux vides et non UTF-8, les lectures fragmentées,
les écritures courtes, les écritures sans progrès, les fautes de
lecture/fsync/fermeture/hardlink/réouverture/manifeste, la collision
exclusive avec original intact, l'état d'erreur typé à message
statique, et un vrai `os.pipe` alimenté par un producteur coopératif
en processus et lu par le double de récepteur inerte détenu par le
test, plus le consommateur en lecture seule `validate_staged`
(témoin sain, contenu brut falsifié, mauvaise provenance, membre de
manifeste manquant ou étranger, couple de hardlinks faux ou entrée
supplémentaire, JSON malformé ou dupliqué, EOF et compte typés
incorrects, cause d'EIO conservée). La classe `ValidateStagedTests`
peut aussi être lancée seule (`python3 -B
tools/verification/test_finite_input_custody.py ValidateStagedTests
-v`) sans rejeu historique. Les producteurs et récepteurs des tests
sont des doubles en processus ; un récepteur de production réel
n'existe pas dans ce paquet, et rien ici n'affirme de consommation par
un récepteur. Aucun compilateur ni sous-processus n'est utilisé, sans
rejeu historique ni harnais de délai imparti.

Exécuter la recette ne lance que ces régressions unitaires. Ce n'est
ni une acceptation, ni un verdict PASS, ni une installation, ni une
fusion : aucun rejeu historique, aucun build complet et aucune
qualification d'un wrapper ou d'un collecteur ne sont effectués ni
sous-entendus par ce paquet.


Complément du principal : original_exception et secondary_exceptions conservent les causes originales non affichées, y compris interruption ; ready_fd_close_attempted prouve une tentative, jamais une fermeture après échec. Seul FileNotFoundError établit un nom absent. Fixtures conservées sous TMPDIR. Le tampon de 65536 octets est un découpage technique, pas un plafond.


Portée du consommateur : identité des hardlinks et égalité courante des octets observées sous bail stable, sans preuve de création historique sans écrasement ni exclusion d’un écrivain concurrent de même UID. Lecture seule signifie aucune écriture de contenu/noms ; les métadonnées de lecture telles que atime peuvent changer. Une fermeture en échec est primaire si première erreur, secondaire lors du nettoyage. Les erreurs de nom absent conservent exception et errno originaux. Neuf nouvelles méthodes du validateur exécutées séparément ;20PASS capture inchangés réutilisés.
