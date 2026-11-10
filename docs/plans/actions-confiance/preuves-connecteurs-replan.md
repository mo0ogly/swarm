# T6 — preuves durables, simulation et connecteurs

Reprise après la tranche locale T5, 16 octobre Paris. Propriétaire Codex,
développement direct, sans délégation ni reprise de la mission native suspendue.
Les besoins initiaux R16–R19 restent ouverts ; aucun compte tiers, déploiement,
paiement réel ou nouveau droit système n’est autorisé.

## État réellement inspecté

- `trustExport` produit `TrustProofExport` schéma2, chaîne et originaux exacts ;
  les nouvelles preuves T5 y portent le contexte d’identité authentifiée.
- `trustAnchor` produit une ancre schéma1 par HMAC-SHA256 de séquence/empreinte.
  `verifyTrustProof` exige la clé symétrique32octets, vérifie originaux, ordre,
  empreintes, complétude sous ancre et MAC. Ce contrat historique est fonctionnel
  et reste conservé. Il ne fournit pas de vérification par clé publique.
- `trustProofKey` lit un fichier régulier privé, sans suivre de lien, depuis
  `trustPaths`. Le journal ne possède pas de registre/version de clés métier.
  La recherche de producteurs dédiés de rotation dans le domaine Trust n’en a
  pas trouvé ; les clés OIDC/client privées sont un autre contrat.
- `EvidenceRetentionDays` est validé, persisté, exposé et éditable ; l’inventaire
  des lecteurs Go ne montre pas de consommateur de purge/archivage Trust. Ne pas
  annoncer une rétention effectivement appliquée sur cette seule configuration.
- `commit` engage directement le `TrustLedgerEntry` simulé après contrôle.
  Le catalogue/SPI multiconnecteur R18 ne découle pas de ce chemin unique.
  Le helper fixe d’attestation protège son canal sans devenir connecteur réseau.

La recherche ne prouve pas que tous les autres composants du moteur sont absents.
Lire leurs producteurs/callers et les contrats R16–R19 avant de choisir une
extraction ; conserver les reçus d’idempotence, originaux et contraintes existants.

## Prochaine tranche bornée — clé et ancrage

Comparer : prolonger le HMAC actuel ; introduire signatures publiques versionnées
avec registre de clés ; connecter directement un coffre tiers. Retenir comme
candidat local la deuxième option, à préciser avant code et après vérification de
sources primaires. Aucun coffre tiers ni stockage secret dans l’état métier.
Les anciens exports/ancrages/octets et leur vérificateur restent inchangés.

Définir d’abord : propriétaire et double contrôle de rotation/révocation, matière
privée protégée et cycle de reprise, clé publique/empreinte acquise séparément,
version exacte, révision et activation, chaîne des transitions, ancre indépendante
et traitement d’une clé indisponible. Les permissions de fichier seules ne prouvent
pas l’isolement face au même UID ; la portée R14 actuelle doit rester visible.
Une perte mémoire doit fermer la nouvelle signature, pas réactiver une ancienne
clé ou fabriquer une preuve. La vérification historique et la confiance après
révocation doivent être deux décisions explicites, pas un résultat silencieux.

Critères isolés avant implémentation : export ancien toujours vérifiable ; clé
nouvelle/publique correcte ; mauvaise clé/transition/rejeu refusés ; rotation puis
redémarrage ; révocation bloque la nouvelle signature ; ancien ancrage acquis
séparément détecte troncature/réordre/altération ; aucune matière privée dans
preuves/originaux/export/arguments/environnement ; refus sans mutation ; mêmes
valeurs persistées/API/CLI/Admin, unités et zéro explicites. Aucun réseau sous
verrou métier ni nouvelle politique cachée. Source d’ancre de fixture distincte
explicite, jamais présentée comme attestation externe de production.

Puis traiter archivage/rétention sous ancre sans effacement non autorisé, laboratoire
isolé versionné R17, second connecteur sensible simulé via SPI partagé R18 et matrice
d’applicabilité R19 avec preuves/propriétaire/écarts. Vérifier chaque effet dans son
parcours public. T7 et recette/RETEX suivent leurs dépendances réelles ; une étude
ou une simulation ne démontre ni collecte eBPF ni conformité globale.

## Contrat retenu — première tranche R16

APEX direct, portée signature publique et cycle de clés éphémères. RFC8032 et la
bibliothèque Go1.24.3 `crypto/ed25519` sont les sources primaires ; aucune nouvelle
dépendance. Documents consultés : https://www.rfc-editor.org/rfc/rfc8032 et
https://pkg.go.dev/crypto/ed25519 . L’URL de documentation Go versionnée demandée
au navigateur a échoué ; `go doc crypto/ed25519` installé confirme l’interface.

