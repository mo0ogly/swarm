# PAEA — incrément38 : matière initiale de provisionnement

## Besoin et décision

Suite37 livrée154,32 tests/7 étapes,7 natifs et56 UI ; PA-02 reste partiel.
Les commandes réclament encore des journaux/signatures/clés/déclaration créés
manuellement. Propriétaire Codex direct, sans GLM/délégation/commit/push.

Options : écrire tout l'installateur multi-fichiers d'un seul lot, ou séparer
la matière cryptographique immuable de sa publication protégée et de sa reprise.
Choix deuxième : la clé métier reste dans le paquet opérateur privé, seule la
clé personnelle sera livrée au service ; pas de nouvel endpoint de signature.
Cette tranche prépare une requête explicite fermée, deux clés indépendantes et
une preuve aléatoires, deux propositions signées base0, un paquet privé réversible,
et la déclaration v3 liée exactement à leurs heads attendus. Elle ne crée aucun
fichier/journal, droit implicite, compte OS, humain ou service. Le grant métier
est fourni explicitement par l'opérateur ; aucun droit/durée/KDF par défaut.

## Séquence et critères

1. Contrat/tests : requête valide puis invalides avant entropie, codec exact et
   refus des ambiguïtés/troncatures, cause entropie conservée et diagnostics privés.
2. Préparation : stockage secret Zeroizing, clés non clonables/non sérialisées par
   dérivation automatique ; export uniquement par codec privé explicitement nommé.
3. Journaux réels isolés : appliquer ces octets signés via API existante, déclaration
   attendue sans adopter le head d'un journal ; initialiser/enrôler/reprendre avec
   le lifecycle courant et la seule clé personnelle, aucune clé métier au service.
4. Revue octets politiques/fonctions/erreurs, recette permanente, Rustdoc, FR/EN,
   build/install supportés et parcours actuel. Pas d'acceptation indépendante.

Suite39 : publication/reprise du paquet et des fichiers privés, avec intentions,
NOREPLACE/CAS, fermeture/UNKNOWN et refus de substitution ; ensuite CLI/UX/réglages.
Les coûts KDF existants restent explicites et persistés, leur réglage Admin requis
avant authentification exposée. Rotation/récupération, UID séparé/confinement,
PA-03…13,0/17 et E2/E3 restent ouverts.

Incident préalable : espace libre1,22Gio, cache incrémental privé PAEA identifié,
21 598 fichiers hashés avant retrait atomique/nettoyage ; libre6,52Gio. Sources,
preuves, exécutables et cible de build partagée préservés. Chemin supposé
private_files.rs absent : inventaire rg établit store/file_io.rs, consommé lu.


## Qualification locale38 et livraison

27 tests nommés/4 étapes PASS, sources stables, intégrité VERIFIED :5 matière/
journaux/lifecycle,4 entropie/sortie,8 chargeur de clé,10 startup à deux sources.
Les4 injections internes sont précédées d'un vrai témoin valide. Les deux premiers
passages internes échouaient avant ce témoin : clés JSON du Principal supposées,
puis digest personnel70 caractères saisi manuellement. Producteur serde relu,
fixture corrigée/générée ; originaux4FAIL+4FAIL conservés, aucun défaut production
masqué. Première compilation rouge avait aussi3 erreurs de harnais,0 scénario.
La préparation5 initiale PASS et le27 initial PASS précèdent la revue suivante.

Revue autorécursive : la sortie privée perdait la distinction écriture incertaine/
flush après32 octets acquittés. OutputPhase ajoutée avec cause typée ; nouveau27
complet et Rustdoc privé -D warnings PASS sur ces octets. Candidat155 antérieur
préservé, nouvelle construction156 requise/effectuée. Aucun retry, timeout ou coût
KDF implicite ajouté. Buffer de paquet réservé avant copie de secrets, clés Dalek
et proof/temporaire/paquet Zeroizing ; copies de l'appelant/OS/compilateur exclues.

Rejeu complémentaire d'un test nommé avec vrais processus CLI sur le binaire normal :
matière nouvelle, clés réellement aléatoires, événements appliqués aux vrais journaux
jetables, init/enroll/refus du doublon/recover courant3. Marqueur obligatoire vérifie
la branche native ; le test ne peut réussir silencieusement par sa branche API.
La clé métier reste dans l'objet/paquet opérateur ; seule la clé personnelle est
écrite dans la fixture de service. Ce harnais n'est pas un installateur production.

Livraison supportée observée : `v0.1.1-dev.156`, PID3045713,
SHA256 `2c38366282b38f8ac53a6f21aaeaf4a55e4b5dc4d9948fa83bf42d9197613466`, commit `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`,
dirty affiché ; profil normal sans fixtures, 536 entrées effectives archivées
et vérifiées. Même SHA que le candidat testé : preuve native réutilisée.56 contrôles
UI protégés et garde du script manquant PASS, Version FR état/EN sombre relue ;
notes complètes comparées au DOM, lignes sous pli non toutes capturées visuellement.
Diagrammes FR/EN rendus/relus, corrections des descriptions périmées32/33/34/37.
Revue locale liée aux octets, jamais acceptation indépendante.

Le cache incrémental PAEA propre retiré après inventaire/hash avait rendu5,3Gio ;
les builds/recettes suivants passent, preuves originales préservées. Les empreintes
tapées et noms wire supposés sont des défauts de méthode avérés : prochains exemples
produits par le codec réel puis validés, champs consommés relus avant édition.

Suite39 : avant publication du paquet contenant des seeds, ajouter une lecture
privée effaçable sans réallocation contenant un secret au contrat Documents partagé.
Le read actuel renvoie Vec et peut grossir : ne pas le réutiliser pour ces seeds
en affirmant leur effacement. Puis publication/reprise sans remplacement, paquet
opérateur physiquement distinct et clé métier absente du service ; CLI/UX/réglages.
Aucun installateur, service, humain, UID séparé/confinement établi. PA-02partiel,
0/17invariants globaux, E2/E3/PA-13 et rotation/récupération restent ouverts. Aucun
commit/push ; sources/index/processus d'autrui conservés.
