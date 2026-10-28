# PAEA — incrément44 : parcours CLI d’installation depuis un artefact privé

Reprise après43 : bibliothèque44tests/5étapes, Rustdoc et revue13fichiers vérifiés,
graphes FR/EN examinés. Livraison43 reste groupée à ce raccord public nécessaire.
Dev165 est la dernière livraison42 vérifiée. Codex seul, sans GLM, délégation,
commit ou push. PA02partiel,0/17/E2/E3/humain/UIDséparé/listener/PA13ouverts.

Commandes retenues, sans nouvelle option/politique : identity service-install,
service-resume, service-reconcile SHA256 --input CHEMIN_PRIVE_ABSOLU [--json]
[--lang fr|en]. Grammaire identity actuelle : seulement input/json/lang uniques,
refus des flags interdits même avec valeur par défaut avant toute IO ; aucune
RootStore, clé ou identité générée. '-' et stdin ne sont pas des entrées ici.
Les trois opérations correspondent aux modes43Create/ResumeStage/RecoverPublished.

Entrée : fichier artefact42, parent existant0700 et fichier0600/singlelink/euid réel,
nofollow/ancêtres/nom-inode retenus via Documents. Vérifier un pin extérieur canonique
avant IO, puis read_private_durable, decode42, finish et close explicites ; aucune
clé rendue si libération/fermeture refuse. Conserver validation/ack de relecture et
causes indépendantes. Aucun plafond ou retry produit ajouté. Pas de lecture du
paquet opérateur40 ni de clé métier/proof ; le récepteur possède seulement l’artefact
limité remis au compte de service. Les verrous techniques d’entrée peuvent être
créés avant un refus de pin ; pas de mutation des octets d’entrée ni de la cible.

Rapport JSON fermé version1 : opération/résultat, stage/intention/journaux/publication,
indicateurs humain/service/confinement faux. Sur succès, données privées de démarrage
attendues (chemins et pin bootstrap dérivé de l’entrée indépendante) pour le seul
opérateur ; jamais seed/password/proof. L’erreur après write/flush conserve le rapport
connu et toute erreur d’opération antérieure ; erreur ne signifie pas absence de
publication. Diagnostic stderr expurgé, branches Error::Joined/fermeture et causes IO
custom conservées par les matrices existantes, pas seulement source().

Tests : unité write/flush/erreur précédente et matrice typée ; générateur Rust de
fixtures protégé limité au test (source persistée) qui fournit artefact et proof
synthétiques hors service, aucun businessseed sur disque ; binaireCLI normal effectue
les vraies installations et lifecycle. Recette permanente Python : froid/init/enroll/
recover, pin/frame/permissions/links, flags avant IO, cible existante, reprise des
stages connus, stdout/dev/full avec effet durable puis réconciliation. Les stages
préparés par hook fixture restent distincts d’un crash du binaire normal. Schéma
Draft202012/FR-EN/Rustdoc/revue liée aux octets puis build/restart supportés, profils
sans fixture, identité réelle, assets protégés/Version et navigateur requis.

Limite : réconciliation43initiale refuse état avancé sans reset ; avant listener,
prévoir preuve durable avant première utilisation et reprise physique distincte de
l’autorité personnelle actuelle. Aucun humain, compte OS, privilège, confinement,
paiement, mission ou service externe autorisé par ce raccord.


Résultat local44 : dev168 activé et identité ELF vérifiée, SHA256
`e5f10f4b9972ab1420327123a2fb22abfebb2f949773464712f6a36aed13da77`.
50tests nommés/6étapes PASS sur entrées stables, intégrité vérifiée ;6tests de
supplément après commentaires Rustdoc PASS,44cas d'autorité inchangés réutilisés.
Rustdoc privé CLI+autorité -Dwarnings PASS.8parcours sur vrai binaire normal sans
fixture PASS (59commandes), plus7anciens parcours personnels et56contrôles navigateur
+ refus de script en erreur PASS. Même ELF candidat/actif, PID rafraîchi avant UI,
ressources protégées, Version/notes FR-EN/commit réel et modifications noncommitées
vérifiés. Les quatre process73 sont le producteur fixture, pas le binaire normal.

Échecs originaux préservés : parent de fixture sous checkout group-writable refusé,
compilation full-debug ENOSPC avant tout test, verify du supplément lancé avant
sa fermeture puis vérifié après résultat final. Cache PAEA seul nettoyé après
inventaire/empreintes, helper et preuves figés conservés ; espace libre passé de2,1à11,3Gio puis
profil de vérification explicite sans symboles/incrémental. Aucun contournement de
chemin, protection, délai ou fixture de production. Aucun commit/push/délégation.
Livraison43/44 technique locale close ; programme PA02/0sur17/E2/E3/PA13 ouvert.
Suite45 : constater capacités Linux réelles et frontière de service/UID avant
exposition protégée ou revendication d'installation personnelle complète.

Requalification courante44 : dev174, SHA256 `e069b9aac7d5d2595b238367519cd822244f26e9972d16e0c32da77f666a1d33`, même ELF réel pour8parcours/59commandes,7régressions et56contrôles UI+garde PASS. Recette bibliothèque fraîche50tests/6étapes PASS et stable sous lock courant ; preuves168 historiques inchangées. Les remplacements concurrents171/173 et refus de build172 restent conservés, sans réemploi entre ELF. Profil normal174 :561entrées archivées, sans feature fixture. Le modèle de recette native inclut désormais environment explicite ; nouvelle exécution8cas PASS. Aucun restart supplémentaire pour cette clôture ponctuelle.

Replan documentation : trois liens antérieurs de tools/verification/README.md vers les result.json Rust104/105/106 ne résolvent plus. Originaux indisponibles, propriétaire migration Rust ; références conservées, aucun reçu recréé. Les liens PAEA ajoutés sont contrôlés séparément ; conformité globale des liens README OUVERTE. Reprise : originaux restaurés/vérifiés ou relocation prouvée par leur propriétaire.