Namespace `evidence`, API `/api/v1/trust-evidence` : `show`, `policy-preview`,
`policy-apply`, `key-prepare`, `key-activate`, `key-revoke`, `anchor`, `verify`.
La configuration `evidence_signing:{enabled,key_ttl_seconds}` est persistée,
strictement typée, éditable Admin ; défaut inactif et86400secondes, entier positif,
0refusé, borne technique time.Duration exposée. Omission conserve. La configuration
partagée et sa révision restent la source unique ; les mises à jour invalident
sessions/mandats comme les autres politiques.

Préparation seulement sur serveur persistant : clé générée en mémoire protégée
non dumpable, registre ne conserve que public/empreinte/proposant/dates/révision.
Un candidat vivant à la fois est un invariant du flux, pas une limite de capacité.
Un second admin I3 distinct active la version exacte ; rotation retire l’ancienne
clé. Révocation I3 immédiate bloque toute nouvelle signature ; le double contrôle
s’applique à l’activation, le retrait d’urgence reste unilatéral et attribué.
Cette décision évite de maintenir une clé compromise en attente d’accord.
Rejeu exact sans nouvel effet ; après perte mémoire, état public conservé mais
signature indisponible, nouvelle préparation/activation nécessaires. Expiration
vient de la politique ; pas de clé privée sur disque ou exportée.

L’ancre publique versionnée signe un domaine fixe, clé/registre/configuration,
séquence/empreinte finale et digest canonique de l’export v2 ; le lecteur fournit
séparément une clé publique de confiance et son état acquis indépendamment.
Clé retirée permet la vérification historique dans sa période ; clé révoquée
refuse l’acceptation. Un ancien pin hors ligne n’apprend pas seul une révocation :
ceci reste explicite, aucune prétention de révocation distante instantanée.
L’export n’auto-désigne jamais sa propre racine de confiance. Vérification publique
possible en CLI sans racine ni clé privée. Les anciens contrats HMAC restent
inchangés et étiquetés séparément. Aucune purge/rétention ni conformité complète
n’est livrée par cette première tranche.

Premier contrôle sous race : cycle de clés, rotation, reprise, confidentialité et
politiques passent ; le pin contradictoire actif + revoked_at est accepté dans le
vérificateur. Original `evidence-first-race.out` conservé. Cause : seul le libellé
state était contrôlé, pas la cohérence des dates/acteurs. Correction : refuser toute
date de révocation, activation incohérente, retrait absent ou contradictoire dans
le pin, sans importer de confiance depuis l’export. Garde nommée conservée et à
rejouer avec expiration/annulation. Les autres refus de corruption ont passé.

La série `evidence-fixed-race` passe21 cas nommés, y compris le pin contradictoire,
la durée/annulation et l’absence de privé dans les sorties. Les neuf contrôles CLI
publics et32 contrôles navigateur propres passent sur le premier binaire.
L’inspection visuelle repère un export ouvert de8174px : garder le détail fermé et
borner sa zone défilante. Afficher aussi la disponibilité actuelle de matière,
distincte du seul état public de clé.

Non-régression publique pilote échouée : `evidence-pilot-public.out`, attente de
`trust-config-error`. Le sélecteur historique global `[name=reason]` remplit le
premier champ homonyme, maintenant dans le nouveau dialogue. Le test ne soumet
plus le motif requis du formulaire ancien. Les gestionnaires applicatifs utilisent
leurs formulaires propriétaires ; aucune panne du contrôle d’authentification
n’est établie. Correction prévue au composant nouveau : préfixer ses noms de
champs pour préserver les consommateurs et ses frontières. Assertions anciennes
inchangées et scénario identique à rejouer. Attendre la fin des groupes Go avant
modification des actifs embarqués, pour préserver leurs entrées compilées.

Contrôle de jonction CLI : le lecteur de recette utilisait `id` pour la réponse de
publication, dont le producteur renvoie `definition_id`. Original
`evidence-public-current.out` conservé ; lecteur corrigé après lecture du producteur,
sans changement du moteur. La même recette vérifie maintenant trois preuves T5 et
leur contexte d’identité dans l’export signé/vérifié.

