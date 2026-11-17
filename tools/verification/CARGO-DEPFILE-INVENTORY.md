# Inventaire des dépendances Cargo

Périmètre courant : observation après compilation uniquement. Aucun droit de compilation, de test ou d’admission.


### Périmètre
`cargo_depfile_inventory.py` est un outil d'inventaire post-build en lecture
seule, de périmètre `postbuild-depfile-inventory-only`. Il lit un fichier
stdout JSON de Cargo capturé, exige un `build-finished` réussi et un
ensemble d'artefacts non vide, résout le fichier dep-info de chaque artefact
(rlib, proc-macro `.so`, exécutable de test, sibling à inode du
custom-build), analyse les règles Rust répétées cohérentes et leurs entrées phony selon la
convention shlex établie ; les commentaires restent des métadonnées brutes non interprétées, hache et classe les sources nommées par rapport
aux racines déclarées par l'appelant, puis publie un rapport JSON frais.
Bibliothèque standard uniquement ; aucune exécution de commande, aucune
compilation, aucune analyse ELF, aucun réseau, aucune exécution de tests.

### Commande (placeholders)
```
python3 tools/verification/cargo_depfile_inventory.py \
  --cargo-stdout <stdout-json-de-cargo-capturé> \
  --workspace <racine-du-workspace> \
  --target <répertoire-cible-cargo> \
  --registry-root <racine-des-sources-registry> \
  --output <chemin-rapport-frais.json>
```
Codes de sortie : `0` tous les artefacts associés et toutes les sources
observées proprement ; `1` rapport publié avec artefacts non résolus et/ou
problèmes de sources ; `2` refus fatal/IO : une nouvelle sortie partielle ou complète peut rester visible, sans rollback déduit.

### Responsabilités de l'appelant
L'appelant possède le montage, la réservation et l'admission des
descripteurs, choisit tous les chemins (aucun défaut, aucune lecture
d'environnement), gèle les arbres source et cible pendant l'exécution et
fournit le stdout Cargo capturé, considéré comme fiable. L'outil ne
réimplémente pas cette orchestration de stockage/contexte et suppose
l'absence d'écrivain concurrent ; il ne protège pas contre un remplacement
hostile par le même UID.

### Sémantique du rapport
`status` vaut toujours `OBSERVED_POSTBUILD_ONLY`, `scope` toujours
`postbuild-depfile-inventory-only` ; `complete_build_qualification` et
`behavioral_admission` valent toujours `false`, même si chaque artefact est
associé. Drapeaux explicitement non prouvés : octets d'entrée au moment de
la lecture par le compilateur ; environnement exact de configuration
compilateur/éditeur de liens/C/ancêtres ; entrées secondaires des
générateurs/proc-macros omises des depfiles ; fermeture complète d'édition
de liens et d'exécution. L'outil n'émet jamais de record d'admission et ne
prétend jamais que toutes les dépendances sont prouvées. `--registry-root`
n'est qu'une déclaration de classification, jamais une preuve de provenance.

### Refus
Fatals (exit 2, publication complète non garantie après IO) : stdout malformé ou non UTF-8, `build-finished`
absent ou échoué, ensemble d'artefacts vide, répertoires manquants, chemin de
sortie existant, échec IO/fsync/fermeture (causes typées regroupées). Les refus par artefact sont consignés avec
une cause sûre (artefact ou depfile manquant/ambigu/mauvais type/mauvais
inode/symlink/hors cible, syntaxe depfile non prise en charge, sources
manquantes, dérive ou observations conflictuelles) et rendent la sortie non
nulle tout en préservant les observations partielles et les octets/hash lus avant refus syntaxique. Sortie privée (0600), exclusive, fichier et répertoire synchronisés. Aucun repli par contenu
ou mtime ; aucune relecture, relance, temporisation ni chemin alternatif.
