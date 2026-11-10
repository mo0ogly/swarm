# R01/R13 — contexte d’identité commun

Reprise du 15 octobre, propriétaire Codex, développement direct autorisé.
Le pilote et les tranches d’authentification restent conservés. Les consommateurs
T4/T5/T6 sont des jointures de la cible, pas des services déjà intégrés.

Observation : `findTrustIdentityAt` rend une identité locale. La session humaine
conserve issuer/sub/sid, celle du service seulement sujet/révision/expiration.
Les preuves attribuent acteur/intégrité sans contexte d’authentification commun.
Le contrat `identity-context-v1` n’est donc pas entièrement produit. Le mapping
explicite des comptes existe ; le mapping de groupes reste une tranche suivante.

Options : enrichir chaque réponse indépendamment, ou résoudre un contexte commun
au point d’authentification et le réutiliser dans les opérations et preuves.
Retenir la seconde : une seule validation, aucun rôle importé automatiquement,
et compatibilité explicite des preuves historiques. Pas de nouvelle politique.

Contrat de cette tranche : commande `identity-context` CLI/API authentifiée,
lecture seule, JSON vide ou `mandate_id` optionnel ; aucune identité fournie par
le client. Dix champs de la jointure, plus `schema_version` et révision locale.
`principal_id` désigne l’identité locale ; `subject` le sujet externe vérifié,
`issuer` l’émetteur vérifié ou `urn:swarm:local` pour le jeton local.
`principal_kind` vaut local/human/service. `assurance` décrit la méthode et les
claims ACR/auth_time disponibles sans leur attribuer un niveau inventé.
`authenticated_at` désigne la validation réussie de la connexion locale opaque ;
null pour une session historique sans métadonnée. Les jetons locaux sont vérifiés
à l’instant de résolution, avec expiration null. `attributes` reprend rôles et
intégrité locaux. `delegation_id` est null sans mandat, sinon mandat actif lié au
principal ; aucune élévation. `token_digest` est l’empreinte de la session locale,
jamais un jeton fournisseur. `expires_at` vient de la session réellement vérifiée.

Nouvelles preuves : contexte établi sous le verrou métier, inclus dans leur
empreinte et visible à l’export. Anciennes preuves sans champ inchangées.
Les rejeux conservent leur résultat et leur attribution historiques.

Assertions avant code : lecture authentifiée sans mutation ; absence/mauvais
Bearer refusé ; local/humain/service distingués ; reprise après réouverture ;
suspension/révision/expiration/logout refusés ; métadonnées legacy null explicites ;
mandat d’un autre sujet refusé ; client acteur/claims refusé ; aucune donnée brute.
Nouvelle preuve liée au contexte, altération détectée, export historique valide.
Recette CLI/API/navigateur : connexion → contexte affiché → demande attribuée →
export vérifié → révocation → refus, FR/EN et thèmes/mobile existants.

Références primaires : OpenID Connect Core §2/§5.7 et RFC9068 §2.2. Elles
distinguent sub, issuer, iat et auth_time ; le temps de création d’une session
locale ne doit pas être présenté comme auth_time du fournisseur.
R13 reste ouvert pour le mapping de groupes et les limites de révocation service.


Précisions de compatibilité : le digest d’autorité des rejeux conserve sujet,
rôles et intégrité ; une nouvelle session du même acteur peut relire son ancien
résultat, sans réécrire son contexte historique. L’empreinte des nouvelles preuves
inclut le contexte optionnel, celle des anciennes preuves omet ce champ.
Le contexte d’un approbateur ne revendique pas le mandat du demandeur comme sa
propre délégation. L’API de lecture n’accepte un mandat que s’il est actif et
appartient au principal authentifié.

Une session historique sans issuer/sujet externe ne les invente pas à partir
d’une configuration actuelle. Les attributs de contrôle viennent toujours de
l’identité locale active et de la révision courante. Le nouveau champ ne crée
aucun droit et aucune durée ; ses bornes viennent de la session existante.


Vérification en cours : une recherche de diagnostic a échoué parce qu’un motif
commençant par `---` était interprété comme option de rg. Correction de la
commande avec `--` avant le motif ; aucun test ni état produit affecté. Cette
erreur d’outil reste distincte d’une panne du moteur.

## Tranche suivante — mapping de groupes (conception, pas implémentation)

Conserver le mapping explicite sujet externe → identité locale comme condition
préalable. Les groupes signés pourront sélectionner seulement un sous-ensemble
des rôles que l’opérateur a déjà autorisés à cette identité. Comparer règles de
création automatique d’identités et restriction d’identités existantes ; retenir
la restriction, qui ne crée ni utilisateur ni plafond d’intégrité depuis un claim.
La configuration versionnée désigne le claim, les groupes et les rôles locaux.
La révision de configuration invalide les sessions établies avant sa modification.