Révocation sous politique désactivée reproduite par CLI réel :
`evidence-disabled-revoke-before` observe key-revoke code4 au lieu du succès attendu.
La garde générique de feature bloquait aussi le retrait de sécurité. Correction
au contrat : seuls préparation/activation/signature exigent enabled ; révocation
reste I3 authentifié, clé/digest/révision exacts et trace complète. Une seule
condition modifiée, original source et diff/hash dans
`evidence-disabled-revoke-source-change.json`, nouvelle garde Go nommée. Aucun
assouplissement d’activation ni autorité supplémentaire. Recette CLI à rejouer.

Les groupes Go126T2/23T5/60T3/117T4 ont passé avant cette garde supplémentaire.
Leurs branches ne consomment pas le gate key-revoke désactivé ; le changement
exact est conservé pour réutiliser leurs résultats sans prétendre à un nouveau
passage complet. Tous les cas R16 sont rejoués sur le garde final. Les ajustements
UI ne changent pas ces sources Go ; les recettes pilote et Preuves sont rejouées
sur le binaire final, avec les anciens originaux conservés.

Capture anglaise finale : état « Révoquée » encore en français dans le select.
Le mapping d’état appelait le catalogue, mais l’entrée féminine manquait. Original
`evidence-browser-final/evidence-result-en.png` conservé. Catalogue complété,
garde visible FR/EN ajoutée ; select utilise explicitement les jetons de champ
existants, sans nouvelle couleur. Vérification navigateur à refaire sur ce binaire.

Mise à jour de l’aperçu : le nouveau serveur est lancé et l’ancien identifié puis
arrêté, mais le script de vérification manque urllib.request et termine code1.
Source originale et erreur observée conservées ; aucun nouveau lancement en
reprise. Identification exacte argv/démarrage/empreinte, reprise HTTP seule et
reçu écrit avant cette vérification. Les deux demandes/un effet et les octets
métier sont inchangés ; aucun rechargement IAB affirmé. Leçon : enregistrer
l’identité du processus dès son lancement, puis actualiser séparément son statut
de vérification ; une erreur de contrôle ne justifie pas un second conducteur.

Contrôle supplémentaire ciblé de concurrence : huit préparations de la même
révision0 partent simultanément, doivent sélectionner une seule clé et sa seule
matière privée. Le contrôle CAS intervient avant remplacement d’un candidat ;
aucun défaut n’est déduit du seul examen d’une fenêtre de publication. Garde
Go ajoutée et exécutée séparément, sans changement de production. Les octets de
l’ancien fichier de tests sont reconstruits par retrait exact de ce seul ajout
et comparés aux empreintes des reçus ; les sources de production et les actifs du
binaire restent inchangés. Pas de reconstruction ni de campagne complète à cause
d’un seul test supplémentaire.

## Reprise — comparaison à un témoin acquis séparément

R16 : la signature seule accepte aussi un ancien export authentique. Comparaison
retenue : témoin opérateur conservé hors racine productrice, plutôt que nouveau
service réseau et nouveau stockage de confiance implicite. Le témoin JSON contient
une ancre et son pin public acquis séparément ; le vérificateur reçoit ce témoin
explicitement, jamais depuis le seul bundle candidat. Aucun nouveau délai ni
politique cachée. Une sauvegarde opérateur ne vaut pas journal OS immuable ni temps
tiers ; le même UID peut modifier ses fichiers. Propriétaire Codex.

Critères : égalité et extension après rotation acceptées ; ancien export signé,
préfixe divergent, témoin malformé/révoqué/signature fausse refusés sans mutation ;
CLI sans racine, API et UX FR/EN ; fonctionnement après réouverture du fichier
témoin séparé. Garder le vérificateur historique sans témoin compatible, avec
statut explicite de comparaison absente. Archivage/rétention restent à construire.

Contre-exemple de capacité conservé : checkpoint-before/out et diagnostic privé
observent verify attendu0/code2 sur champ trusted_checkpoint non pris en charge.
Après implémentation, checkpoint-public-current passe14 contrôles CLI/API :
témoin relu hors racine, égalité, extension après rotation, refus d’un ancien
bundle authentique par CLI/API sans mutation. Le vérificateur historique sans
témoin reste compatible avec statut explicite. Les38 contrôles navigateur passent
(FR/EN, thèmes et cinq largeurs), dont refus de rollback et extension sous témoin ;
images du dialogue inspectées. Les contrôles Go restent à lire depuis leur reçu
final avant toute qualification. Pas de service externe ni d’immutabilité système.

## Archivage non destructif — contrat avant code

