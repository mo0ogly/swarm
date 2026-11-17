# Témoin de transport de processus — portée limitée

`trace_process_witness.run_witness(out, admission, strace, python)` observe trois
processus Python factices connus : sortie 0, sortie 7 explicite, reprise 0 dans
un autre sous-dossier. Il réutilise `native_build_outcome.execute`. Son reçu
numérique décrit le traceur lancé ; le PID, l'exec/argv/environnement complets
en hexadécimal et le terminal de la trace sont comparés au marqueur du processus
primaire. Une erreur attendue 7 n'est admissible que seule, sans échec de fermeture,
hash, publication ou post-admission. Toute autre cause demeure bloquante et liée.

Le caller doit fournir un processus privé avec umask077, un leaf neuf et vide
mode0700 sur stockage qualifié, les chemins absolus des outils et leurs pins,
les sources/helpers stables, les FD retenus et les gardes hôte/sorties de 20 Gio.
L'admission est contrôlée avant/après les enfants et avant publication. Le module
ne choisit ni ne monte de volume, n'enregistre pas de cible, ne configure aucun
service et ne prétend pas protéger contre un écrivain hostile du même UID.

L'environnement factice est explicitement `LC_ALL=C`, `PYTHONUTF8=1`,
`PYTHONCOERCECLOCALE=0` ; aucune variable du parent n'est transmise. L'allocation
`strace -s` vient du maximum des longueurs d'octets des seuls argv/environnements
connus de ce témoin sans descendants. Elle ne qualifie pas des arguments de build
inconnus. `-xx` permet une comparaison exacte des octets ; une syntaxe inattendue
refuse. Les fichiers doivent être mode0600. La publication exclusive peut laisser
un fichier partiel ou complet visible en erreur ; les causes de cleanup sont
conservées, sans réécriture ni tentative supplémentaire.

Le résultat porte toujours `complete_build_qualification=false` et
`behavioral_admission=false`. Il ne qualifie ni lectures de fichiers, ni compilation,
ni signaux, forks, défauts du traceur, corruption de capture ou fautes de publication
et cleanup. Aucun Cargo, provider, service Swarm ou corps de test Rust n'est appelé.
Le caller conserve les traces, reçus natifs et empreintes des sources/outils ;
un retour numérique seul ne suffit pas à qualifier le témoin.

La première étape de `recipes/trace-process-transport-inert.json` exécute le
contre-exemple synthétique nommé : contrôle0, contrôle7 propre, exit7+EIO refusé,
reprise0. Les appels enfants sont interdits par mocks. Définir explicitement
`SWARM_TRACE_TEST_PARENT` vers le parent temporaire admis avant cette recette ;
son absence refuse, sans fallback. Elle ne prouve aucun comportement réel de
strace, admission de montage ou résultat d'un build. La recette réelle de trois
processus doit être allouée séparément avec les préconditions du caller ci-dessus.

Le second contrôle synthétique de cette recette injecte ENOSPC après un octet
écrit puis EIO après libération du descripteur. Il exige les deux errno dans
leur groupe, le fichier partiel conservé, une publication fraîche réussie et
le refus d'écraser l'original. Cela ne simule pas une saturation réelle du volume
ni une défaillance de stockage/démontage ; aucun enfant n'est lancé.
