# PAEA32 — coordinateur personnel intent/checkpoint durable

## Reprise, choix et dépendances

30dev128 installé, codec exact sans IO.31 :43tests/5étapes et Rustdoc PASS,
build normal dev129 en cours ; aucune source effective31modifiée pendant ce build.
Preuves31/limites antérieures conservées. Besoin PA-02 : ne jamais consommer le
bootstrap puis perdre la proposition/le checkpoint indépendant au crash.

Choix : coordinateur distinct au-dessus de Source et Documents, plutôt qu'ajouter
FS/protocole de reprise dans la machine signée Source. Préserver responsabilité,
bytes exacts, pins protégés et erreurs de publication ; ne pas dupliquer IO.
Source ne devient ni premier appelant ni transport authentifié. Codex propriétaire
technique, indépendant humain/bootstrap/E2/E3 restent requis.

## Plan et critères observables (non réalisés)

32a : contexte local explicite annulation→deux contextes actuels (journal et fichiers),
sans minuterie/default. Checkpoint fermé/canonique lié à Expected+clé+projet déjà
sélectionnés indépendamment. Head positif valide, bootstrap ou compte+pin exact,
intent exact optionnel seulement avant commit du compte. JSON ambigu, changement
d'installation/principal/génération/clé/projet/phase/head refuse ; aucun pin appris
ni tête reconstruite depuis la DB. État manquant refuse.

32b : initialisation explicitement autorisée exige head bootstrap indépendant et
observation réelle du bootstrap actif/contexte ; crée le checkpoint seulement en
absence, sans écraser/réinitialiser de fichier existant. Ce raccord reste interne :
provisioning protégé de clé/head/proof et remise humaine hors agent non fournis.

32c : enrollment prend réservation de fichiers, lit/valide checkpoint, prépare
avec preuve initiale réelle puis persiste intent exact+fsync AVANT Source.enroll.
Au retour du journal, confirme checkpoint compte+pin exact par écriture atomique.
Après crash/erreur, reprise explicite restaure SEULEMENT l'intent protégé exact,
rejoue CAS/lot sans nouveau sel/signature, puis confirme. Ni secret initial ni
mot de passe ne sont demandés pour reconstruire une proposition perdue ; si
l'intent protégé manque/corrompt, refus et récupération indépendante requise.
Réutiliser les témoins actuels sans apprentissage automatique de head.

32d : erreurs suivent séparément journal non tenté/UNKNOWN/acquitté et phase
physique checkpoint ; préserver acquittement de compte après échec checkpoint/
unlock. Une erreur n'affirme ni compte absent ni effet annulé. Causes primaires,
cleanup et branches après effet sont conservées sous diagnostics sûrs. Exact
replay ne duplique ni bootstrap ni compte. Faults avant/après chaque phase sur
fixtures seulement ; témoin valide avant chaque injection, aucun défaut live.

32e : authentification lit le checkpoint protégé sous FS, refuse pending, libère
et vérifie finish AVANT Source.with_authenticated. Le journal vérifie head+pin
actuels et reste tenu jusqu'au callback/fin d'autorité. Aucun FS tenu pendant
effet métier, aucun compte/grant/session inféré. Préserver intégralement AfterUse
succès/échec si l'autorité échoue après effet. Révocation/conflit intercalé refuse.
Enrollment peut tenir son FS dédié avant journal mais n'exécute aucun callback
externe ; tous verrous sont immédiats, pas attente circulaire/retry implicite.

32f : reproduction API absente puis recettes persistence/réouverture, crash avant
journal/après journal/avant checkpoint/après checkpoint, rollback isolé/altération,
contention et annulation/reprise, autorité retenue/callback outcome, mauvais head
et pins, refus sans mutation. Régressions affectées Source/Store/Coordinator,
Rustdoc, revue politiques/fonctions/erreurs liée aux octets, docs FR/EN, build et
installation normale+Version FR/EN. Ne pas confondre cette bibliothèque et un
service démarré. Suite : raccord provisioning/transport protégé/Admin/UX puis
confinement et sinks PA-03 ; PA-13RETEX global seulement après besoin initial.


## Progression d’implémentation réelle

31installé/qualifié dev129.32API absente reproduite ; première compilation refusée
pour Record owned/borrowed (0scénario), contrat producteur et table SQL réels corrigés.
5premiers scénarios PASS. Ajouts : contexte canonique/rollback, writer/révocation
réels et matrice d’arrêt de sous-processus ; recette31tests/5étapes à qualifier.


### Reprise de qualification

recipe-01FAIL :7/8nouveaux scénarios passent, dont3paires contrôle/arrêt réel.
Le harnais attendait busy au lieu de la catégorie sqlite conservée ; assertion
sur is_busy typé, release/join avant assertion. sync-red :1test échoue car le
callback est atteint sans fsync frais du checkpoint. Authentification corrigée :
fsync du témoin validé puis release vérifié avant effet, refus Before et résultats
AfterUse succès/échec conservés. Recette désormais32tests/5étapes,9coordinateur.
Premier edit refusé sur forme de formatter avant écriture ; frontières réelles
relues puis correction, aucun check dépendant lancé sur le refus.


### Bibliothèque vérifiée, installation locale encore ouverte

recipe-02 :32tests nommés/5étapes PASS, intégrité VERIFIED, Rustdoc et diagrammes
FR/EN rendus PASS (PNG relus visuellement).34tests config31liés aux7sources réelles
config inchangées ;9control réexécutés. Normal130premier refus avant compilation :
cache regex verrouillé absent,4archives récupérées et vérifiées, lock inchangé.
Deuxième Cargo/profil499sources/candidat isolé PASS ; lanceur refuse l’inventaire
large modifié : nouveau root_store/preparation_capture.rs du chantier parallèle,
aucun delta des sources effectives. Originaux complets conservés. L’autre chat
corrige activement les doublons/null JSON imbriqués du budget ; pas de retry inchangé.
Le serveur géré dev129 reste installé ; aucune installation32annoncée. Poursuivre
composition authentifiée33 indépendante ; reprendre livraison supportée sur sources
courantes cohérentes/stables. Revue locale liée aux octets, pas acceptation indépendante.


### Preuve additive de livraison courante34

Sources internes actuelles incluses dans dev136 qualifié/installé ;59tests de
bibliothèque, profil515sources sans fixture,6tests du harnais CLI et56assertions
UI+garde de refus PASS. [Détails34](INCREMENT-34.md). Refus/preuves historiques
conservés ; aucun service personnel/humain/confinement/acceptation indépendante.