Comparer purge automatique et copie scellée protégée avec levée explicite. Retenir
la copie non destructive : aucune autorité d’effacer les originaux historiques
n’est déduite de la feuille de route. `archive-preview/create/show/release` dans
le namespace evidence ; Admin I3 courant, révision, ancre et contenu exacts.
L’archive contient le préfixe signé et ses originaux, sous identifiant de contenu,
avec registre de conservation/reçu attribué. Prévisualisation sans fichier ni état
modifié ; publication sans remplacement, reprise du même fichier exact possible
après publication orpheline. Lecture refuse absence/altération/lien.

`evidence_retention_days` existant devient durée minimale de protection, départ
création de copie ; instant et durée sont persistés. Zéro reste refusé. Changement
ultérieur de politique ne raccourcit pas les copies anciennes. Aucun calcul de
durée par multiplication susceptible de déborder ; comparaison des jours révolus.
Levée I3 explicite après échéance, jamais automatique, ne supprime ni copie ni
original. Conservation au-delà autorisée ; aucune promesse de purge maximale.
Une éventuelle suppression physique reste hors tranche et hors autorité implicite.

Assertions préalables : aperçu sans mutation, copie signée/octet exact, refus de
liaison/altération/absence, reprise après fichier créé mais état non publié,
réouverture, double appel/concurrence, refus avant frontière puis levée à frontière
sans suppression, durée ancienne conservée après réduction Admin, droits courants,
CLI/API/UX FR/EN et refus sans mutation. Propriétaire Codex ; même UID et stockage
local restent limites explicites, témoin indépendant nécessaire à la comparaison.

Inspection visuelle d’archive : le dialogue réutilisé affichait encore « décision
sur la clé » pour une levée de rétention. Original archive-browser-visual/
evidence-archive-hold-fr.png conservé. Cause : titre spécifique dans un composant
de confirmation désormais partagé. Titre de décision de preuve commun FR/EN et
garde visible ajoutés ; aucun changement Go. Rebuild et parcours navigateur sur
ce libellé attendus, résultats Go47 réutilisables aux sources natives inchangées.

Résultats natifs : archive-final-race passe47 cas R16 (signature, témoin, archive),
avec sources Go inchangées et reçu code0. Les9 premiers cas d’archive ont aussi
passé avant ajout des variantes d’entrées invalides ; ne pas les additionner.
La recette publique archive-public-current passe18 contrôles ; navigateur
archive-browser-current puis visual passent42 contrôles. Les campagnes sur le
binaire au titre de dialogue corrigé sont à lire depuis leurs reçus labelled.
Aucun groupe interrompu ni simple sortie de compilation n’est utilisé comme PASS.
Les326 régressions Go du bilan précédent restent historiques à leurs entrées ;
les branches métier sans archive/témoin sont inchangées, sans fabriquer une
exécution globale de373 cas. Le pilote public est rejoué sur le binaire final.

La conservation locale consomme maintenant la durée minimale Admin. Elle ne
fournit ni purge automatique, ni sauvegarde tierce, ni horodatage tiers, ni
protection matérielle contre le même UID. Ces limites restent à la qualification,
sans bloquer artificiellement les travaux indépendants R17–R19.

La nouvelle capture anglaise labelled révèle aussi « Confirmer » resté français,
entrée générique absente du catalogue Trust. Original conservé. Catalogue partagé
complété et assertion du bouton rendu ajoutée, en plus du titre. Le contrôle de
parité de catalogues ne suffisait pas à établir la traduction de chaque contrôle
visible ; inspecter titre, contenu ET actions dans les deux langues. Aucun Go
modifié ; seules les recettes publiques sur le nouveau binaire sont à rejouer.

Binaire final confirmed : les18 contrôles publics et42 contrôles navigateur,
ainsi que les sept scénarios pilote, ont terminé code0 ; titres ET actions FR/EN
sont contrôlés et captures finales inspectées. Build/vet/i18n et12 contextes
terminés ; manifeste et aperçu doivent encore être liés au reçu de cette tranche.
Les campagnes labelled/current restent antérieures, sans mélange des binaires.

## Prochaine tranche R17 — conception, non implémentée

Le contrat R17 exige scénarios versionnés, racines/stockages isolés, entrées,
politiques, résultats et limites persistés, rejet de contamination/production et
rejeu attribué après réouverture. Sources consultées : contrat R17, architecture
« laboratoire », producteur trustExport(simulation:true) et recettes publiques
existantes. tools/actions-confiance/README décrit le validateur des huit lots,
pas un service de laboratoire. Le scan ciblé ne suffit pas à démontrer l’absence
de tout composant ; poursuivre les producteurs/callers avant code.

