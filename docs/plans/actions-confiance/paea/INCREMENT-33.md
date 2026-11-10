# PAEA33 — lier le compte personnel tenu au contrôle métier

## Reprise et décision (plan, non réalisé)

32 :32tests/5étapes et Rustdoc PASS,3paires témoin/arrêt réel et reprises exactes.
Build normal130 initial refusé avant compilation faute du cache verrouillé regex ;
4archives téléchargées et vérifiées sans changer Cargo.lock, build repris en cours.
31dev129 reste la dernière installation vérifiée. Aucun code33 avant candidat32
stable ; ni endpoint, ni compte humain réel, ni E2/E3 ne sont déduits des bibliothèques.

Le Control existant tient le mandat signé courant pendant Coordinator config et
sa fin d'autorité. Le coordinateur personnel32tient le compte courant pendant
callback et sa fin d'autorité, après fsync/release de son FS. Il manque la composition
qui lie la preuve au principal ET à l'installation de ce contrôle. Une égalité de
nom fournie dans une requête, ou le seul succès préflight27, ne suffit pas.

Choix : wrapper typé qui possède les deux noyaux et consomme une preuve empruntée
non constructible/non sérialisable. Écarter un acteur/booléen libre envoyé par client,
et ne pas reconstruire un permit depuis une réponse antérieure. Préserver les APIs
internes existantes, non exposées comme contournement du service protégé. La frontière
OS et le transport restent requis avant activation publique du profil protégé.

## Tâches et critères

33a : getter de l'installation du record déjà vérifié dans credentials::Proof,
donnée privée sans grant/session. Control ajoute une entrée authentifiée qui exige
principal humain exact ET installation, avant toute permission/stockage ; génération
du compte et génération du mandat restent deux contrats distincts, pas égalité
inventée. Le projet/périmètre/droits viennent ensuite du vrai mandat signé courant.

33b : AuthenticatedControl possède personal::Coordinator et Control, et reçoit
uniquement contexte, secret personnel emprunté, head de mandat attendu et requête
métier fermée existante. Aucun acteur client. Ordre : lire/fsync/release FS personnel,
compte courant tenu→mandat courant tenu→config/witness, effets et fins d'autorité.
Les journaux personnel/métier doivent être distincts ; la déclaration/provisioning
protégés suivants devront vérifier racines lexicales ET objets physiques retenus.
Un refus de concurrence reste immédiat ; aucun délai/retry/valeur opérateur ajouté.

33c : ne pas aplatir les couches d'erreur. Si Control a observé un effet et qu'une
fin d'autorité échoue (métier ou personnelle), conserver Response/StorageErr et
chaque cause. Pas de faux succès vide ni d'affirmation sans effet depuis Err. Tous
consommateurs futurs (service/CLI/UX) devront traiter ces observations explicitement.
Fermer les deux noyaux sans effacer un échec secondaire ou une fermeture partielle.

33d : témoin légitime réel sur fixtures puis secret erroné, autre humain/autre
installation, mauvais projet/périmètre/head, mandat révoqué et compte révoqué,
contention sur chaque couche, annulation après effet succès/échec conservé. Les
preuves restent celles des APIs publiques Rust sur stockage réel isolé, pas un
endpoint/service ni un humain réel. Recette permanente, docstrings, FR/EN, revue
politiques/fonctions/erreurs liée aux octets, profil normal/local install/Version.

Suite : déclaration startup composite protégée et garde de deux domaines ; custody
clé/init bootstrap explicites ; service/CLI puis UX directe, compte OS séparé et
confinement avant exposition protégée. Les choix de transport/politiques utilisent
la source persistée/Admin existante, jamais une constante de commodité. PA-02reste
partiel ; PA-03…13/E2/E3 et invariants globaux non inventés.

## Amélioration retenue — récupération/rotation du secret personnel

Origine : Source v1 immuable ACCOUNT_ID et Expected.generation, revocation terminale
volontaire ;32 ne livre pas de changement/reset de mot de passe. Valeur : rétablir
un accès humain légitime sans ressusciter bootstrap ni effacer l'historique. Périmètre
identité/control, propriétaire Codex puis opérateur humain pour l'approbation hors
bande. État : nécessité identifiée, protocole non décidé/non réalisé. Reprise :
concevoir versionnement/générations et intention durable explicitement autorisée,
secrets hors agent ; témoin ancien secret refusé/nouveau accepté, révocation courante
et reprise sans doublon, ancien bootstrap toujours consommé. Ne pas bricoler un
nouvel ID/compte/journal pour contourner un refus. Traiter avant déclaration globale
PA-02 complète ; ne bloque pas le raccord typé33 autorisé et indépendant.


### Affinement par le contrat réel de preuve

Credential Proof prouve une connaissance ; create/verify publics la produisent
aussi pour des labels choisis par l’appelant. Ne pas exposer une opération Control
qui la traite seule comme autorité de compte courant. Helper choisi privé au crate,
précondition compte courant tenu explicite. Le wrapper public accepte le secret
seul et possède ses noyaux configurés. Custody/racine réelle relève du prochain
startup protégé. debug-red1test reproduit aussi une fuite du contenu Response dans
Debug de Control ; correction Debug expurgée écrite, données structurées conservées,
vérification à faire. Aucune fuite live ni endpoint public déduit.


## Progression d’implémentation réelle

Debug-red1scénario FAIL puis green1PASS, réponse structurée conservée et texte expurgé.
API absente reproduite (0scénario).14tests contrôle/composition PASS, puis1test ciblé
de double autorité tenue PASS : révocations réelles compte+mandat en concurrence
pendant le même effet, réussies après usage, nouvelle opération refusée. Erreurs
imbriquées après effet : reprise exacte de la configuration et1événement d’historique.
Wrapper public avec secret seul, helper privé au crate.7compteurs de recettes
consommatrices9→15actualisés sans toucher les originaux historiques ; nouvelle recette
38tests/5étapes à qualifier.129dernière installation ; livraison32encore ouverte
après modification concurrente de l’inventaire candidat.


### Preuve additionnelle de fermeture des erreurs

Recette38tests recipe-01PASS/intégrité VERIFIED, Rustdoc et diagrammes FR/EN rendus
et relus PASS. Cargo normal réussit, profil refuse les edits effectifs concurrents
web/service et provider-policies ; refus complet conservé, aucun candidat33installé.
Test additionnel réel des deux erreurs de close PASS, branches conservées, réutilisation
refusée et seconde fermeture explicite idempotente réussie. Compteurs de contrôle16
actualisés ; nouvelle recette39tests/5étapes à qualifier. Original38gardé historique.


### Preuve additive de livraison courante34

Sources internes actuelles incluses dans dev136 qualifié/installé ;59tests de
bibliothèque, profil515sources sans fixture,6tests du harnais CLI et56assertions
UI+garde de refus PASS. [Détails34](INCREMENT-34.md). Refus/preuves historiques
conservés ; aucun service personnel/humain/confinement/acceptation indépendante.
