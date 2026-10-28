# PAEA — incrément37 : commandes natives du lifecycle personnel

Reprise36 installé dev142 :43 tests de bibliothèque/6 harnais CLI existants/56 UI,
revue18 fichiers et193 liens. Enrôlement personnel encore interne. Codex direct,
sans GLM, délégation, commit/push. Autres travaux/index/processus préservés.
PA-02partiel ; humain/service/transport réseau/UID séparé/confinement non établis.

Choix : exposer init/enroll/recover dans la CLI native avec entrée secrète binaire,
plutôt qu’un premier endpoint HTTP avant protection des rôles. Les chemins du
bootstrap et de clé sont des entrées explicites de démarrage opérateur, pas des
champs d’une requête de service. La CLI conserve le même propriétaire36.

Cible : identity personal-init|personal-enroll|personal-recover PIN SEED_PATH
--input BOOTSTRAP [--json] [--lang fr|en]. Aucun RootStore ouvert. enroll reçoit
une frame versionnée sur stdin : en-tête public,32 octets de preuve d’installation,
puis mot de passe exact non vide jusqu’à EOF, sans normalisation. Aucun secret
dans argv/environnement/JSON/logs. Buffer détenu Zeroizing alloué une seule fois,
sans réallocation intermédiaire contenant le secret. Copies runtime/OS non prouvées
effacées. Réutiliser COMMAND_INPUT_MAX_BYTES et check_command_input_size :16 Mio,
limite de transport CLI existante, pas une politique de mot de passe ou d’exécution.
Aucun watchdog/retry/valeur KDF ajouté. Le propriétaire de stdin garde son FD.

| Étape | Critère observable |
| --- | --- |
| 37a | Reproduire l’acceptation d’options interdites restées aux defaults, puis garde commune fermée pour les trois familles identity |
| 37b | Init/enroll/recover sur le vrai lifecycle ; lecture privée protégée et frame sans copies sérialisées |
| 37c | Rapport de phases/publication sans digest de credential ; erreurs primaire/close/output/flush toutes conservées, succès connu ne devient pas absence d’effet |
| 37d | Témoin valide avant défaut, vrais processus froids, preuve erronée, frame invalide/trop grande/tronquée, secret unique, reprise, sortie perdue et diagnostic expurgé FR/EN |
| 37e | Recette permanente ciblée, Rustdoc/docs FR/EN, revue octets politiques/fonctions/erreurs, profil normal installé et interface Version |

Avant intégration : lire parse/Arguments, producteurs des erreurs et leurs sources,
matrice custom IO/causes secondaires, stdout/flush, fixtures et toutes les options.
Les commandes ne provisionnent pas un bootstrap signé initial ni un vrai humain :
le futur installateur protégé et la livraison indépendante de la preuve restent
nécessaires. Enregistrer aussi le parcours Admin des paramètres KDF/provisionnement
avant clôture globale, sans changer les records historiques. Rotation/récupération
33, service/UX, OS/confinement et E2/E3/PA-13 restent ouverts.

Méthode vérifiée36 à conserver : sélectionner le profil par le SHA du processus
actif, observer séparément le dernier candidat préparé ; persister avant assert,
attendre le prérequis puis bloquer toute étape dépendante après refus ; résoudre
les exécutables installés avant appel. Les notes locales reflètent la capacité
courante consolidée, les anciens octets restent dans les preuves.


37a green-options02 :1 test nommé PASS, après correction de l’attente du harnais
sur les diagnostics texte du parseur. Red et premiergreen en échec conservés.
Garde locale dans parse, sans changement Arguments/helper privé. Recette pérenne
13 tests/3 étapes créée avant livraison.37binit/enroll/recover encore à réaliser.


Édition des variantes appliquée depuis les limites réelles de la fonction détenue :
9 variantes, assertion texte explicite. Instructions produit inchangées, couverture
étendue ; options-recipe02 rejoué inchangé ne prouve aucune nouvelle variante.
Recette03 fraîche requise. Architecture distingue réponsev1 inchangée/options
modifiées.37b non implémenté.