Comparer rejouer une recette de développeur et fournir un laboratoire produit
à corpus fermé versionné. Candidat : corpus fixe de scénarios de simulation,
aucun chemin, exécutable, URL ou connecteur de production fourni par le demandeur.
Le serveur crée lui-même une nouvelle racine de fixture, n’accepte jamais celle
d’une mission existante ; jetons de fixture seulement en mémoire et états hachés.
Exécution hors verrou métier, révision/autorité courantes contrôlées au retour.
Avant code : définir Admin/API/CLI/Go, corpus/version/digest/graine, politiques
persistées, annulation/reprise et résultat déterministe séparé des horodatages
d’exploitation. Pas de nouveau budget caché, aucun lancement natif ou subagent.

Assertions à préciser : injection/exécution et destination production refusées,
contamination de fixture impossible, scénarios d’escalade/digest/rejeu/effet
inconnu, résultat attribué à la version/graine/politiques exactes, réouverture
et rejeu dans une autre racine propre. Une simulation ne qualifie ni fournisseur
ni collecte Linux. Propriétaire Codex, suite R18/R19 puis T7/recette/RETEX inchangée.

## Contrat R17 retenu avant code

APEX direct Codex : laboratoire produit à corpus fixe, plutôt que simple emballage
de tests développeur. Namespace laboratory, API /api/v1/trust-laboratory, carte
FR/EN sur le pilote. show, policy-preview/apply, run, result, replay. Politique
laboratory:{enabled,timeout_seconds,max_concurrent_runs}, défaut inactif30s/1,
champs positifs explicites même inactif, zéro refusé, omission conserve. Révision
partagée et Admin dédié ; consommation distincte. Réserver une tentative durable
avant lancement ; historique jamais remis à zéro. Propriétaire PID/démarrage
réel enregistrés, reprise interrompue seulement si propriétaire effectivement
disparu. Aucune boucle de retry automatique.

Entrée run : version/digest de corpus exacts, graine uint64 décimale en chaîne,
liste de scénarios connus sans doublon, connecteur fixe local-ledger-simulation-v1,
event_id/motif/révision. Aucun chemin, commande, URL, jeton ou destination externe
accepté. replay référence un résultat et crée une nouvelle tentative liée ;
résultat déterministe séparé de l’heure et du propriétaire d’exploitation.

Le parent crée racine temporaire propre et lance son seul helper _trust_laboratory
par pipes/environnement minimal, fermeture FD,14 gardes Landlock/seccomp existantes
sur thread d’opération verrouillé, puis opérations Trust dans ce périmètre.
Aucun magasin SQLite nécessaire au corpus : Store root de fixture, fichiers
Trust publics habituels. Pas de code utilisateur/goroutine d’extension. Délai
parent ferme le groupe entier ; tout résultat est contrôlé avant publication
sous autorité/révision courantes. Refus/annulation/invalidation ferment la tentative
sans la convertir en succès ; seules les métadonnées de fixture sont rapportées.

Corpus v1 cible : Biba direct/élévation, accord exact, effet unique/rejeu, réponse
perdue/réconciliation, altération d’original sous ancre. Graine, horloge de fixture
et politiques fixes sont dans la version du corpus ; politiques opérateur et
observations sont persistées. Une .swarm préexistante ou racine non conforme
refuse avant écriture, pas de contamination de mission. La portée OS reste le
helper fixe, pas les providers. Assertions Go/API/CLI/UX avant acceptation locale :
entrée illégale sans mutation, reprise/réouverture/rejeu déterministe, budget de
concurrence, annulation/révision/identité, aucune donnée brute de fixture dans
rapport, refus de production/chemins et14 gardes réelles. Aucun appel fournisseur.

R17 premier groupe native :15 PASS bruts (helper vide exclu). Le groupe suivant
laboratory-final-race conserve un FAIL de témoin policy-replay ; les17 autres
cas ont passé. Le CLI réel observe réponses JSON sémantiquement égales et octets
d’état inchangés : laboratory-policy-replay-discrimination.json. Cause du témoin :
hashMust sur une première réponse contenant un struct Go puis sur une map issue
du rejeu JSON compare l’ordre de propriétés, pas le sens JSON. Garde normalisée
des deux réponses, assertion d’octets d’état inchangés conservée ; aucun moteur
assoupli. Le diagnostic ad hoc initial a échoué avant son test sur jeton de fixture
effect trop court ; rejet conforme, fixture corrigée, original dans sortie outil
de00:47:44. Ne pas qualifier cette erreur de fixture comme défaut de rejeu.