Définir avant code le comportement d’un groupe absent/inconnu, d’un claim scalaire,
null ou malformé, du cumul de groupes, d’un rôle hors plafond et d’une configuration
inactive. Ne pas stocker tous les groupes bruts : conserver les décisions locales
et leur révision. Refuser une connexion qui n’obtient aucun rôle, plutôt que rendre
silencieusement une session sans effet. Tester la connexion renforcée avec les
rôles effectivement dérivés ; un accord ne doit pas utiliser les rôles d’une
identité locale plus large que ceux de sa session. Admin/API/CLI/Go et preuves
emploient la même décision. Cette conception ne qualifie pas la tranche.

### Échec lié de vérification — fixture historique

`identity-context-all-race.out` conserve le refus de
`TestActionsConfianceT3IdentityContextLegacyAndExpiry` : la fixture persistait
OIDC enabled avec issuer/client/redirect manquants. La lecture authoritative
refuse correctement cet état. Le défaut est dans notre test, pas une session
historique prouvée défaillante. Remplacer sa configuration artificielle par la
configuration complète de l’émetteur isolé existant, puis conserver seulement
la session sans nouvelles métadonnées. Ne pas assouplir la validation moteur.
Corriger après la fin de la série active ; reprendre ce contrôle nommé sous race.
L’annonce antérieure de gardes toutes passées était prématurée : le critère
reste ouvert jusqu’à la vérification de la fixture corrigée.

### Série interrompue — preuve partielle conservée

La série commune finit avec code143, sans PASS final, après plusieurs minutes.
Les processus Go/compilateur ne sont plus présents lors de l’inspection ; aucune
cause de SIGTERM n’est établie. Ne pas l’attribuer à une panne produit ou à une
limite précise sans preuve. L’original reste `identity-context-all-race.out` ;
aucune moyenne verte ne clôt ce contrôle.

Précondition modifiée : fixture legacy corrigée sans changement moteur, puis
contrôles nommés plus courts : contexte seul, T2, puis autres T3. Un conducteur
local de vérification identifié, détaché du canal de sortie, conserve commande,
empreintes, démarrage/fin et code dans un reçu pour chaque groupe. Pas de
mission native, de privilège, de remise à zéro ou de nouvelle limite produit.
Les groupes se terminent au premier échec ; leur observation reste bornée.

Leçon de supervision : l’annonce de réussite d’un ensemble exige son reçu final
(code0, noms réellement exécutés, entrées inchangées), pas les derniers PASS du
journal. Le conducteur de vérification impose cette condition et arrête la chaîne
sur échec. Les contrôles partiels restent visibles mais ne qualifient pas le lot.
Cette règle existait déjà dans le contrat commun ; l’ajout opérationnel est le
reçu de fin vérifié, pas une nouvelle promesse documentaire.


La sonde CLI d’expiration a d’abord échoué à construire sa fixture : une seule
identité au lieu des trois exigées. Code2 conservé dans la sortie d’outil ; ce
refus de configuration ne prouve rien sur l’expiration. Inspection du validateur
puis ajout des identités séparées avant nouvelle sonde, sans changer le moteur.

### Expiration après attente du verrou — défaut reproduit

`identity-context-clock-before.json` : CLI bloqué en `locks_lock_inode_wait`,
session expirée avant libération, code0 et contexte rendu. Fixture de session
synthétique isolée explicitement étiquetée ; aucune connexion fournisseur n’est
revendiquée par cette sonde. Le temps était capturé avant le verrou dans CLI/API.
Les opérations métier partagent cette capture ; leur frontière est concernée.

Comparer expiration vérifiée seulement avant l’attente et temps courant choisi
sous verrou. Retenir le second pour toutes les entrées publiques concernées.
Le temps explicite des tests déterministes internes reste conservé ; un temps
zéro interne demande le temps réel sous verrou, jamais un paramètre JSON client.
Aucune durée ni politique nouvelle. Propriétaire Codex. Ne pas qualifier la
tranche avant refus observé après attente, sans mutation, puis régressions publiques.

La recherche de l’aide a aussi nommé un `main.go` racine absent ; le point
d’entrée réel est `internal/engine/main.go`. Cette erreur de chemin n’affecte
pas le binaire ni les tests ; l’aide sera modifiée à son emplacement constaté.


Consolidation : le sous-cas de date future nomme son instant dans le journal.
Les trois groupes communs369 et29 ciblés finaux ont373 noms bruts dans leur
union, dont deux noms pour cette même variante temporelle. Aucun total unique
de campagne n’est annoncé. L’assertion initiale de total372 dans le consolidateur
a échoué ; les reçus natifs sont inchangés. Le rapport expose les groupes réels
et cette variation, sans compter un scénario nouveau à partir d’un timestamp.
