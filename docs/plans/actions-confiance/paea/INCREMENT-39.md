# PAEA — incrément39 : lecture privée effaçable

38livré/vérifié156,27 tests+1natif+56UI. Avant publication du paquet opérateur,
Documents::read renvoie un Vec pouvant réallouer, sans effacement promis. Source
réelle store/file_io.rs et consommateur Documents lus. Propriétaire Codex direct.

Options : lire le paquet via read puis envelopper son Vec (trop tard pour ses
réallocations), ou ajouter un parcours distinct dès l'ouverture/avant le premier
octet secret. Choix deuxième ; ancien read non secret inchangé. read_private
retourne PrivateBytes non Clone/non Serialize et Debug expurgé, emprunt as_bytes.
Réservation avant IO, FD privé0600/single-link/NOFOLLOW, taille réelle stat+1
sentinelle et une allocation Zeroizing avant lecture. Pas de plafond opérateur ;
représentation usize/allocation restent des refus typés. Contrôles du nom/inode,
taille/temps de modification et fermeture explicite. Seule lecture EINTR continue,
annulation entre lectures ; pas de timer ni d'attente/retry métier.

Critères : témoin privé exact/vide/binaire avant chaque faute ; refus missing/
nom/modes/liens/types sans mutation ; dossier retenu après renommage ; cancellation,
close réel+erreur ReadClose/reprise. Unitaires : read partiel/EINTR, croissance/
troncature, IO custom, substitution physique avant/après lecture et changements de
métadonnées ; buffer détenu protégé sur chaque issue, aucune preuve d'effacement
des copies empruntées/OS/compilateur. Anciens9 Documents et consommateurs ciblés
inchangés ; recette pérenne, revue trois axes/Rustdoc/FR-EN/build normal/install.

Le lecteur ne vérifie ni schéma, pin/authenticité/fraîcheur ni autorité humaine.
Un propriétaire non coopératif/root n'est pas confiné. Suite40 : publication/reprise
protégées du paquet/installation, puis CLI/UX/réglages ; coût KDF existant explicite,
aucune nouvelle politique. PA-02partiel,0/17/E2/E3/PA13ouverts. Aucun commit/push.


Découverte consommateur : write relisait les anciens octets via Vec avant refus Exists/CAS. Raccord au read_private partagé pour ces contrôles, sans nouvelle politique. Les régressions incluent désormais Documents9, Control16, Coordinator9 et matière5 en plus des12nouveaux cas.


## Qualification composée39 et livraison

51 tests nommés vérifiés sur les mêmes sources :12nouveaux+9anciens Documents
dans les trois premières étapes, puis16Control+9Coordinator+5matière dans une
recette complémentaire fraîche. Le manifeste initial supposait une cible --test
control inexistante : arrêt avant scénarios à cette étape, original FAIL conservé,
intégrité VERIFIED. Commande corrigée depuis la recette existante vers --lib
control::tests::. Les21 positifs inchangés sont réutilisés, pas rebaptisés recette
globale PASS : inventaires identiques sauf manifeste corrigé/nouveau manifeste
dérivé, mêmes overrides d'environnement, premières3étapes identiques, dernières3
identiques à la recette complémentaire30PASS. Contrôles source/manifeste/markers
dans composed-checks.json. Aucun zéro-test compté. Rustdoc privé -D warnings PASS.

Nouveau lecteur : test binaire/vide, refus physiques/annulation/fermeture/reprise,
lectures partielles/EINTR, taille/overflow/allocation, IO custom, remplacement du
nom avant/après lecture, croissance/troncature/mtime et deux causes conservées.
Le contrôle avant create/replace relit aussi par ce buffer ; les anciens scénarios
publication/ENOSPC/CAS/poison et consommateurs actuels restent vérifiés. Dépendance
directe zeroize1.9.0 déjà verrouillée, aucune version modifiée. Le read historique
Vec n'est pas renommé ni présenté comme effaçable. Pas de nouveau plafond/délai.

Livraison supportée observée : `v0.1.1-dev.157`, PID3087568,
SHA256 `07d1b712d59d9865b690f7b482e001a346ae6555c75c45d4d65455f62e4f2954`, commit `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`, dirty affiché.
Profil normal sans fixtures,537entrées effectives archivées/vérifiées.
Sept parcours natifs sur le binaire exact (harnais37figé, source de test inchangée,
helpers Python/schéma courants déclarés) PASS ; le nouveau précontrôle d'écriture
est consommé par les vrais checkpoints du lifecycle CLI. La bibliothèque liée au
harnais ne remplace pas le binaire normal. Même SHA activé : preuve réutilisée.
56contrôles UI protégés+garde script manquant PASS. Version FR état/EN sombre
relue, toutes notes comparées au DOM ; lignes sous pli non toutes capturées.

Diagrammes FR/EN initiaux rendus mais formulation après échec de close pouvant
suggérer un deuxième close : défaut documentaire détecté à la revue visuelle,
originaux conservés. Diagrammes finaux indiquent fermeture unique traitée/causes
conservées, rendus et relus. Aucun changement de code après les contrôles.
Revue locale liée aux octets politiques/fonctions/erreurs ; non indépendante.

Suite40 : paquet opérateur protégé avec pin indépendant, publication initiale et
reprise exacte des fichiers/journaux ; aucune adoption silencieuse/remplacement,
clé métier hors service. Puis CLI/UX/réglages de politique. Le lecteur protège son
allocation et ses descripteurs, pas l'authenticité du paquet, l'humain, le même UID
hostile/root ou les copies OS/compilateur/empruntées. PA-02partiel,0/17globaux,
UID séparé/confinement, rotation/récupération, E2/E3 et PA-13 restent ouverts.
Aucun commit/push ; index/sources/processus d'autrui préservés.