Une annonce intermédiaire a attribué ce FAIL à l’arrêt de l’enfant avant lecture
de la ligne nommée ; rectifiée après lecture du reçu. Les cas timeout/annulation
passaient effectivement. Toujours lire le cas FAIL original avant d’en annoncer
la cause. Dernière recette publique laboratory-public-current :8 contrôles CLI/API
passent ; version au catalogue final et garde native corrigée restent à vérifier.
Source primaire Pdeathsig/thread parent Go1.24.3 consultée :
https://pkg.go.dev/syscall@go1.24.3#SysProcAttr ; parent verrouillé sur son thread
pendant toute la durée de l’enfant, pas de mise à niveau de Go.

R17 vérification intermédiaire : laboratory-verified-race passe19 PASS bruts ;
le helper vide n’est pas un cas significatif, donc18 (8 principaux/10 sous-cas).
Parent effectivement tué et enfant arrêté observés ; annulation et racine déjà
occupée refusées. Le test nettoie seulement sa fixture identifiée ; un arrêt
brutal de serveur peut laisser des fichiers de simulation orphelins, pas de
données métier ni d’effets de production. Pas de promesse de nettoyage historique
par scan global. Les8 contrôles publics et32 navigateur passent sur premier
binaire final, captures Admin FR/EN inspectées.

Revue locale finale : exposer les bornes réelles commande/résultat/état partagé
et uint64 dans show ; ne vérifier le PID propriétaire que pour une tentative
encore running, pas les résultats historiques. Préimage/diff conservés dans
laboratory-bounds-change.json et fichier source original. Aucun algorithme de
scénario ni autorité modifiée ; native/public à relier au binaire delivered.

Vérification élargie ciblée justifiée : ajout de champs à TrustConfigChange/
trustState, validation de chargement et routes CLI partagées. Rejouer126 T2 et
47 signature/témoin/archive R16, en plus du laboratoire ; pas de campagne complète
570 identités, dont les producteurs consommateurs pertinents restent inchangés.

Méthode : l’analyse de risques hypothétiques pour le futur connecteur a produit
un intervalle de communication excessif pendant les régressions. Les outils
répondaient brièvement ; ne pas attribuer ce délai au moteur. Revenir au reçu
actuel et à l’action bornée ; analyse interne courte, horloge et point utile entre
blocs. Reporter les choix R18 dans la prochaine tranche après lecture des callers,
sans prolonger le silence par une nouvelle conception abstraite.

## Prochaine tranche R18 — à définir, non implémentée

Les callbacks publics commit/reconcile produisent encore le ledger simulé unique ;
les interfaces effect-intent/result du contrat restent cibles du SPI. Lire ces
producteurs et DTO/validation/UX avant extraction. Le corpus laboratoire ne
constitue pas un second connecteur métier et son confinement ne qualifie pas les
providers existants. Retenir une interface versionnée partagée pour autorité,
contenu exact, admission, unicité/réconciliation, avec deux effets simulés réellement
distincts. Définir le second schéma sensible simulé, capacités déclarées/réelles,
paramètres et jeux de contrats communs avant code. Aucun compte tiers ni effet réel.
Préserver champs/exports/reçus legacy et tester les deux chemins par CLI/API/UX.
Puis R19 applicabilité/preuves/écarts sans certification inventée, T7 et RETEX.

Inspection du rapport mobile : JSON technique trop présent après exécution.
Origine laboratory-browser-delivered/laboratory-result-en.png conservée. Verdict
lisible avec5 scénarios/14 contrôles OS et liste traduite ; JSON replié sans
ouverture automatique. Garde navigateur sur résumé, cinq libellés et détail
fermé ajoutée. Aucun Go changé ; les191 cas natifs restent liés à leurs sources
identiques, nouvelles recettes publiques sur binaire readable.


## Reprise R18 — 16 octobre, contrat retenu avant code

Constat : commit écrit directement le ledger ; reconcile ne contrôle que le reçu.
Deux choix : ajouter une seconde branche directe, ou extraire une frontière commune
versionnée. Retenu : frontière commune et catalogue fermé ; pas de plugin arbitraire.
Codex propriétaire, sources canoniques, recettes en racines isolées.

Second effet : octroi d'accès simulé dans un registre distinct, sans modifier les
identités ou permissions réelles. Paramètres explicites subject/role ; aucune
réutilisation de montant/devise/bénéficiaire pour ces données. Le mandat I3 fixe
connecteur/version/paramètres, la demande et l'accord exact les relisent.
L'absence du sélecteur conserve le règlement et ses digests historiques.
Aucune nouvelle politique d'exécution, durée ou budget ; admission par mandat et
politique de données existants. Catalogue technique fermé, capacités vérifiables.

