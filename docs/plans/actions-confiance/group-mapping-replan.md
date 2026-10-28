# R13 — mapping local des groupes signés

Reprise du 15 octobre, Codex seul. Les tranches existantes sont conservées.
Références primaires : [OIDC Core §5.1.2](https://openid.net/specs/openid-connect-core-1_0.html#AdditionalClaims)
et [RFC9068 §2.2.3](https://www.rfc-editor.org/rfc/rfc9068.html#section-2.2.3).
Les groupes sont des données signées, pas des permissions Swarm directes.

Observation : le mapping explicite compte/client → identité locale existe ;
les groupes externes ne sont pas transformés par des règles configurables.
Comparer attribution automatique et restriction des identités déjà autorisées.
Retenir la restriction : aucune identité créée, intégrité toujours locale.

Contrat `group_mapping` commun : enabled booléen strict, claim de premier niveau
(non vide, par défaut groups), rules tableau de {group,roles}. Tous les champs
internes explicites non null, y compris politique inactive et appel Go typé.
Groupes et rôles de règle uniques, non vides ; rôles connus requester/approver/
effect/admin. Règles vides seulement si inactive. Omission de la politique
conserve l’actuelle, ancienne configuration sans politique reste inactive.
Aucune durée ni limite de nombre de groupes supplémentaire.

La politique active s’applique à chaque connexion fédérée humaine ou service.
Extraction après vérification complète du jeton approprié : ID token humain,
access token de service, jamais assertion client ou champ de requête. Claim
attendu tableau de chaînes non vides. Absent/null/scalar/élément invalide refusé,
liste vide ou aucun rôle effectif refusé. Groupes inconnus ignorés ; leur union
sélectionne seulement les rôles du plafond local de ce principal. Des doublons
externes ne multiplient aucun droit. Sans politique active, comportement existant.
Les jetons locaux restent des accès opérateur/pilote explicitement distincts.

Persister seulement rôles dérivés et digest de la politique, liés à la révision
de session ; aucun groupe brut. La résolution commune réintersecte ces rôles
avec l’identité locale. Contexte et nouvelles preuves attribuent rôles effectifs
et digest. Reconfiguration, suspension, expiration/logout restent autoritaires.
Une session ancienne sans décision de mapping ne contourne pas une politique active.
Le finish step-up utilise ces rôles dérivés, puis l’accord les contrôle encore.

Admin FR/EN : activation, nom de claim, JSON des règles ; aperçu sans réseau ou
mutation, sauvegarde/réouverture, exemples et conséquences de l’activation sur
les deux types de connexion. API/CLI/Go utilisent la même validation.

Recette avant code : configuration inconnue reproduite ; invalides/null/nil,
union/intersection, groupe admin sans rôle local, aucune session sur refus,
reprise après réouverture, reconfiguration, effets/préuves locaux attribués,
service authentifié avec groupes signés, groupe d’assertion client ignoré,
step-up approver obtenu/refusé selon groupes effectifs, FR/EN/thèmes/mobile.
Les preuves communes sont réutilisées seulement aux entrées inchangées ; groupes
ciblés complets avec reçus finaux, aucune campagne globale silencieuse.


Décision de coût avant qualification : la première implémentation recherche les
règles pour chaque groupe. Comparer ce produit groupes×règles à un index construit
une fois par décision. Retenir un index local puis recherches directes, sans
introduire de limite cachée. Cette analyse ne prouve aucune cause des anciens lags.
Attendre la fin des contrôles actifs avant de modifier la source testée ; garder
les reçus de cette première variante et relancer les gardes actives après l’index.
Les branches de politique inactive et les autres parcours restent inchangés.