## Livraison partielle37a — garde d’options,37b ouvert

Défaut reproduit sur dev142 après témoin valide ; garde commune corrigée dans
parse, sans modifier Arguments ni le helper privé. Erreur de harnais sur JSON
corrigée vers le contrat texte ; neuf variantes sont désormais exercées.
13 tests/3 étapes PASS sur les premières entrées. Dernière recette03 :7 tests
PASS puis refus de stabilité de sources (service.rs, i18n-en.js, notes). Rejeu
actif145 :7 PASS puis même refus sur six fichiers du chantier API. Ces recettes
restent FAIL et ne sont pas rebaptisées PASS. Recette13 actuelle globale non close.

Qualification complémentaire du comportement livré : sept groupes PASS sur
145figé avec inventaire limité aux exécutables/profil archivés, distinct de la
copie de travail. Six groupes exercent de vrais processus, un garde de factory
est lié à une bibliothèque de fixture. Binaire147 ensuite observé par SHA réel,
profil normal sans fixtures et mêmes quatre entrées propres ; cas supplémentaire
nommé avec les neuf variantes PASS sur ce binaire,56 UI+garde de script manquant
PASS. Sources globales d’autrui encore en évolution ; aucune conformité globale
courante ni acceptation indépendante déduite. Rustdoc privé CLI -D warnings PASS ;
deux avertissements Rustc étrangers conservés, aucun résultat zéro-test compté.
Livraison observée `v0.1.1-dev.147`, PID2822662,
SHA256 `981222cdd5fd51515d192b7dcf865070f7af3aa432c0ef698afb8e71a64fd454`, commit `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`, dirty signalé.
Relecture visuelle Version FR état/EN sombre à terminer avant bilan visuel37a.

Méthode : l’édition finale avait supposé la forme conservée par rustfmt et s’était
arrêtée avant écriture ; le lanceur enchaînait pourtant. Garde functions.exec
réellement testée sur fixture :0 autorise l’étape suivante,23 la bloque, y compris
via wrapper Python propageant le code de son enfant. Ne pas sérialiser argv en
JSON manuel dans la commande shell ; listes Python ou JSON.stringify + quoting
approprié. Les preuves historiques, refus et répétition non utile sont conservés.

Mesure : tests figés1,73s, préparation/archivage61,27s pour un driver228,8Mio.
Amélioration à conserver, propriétaire outil de vérification : réutiliser les
artefacts immuables déjà retenus par SHA sans les recompresser ; critère observable
mêmes garanties de rétention/contrôle avec temps d’acquisition séparé réduit.
Pas de changement de l’outil commun ici : ce sous-chantier n’est pas nécessaire
pour poursuivre37b et exige son propre contrat/régression. Reprise lors d’un
prochain rejeu réutilisant le même artefact lourd. Problème global13qualification
à revisiter après stabilité des entrées communes ; aucune borne ajoutée.

37binit/enroll/recover, frame secrète et erreurs de publication/sortie restent
à implémenter. Ce bilan ne clôt pas37, PA-02,0/17, E2/E3 ou PA-13. Aucun commit/push.


37blecteur de frame écrit, pas encore de commande publique : en-tête versionné,
preuve brute32, mot de passe exact non vide. Un buffer pré-réservé effacé reprend
la borne transport16Mio ; seule la lecture EINTR continue, aucun retry métier.
4 cas unitaires définis ; validation/raccord consommateur/erreurs/rapport à faire.
Captures Version147 FR état/EN sombre relues, note de garde exacte. Cette édition
n’installe aucune nouvelle commande CLI.


Frame4 cas unitaires PASS après correction du producteur étranger vers Int64 ;
compilation initiale sans scénario conservée.37bcommandes/rapport/parcours des
erreurs raccordés en sources, pas encore validés/installés. Init/recover ne lisent
pas stdin ; enroll emprunte la frame, le vrai lifecycle ferme après les deux
issues, rapport connu conservé après échec encode/write/flush. Recover courant
confirmé ne déduit pas un nouvel apply. Régressions unitaires/processus requises.