Séquence : (1) garde publique du catalogue absent ; (2) SPI intent/result v1,
autorité et idempotence partagées, readback natif des deux effets ; (3) recettes
Go/CLI/API/UX, admission exacte, concurrence, unknown/reprise, champs invalides
et absence d'effet réel ; (4) docs FR/EN et bilan lié aux entrées.
Ne pas qualifier les fournisseurs sensibles ou l'acceptation globale.

Garde de méthode R18 : le premier groupe Go (22 cas PASS) a figé ses sources Go,
mais son inventaire ne couvrait pas les fichiers web embarqués en cours d'édition.
Il reste un diagnostic ciblé, pas la qualification du binaire final. Le conducteur
final inclut aussi ces entrées ; aucun fichier Go/web modifié pendant ses contrôles.
Une recherche initiale de web_server.go en racine était erronée ; rg --files a
établi internal/engine/web_server.go, aucun fichier créé pour contourner l'erreur.

Second échec du conducteur avant tests : le scan hérité cherchait les noms Go
dans toutes les entrées, dont des images. Retiré ce scan inutile : les groupes sont
déjà explicitement sélectionnés. Séparation inventaire binaire/lecture de sources ;
originaux qualified2 conservés. Aucun test produit exécuté par ces deux conducteurs
en échec ; ils ne valent pas PASS. La recette publique initiale, distincte, passe
11 contrôles sur le binaire construit.

Périmètre de régression R18 : extraction du commit commun affecte le pilote,
la validation du step-up, l'admission et la délégation ; ces groupes sont rejoués.
Le laboratoire appelle aussi le commit réel en fixture scellée : son groupe natif
est repris pour ce changement précis. Les signatures/témoins/archives contrôlent
la compatibilité des anciens formats. Pas de campagne globale ajoutée sans cause.

Accord de grand montant reproduit dans le nouveau formulaire : refus « accord non
exact » pour 9007199254740993. La copie depuis Number arrondissait le montant.
Correction du sérialiseur uniquement : texte décimal serveur, validation int64
et émission en nombre JSON exact ; contrôle Go inchangé. Le mandat d'accès montre
aussi sujet/rôle plutôt qu'un plafond monétaire. Les 327 cas Go significatifs
(91 principaux,236 sous-cas) ont passé avant ce seul changement web, en quatre
groupes ; helper parent vide exclu. Réutilisation limitée aux branches Go et
entrées pertinentes inchangées ; recette finale exige le nouveau binaire embarqué.

## Résultat R18 vérifié — 16 octobre 2026, 01:38 UTC

SPI local versionné et deux effets distincts livrés : règlement et octroi d'accès
simulés. Intention/résultat v1, autorité et admission communes, accord exact,
clé d'idempotence globale et confirmation de l'unique entrée native. Aucune
permission réelle ou connexion fournisseur créée. Compatibilité legacy contrôlée.

Le reçu connector-verification.json consolide 327 cas Go distincts (91 principaux,
236 sous-cas) en quatre groupes ; un helper parent vide est exclu, pas de total
unique d'exécution annoncé. Les entrées Go sont restées inchangées pendant les
corrections web. Binaire final : 11 contrôles publics connecteurs, sept scénarios
pilote et 56 contrôles navigateur (33 connecteurs/23 pilote), aucun diagnostic
inattendu. Build/vet/i18n/12 contextes/diff et manifeste1455 sources, test nommé
sous race, passent. Auto-revue uniquement.

Les originaux de conducteurs (répertoire puis image lus comme texte) et de
l'arrondi UI/erreur anglaise sont conservés ; ils ne sont pas requalifiés PASS.
La reprise corrige l'inventaire de fichiers, sérialise le texte décimal exact et
contrôle la langue réellement rendue. Aucun contrôle métier assoupli.

Aperçu : deux demandes/un règlement simulé et octets métier inchangés ; reçu
connector-preview-process.json, identité/commande/empreinte vérifiées avant arrêt
de l'ancien processus, nouveau reçu écrit dès spawn. Nouveau formulaire EN390px
inspecté ; rechargement IAB non observé.

### Prochaine tranche R19 — non implémentée

