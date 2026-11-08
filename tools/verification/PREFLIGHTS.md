# Précontrôles locaux des preuves et de Git

[English](PREFLIGHTS.en.md) · [Recettes](README.md)

Ces deux CLI d'outillage lisent leurs entrées et rendent un rapport JSON. Elles
n'accordent aucun droit et ne changent ni le moteur, ni une mission, ni un service.
Elles ne sont pas raccordées automatiquement à une session ou à un lancement.
Un PASS décrit l'observation effectuée ; il n'autorise aucune mutation ultérieure.

## Archive : vérifier les vrais membres

`archive_preflight.py` lit une archive **tar non compressée**, entière ou
fractionnée. Python 3.13+ et les primitives Unix O_NOFOLLOW/O_NONBLOCK sont requis.
Fournir chaque part dans son ordre exact avec `--part`, sans glob dont l'ordre
serait supposé. L'outil ne reconstitue aucun gros fichier et n'extrait rien.

```sh
python3 tools/verification/archive_preflight.py --part /CHEMIN/archive.tar --manifest /CHEMIN/attentes.json
python3 tools/verification/archive_preflight.py --part /CHEMIN/archive.part000 --part /CHEMIN/archive.part001 --manifest /CHEMIN/attentes.json
```

Le manifeste version 1 a exactement les clés `schema_version` (entier 1) et
`members` (objet non vide, nom POSIX relatif canonique → SHA-256 en 64 caractères
hexadécimaux minuscules). Les attentes viennent des originaux/inventaires que
l'opérateur souhaite rapprocher ; un index de noms seul ne fournit pas de hash.
Les doublons JSON, chemins absolus/ambigus/traversants et hashes invalides refusent.

Chaque membre demandé doit être un fichier régulier unique et avoir l'empreinte
attendue. Un répertoire, lien symbolique ou lien dur ne tient pas lieu d'original.
Le scan continue après un premier membre trouvé pour détecter les doublons tardifs.
Le contrat tar retenu exige deux blocs nuls terminaux puis un padding nul : une
fin prématurée ou un en-tête ultérieur corrompu ne devient pas une fin valide.
Le tampon d'un Mio est une allocation technique, pas une limite de volume/durée.

Les parts sont ouvertes en lecture seule avec des descripteurs conservés ; alias
vers le même inode, symlink final, fichier non régulier ou entrée inaccessible
refusent. Identité, taille et temps de modification des descripteurs **et chemins**
sont rapprochés après lecture. Cela détecte certaines modifications, sans exclure
un écrivain concurrent ni confiner les répertoires parents. Ne pas changer les
archives pendant ce contrôle. Les erreurs de lecture/format refusent avec cause
sûre ; le contenu des membres n'est pas imprimé.

Sortie : `schema_version`, `status`, `members` (taille/hash attendu/hash observé),
`errors` et portée du contrôle. Codes principaux : `manifest_invalid`,
`archive_source`, `archive_invalid`, `archive_changed`, `member_missing`,
`member_non_regular`, `member_duplicate`, `member_digest`. Les échecs avant rapport
complet peuvent ne conserver aucun membre validé : aucune réussite partielle déduite.
Code 0 = PASS, 1 = refus/erreur contrôlée, 2 = erreur de syntaxe argparse.

Limites : pas de gzip/zip, récupération automatique, extraction, signature d'auteur,
hash de tout le flux ou validation des octets des fichiers non demandés. L'outil
peut parcourir longtemps un gros inventaire ; aucun watchdog, retry ou plafond
arbitraire n'est ajouté. Pour reprendre, corriger la provenance/les parts/attentes
sur preuves, préserver le refus puis lancer un nouveau contrôle, sans écraser
l'archive ni les anciennes preuves.

## Git : établir la base avant de l'annoncer

```sh
python3 tools/verification/git_preflight.py --root /CHEMIN/DEPOT --expected-branch main --tracking-ref refs/remotes/origin/main --source-ref COMMIT_SOURCE
```

Les quatre arguments sont explicites. La racine physique doit correspondre au
worktree choisi. Le commit source peut être fourni par SHA même si sa branche a
été supprimée. L'outil résout HEAD, branche, suivi et source puis calcule ascendance
et avance/retard. Il refuse branche incorrecte/détachée, référence absente, source
non incluse, retard ou divergence. Une avance locale est rapportée et acceptée si
la source est incluse ; elle ne prouve pas une publication. Le suivi est la
**référence locale observée**, sans fetch ni connaissance de la fraîcheur distante.

L'état dirty et les chemins (renommages et noms avec sauts de ligne compris) sont
signalés. Le hash du diff porte les données suivies HEAD→index/worktree ; celui du
status porte le status, **pas les contenus non suivis**. Aucun contenu dirty n'est
sauvegardé. `checkpoint_required_before_mutation=true` rappelle qu'un autre
parcours devra préserver le travail avant sa mutation. Un worktree dirty n'est
pas un échec si la base demandée est bien observée ; aucun checkout n'est effectué.

Le lecteur désactive les verrous Git optionnels et fsmonitor, évite les diff/textconv
externes et retire les variables GIT_* héritées qui pourraient rediriger la lecture.
Il respecte la configuration locale lue par Git ; ce n'est pas un sandbox contre
un dépôt/configuration malveillant. Il compare deux observations locales et refuse
si elles diffèrent, sans verrouiller tous les écrivains ni les contenus non suivis.

Le JSON donne les OID effectivement lus, la relation `equal/ahead/behind/diverged`,
`source_in_head`, dirty et ses limites. Codes : `git_command`, `git_io`,
`git_root_mismatch`, `head_missing`, `tracking_missing`, `source_missing`,
`branch_mismatch`, `tracking_behind`, `tracking_diverged`, `source_not_in_head`,
`git_observation_changed`, `git_status_format`, `git_count_format`. Le code/exit et
le type de cause sont publiés, jamais le stderr privé de Git ; le code du processus
ne devient pas un succès vide. Codes de sortie identiques à la CLI archive.
Corriger la décision de base ou la précondition manquante avant une nouvelle
observation ; l'outil n'effectue jamais switch, merge, reset, commit, fetch ou push.

## Vérification maintenue et portée

`test_preflights.py` exerce les deux CLI sur fixtures isolées, conservées sous
l'aire configurée avec sorties brutes. Les commits/branches de ces fixtures sont
fictifs ; aucune référence du dépôt de travail n'est mutée. Les témoins réussissent
avant fautes/refus et reprises. Le changement de seconde observation Git est une
injection déterministe de test, pas une preuve d'exclusion d'écrivains concurrents.
La recette `recipes/evidence-git-preflights.json` conserve les entrées et sorties.

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/evidence-git-preflights.json --output /CHEMIN/NEUF
python3 tools/verification/recipe.py verify --evidence /CHEMIN/NEUF
```

Ces tests et leurs résultats ne prouvent ni une approbation humaine authentifiée,
ni l'immunité d'un modèle à l'injection, ni une couverture universelle des outils.
Ils complètent la méthode de preuve ; la régression Trust R1 reste une preuve
distincte sur un registre simulé. Évolution issue de QW3/QW4 dans le
[plan Peter](../../docs/plans/AGENT-CONTROL-POCS.md).