37blecteur4+commandesunit3 PASS ;5 scénarios natifs PASS puis gardePTY1 PASS.
Compte réel de fixture conservé après stdout ENOSPC, recover confirme3 sans doublon ;
preuve erronée conserve le bootstrap. Producteurfixture corrigé : métier1/personnel7,
helper par défaut1 préservé. Avertissement new inutilisé dans le consommateurCLI
partagé : méthode utilisée par les testsAuthority originaux, pas ce consommateur
paramétré ; aucune suppression de warning produit. Schéma/format/contratFR-ENécrits ;
recette complète/profil normal/activation réelle encore requis.


37b : recette pérenne32tests/7étapes PASS, sources déclarées stables, intégrité VERIFIED ; Rustdoc privé CLI avec -D warnings PASS. Diagrammes FR/EN rendus et relus. Build normal154 préparé avec535entrées effectives archivées et sans fixtures. Rejeu natif du candidat, activation et UI restent requis. Incident de méthode : copie ELF envoyée au helper texte, TypeError avant publication ; helper binaire atomique utilisé après diagnostic, profil déjà retenu réutilisé seulement après comparaison exacte.


## Qualification et livraison37b

32 tests nommés/7 étapes PASS, sources déclarées stables et intégrité VERIFIED :
7 parcours processus natifs,3 commandes/erreurs unitaires,4 frame,7 ancien Control,
5 helper privé,1 bootstrap incomplet,5 consommateur personnel original. Les13 cas
de37a sont donc réexercés avec les sources actuelles stables ; leurs anciens refus
de stabilité restent FAIL historiques, sans empêcher cette nouvelle preuve bornée.
Rustdoc CLI privé -D warnings PASS. Deux avertissements engine étrangers et le
constructeur fixture new inutilisé dans le consommateur paramétré restent visibles ;
le consommateur Authority original exerce bien ce constructeur par défaut.

La première compilation frame avait0 scénario (producteur étranger Kind::Int,
corrigé Int64). Les cinq premiers tests natifs échouaient dans le setup signé,
avant CLI : génération métier7 incompatible avec base0. Fixture corrigée au vrai
contrat métier1/personnel7 ; validation production inchangée. Tous originaux gardés.
Le rejeu des7 scénarios sur le candidat normal154 passe avec les vrais processus,
schéma et PTY. Même SHA que le serveur activé : preuve réutilisée sans relancer
les tests inchangés. Initialisation, compte unique, reprise, mauvais secret/clé,
frame invalide, EISDIR, dépassement transport, terminal et ENOSPC après effet
réel sont observés dans des racines jetables. Aucun humain ni Root vivant utilisé.

Installation supportée observée : `v0.1.1-dev.154`, PID2995461,
SHA256 `341dcdfb6e4294c62b5186e89d9771dd5bdd69bef1c59389b221ef36f73e9dc9`, commit `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`,
modifications non commitées affichées. Profil normal sans fixtures,
535 entrées effectives archivées/vérifiées.56 contrôles HTTP/browser protégés
et garde du script manquant PASS ; Version FR état/EN sombre relue avec les notes
actuelles. Diagrammes FR/EN rendus et relus. La revue locale des octets couvre
politiques, fonctions et erreurs ; ce contexte n'est pas un évaluateur indépendant.

Amélioration de méthode : helper texte refusait les octets ELF avant publication ;
diagnostic puis copie binaire temporaire/fsync/SHA/rename, profil déjà acquis
comparé avant réemploi. Les preuves, l'état préparé et le processus actif restent
distincts. Aucun timeout ou retry métier ajouté, aucune source d'autrui nettoyée.

Suite38 : provisionnement initial protégé, encore prérequis manuel des commandes :
production des clés/preuve/journaux/déclaration épinglée avec reprise sans adoption
silencieuse, puis transport et UX d'enrôlement. Politique KDF explicite à raccorder
aux réglages ; aucune politique opérateur cachée nouvelle. Rotation/récupération
du mot de passe, UID séparé/confinement, deux domaines d'effet, E2/E3 et PA-13
restent ouverts ; PA-02 partiel,0/17 invariants globaux. Aucun commit/push.