Matrice d'applicabilité contextualisée : référentiel/version et contrôle, portée,
justification, propriétaire, preuve liée à ses entrées, statut et écart/évaluation
externe manquante. Examiner les producteurs actuels avant de créer un validateur
doublon ; vérifier les sources primaires des référentiels retenus. Contexte cible
local public/internal et effets simulés explicite, aucun claim de certification.
Prévoir un contrôle qui refuse une déclaration globale non étayée ; dossiers
source/historique/recette inchangés. Puis T7 eBPF et qualification globale/RETEX.

## Reprise R19 — contrat retenu avant code, 16 octobre

Inventaire : verify.py contrôle les lots, qualify.py sélectionne T2 ; aucun
producteur de matrice d'applicabilité établi dans ces composants. Comparaison :
document libre seul, ou JSON versionné unique avec validation et lecture produit.
Retenu : JSON commun, CLI Python pure sans écriture et carte de lecture ; aucune
nouvelle politique d'exécution, durée, permission ou orchestration.

Contexte explicite local public/internal, effets simulés, sous-ensemble illustratif
de douze contrôles NIST SP800-53 catalogue5.2.0. Source primaire officielle figée
par révision et SHA ; une matrice n'établit pas équivalence, certification ou
conformité organisationnelle. États partiel/non évalué/hors périmètre seulement.
Chaque ligne : référence/version, justification/portée, propriétaire, preuves
relatives et empreintes, entrées source pertinentes, limites et évaluation externe
manquante. Pièce historique conservée distincte d'une recette actuelle.

Avant code : assertions de refus d'une revendication globale, contrôle inconnu,
ligne/propriété vide, preuve absente/altérée, source dérivée périmée, lien/path
sortant, duplicat et changement de contexte. Schéma strict. La carte ne lit
aucune URL fournie par l'utilisateur, ne lance aucun contrôle ni opération métier.
Erreur/chargement/vide, FR/EN, deux thèmes et cinq largeurs contrôlés. Reprise
par correction du document et de ses preuves, jamais par changement d'un reçu
historique. Codex propriétaire des modifications ; revue externe non effectuée.

La page web primaire liée redirige vers CPRT et l'ancien lien GitHub est
indisponible via l'outil web. L'API officielle du dépôt a permis de découvrir
le chemin puis de figer le catalogue ; chemin src testé404 non réutilisé.
Aucune de ces erreurs de recherche ne prouve une panne de Swarm.

R19, première liaison de preuves : le reçu initial du courtier ne correspond plus
à trust_secret_broker.go après introspection. Le reçu ultérieur
introspection-verification.json correspond à ce fichier et ses tests nommés
SecretBroker ont passé dans introspection-lock-t2t4-race.json. Référence remplacée
dans la nouvelle matrice, aucun reçu historique modifié.
Le test reference_bool_schema reproduit true accepté comme version1 : Python bool
est égal à int. Type exact contrôlé comme pour le schéma principal ; original
mapping-guards-first.out conservé, garde ciblée et groupe complet rejoués.

## Résultat R19 vérifié — 16 octobre 2026, 02:01 UTC

Source commune web/trust-controls.json, référence officielle figée et validateur
CLI sans écriture, carte produit à lecture explicite livrés. Douze contrôles
sélectionnés : neuf partiels/deux non évalués/un hors périmètre, cinq reçus
historiques et36 entrées pertinentes. Aucun score, baseline, certification,
conformité globale ou évaluation indépendante déduits.

Dix tests nommés Python,13 contrôles publics et sept scénarios pilote passent.
96 contrôles navigateur (40 matrice/33 connecteurs/23 pilote), aucun diagnostic
inattendu ; réponse503 volontaire attribuée distincte du refus réseau pilote.
Build/vet/i18n/12contextes/diff et manifeste1465 sources/test nommé race passent.
Aucun Go métier modifié ; les groupes R18 restent à leurs entrées Go inchangées,
pas de nouvelle exécution globale annoncée. Reçu controls-verification.json et
consolidateur dans verification-referentiels-20261016. Auto-revue uniquement.

Le contrôle syntaxique du script navigateur a été demandé avant sa fermeture :
Unexpected end of input, aucun scénario exécuté ; script terminé puis syntaxe et
recette réellement passées. La référence True=1 et la preuve courtier périmée
restent leurs originaux, corrigés sans modification de reçu ni contrôle affaibli.

Suite R20 : voir ebpf-replan.md. BTF disponible, mais la création d'une map vide
est refusée EPERM avec CapEff0 ; aucun attachement/événement. Propriétaire Codex,
préparation d'un collecteur borné autorisée ; pas d'élévation Linux implicite.
Puis qualification globale et RETEX, critères non prouvés conservés ouverts.
