# PAEA — incrément 21 : examiner une action exacte

## Reprise directe et décision APEX

L’utilisateur demande « go nxt » après commit20 `b6d61fc7c`. La décision sans GLM
persiste : Codex racine travaille seul. Les plans Identité/AC-02 gardent les
prérequis protégés ouverts ; aucun contexte fourni n’est promu en autorité.

PA-01/08 et guide §8 : lire les termes exacts et le contexte avant une demande
d’accord. Ajouter `inspect_document`, rapportv1 commun pur, CLI `protection inspect
--input FICHIER|- --json`, API `POST /api/v1/protection/inspect` et page depuis le
menu PAEA. Le format `canonicalize` existant et les préimages v1 restent inchangés.

Choix : rapport commun lié au fichier brut et rendu structuré exact, plutôt que
JSON canonique seul non lié aux octets d’entrée. Le texte canonique passe par
`RawValue` sans conversion numérique ; feature technique serde_json déclarée
explicitement dans le crate, sans fixture. UI : termes, contexte déclaré, sources
ordonnées, révisions i64 par lexème ; absence de lecteur exact refuse et propose
la CLI. Canonique exact lisible sous aide ; téléchargement du rapport brut qui
contient ce texte canonique. Aucun accord ou effet.

## Critères observables

1. Témoin sur dev96 immuable, Root isolé : page existante200, nouvelle API404 et
   commande inspect sortie2, arrêt propre. `baseline-public.json`.
2. Noyau : tous les vecteurs, i64max, raw/canonique, nouvelle tentative, sources
   ordonnées/vides ; erreurs strictes avec cause, refus sans rapport et reprise.
3. CLI/API : octets communs exacts deux domaines/FR/EN ; version et enveloppe
   fermées, admission, doublons échappés, UTF-8, flottants, causes sûres, Root inchangé.
4. UI : une entrée, termes/canonical/rapport liés aux hashes, texte seulement,
   données non authentifiées ; erreurs/reprises/chargement/clavier/exports exacts.
   Les quatre vues sont vérifiées en FR/EN × thèmes ×1440/390/320, avec les trois
   vues existantes rejouées car contrôleur/CSS partagés. Exemples existants réutilisés.
5. Documentation FR/EN, recettes maintenues, revue politiques/fonctions/erreurs
   liée aux octets ; profil natif normal et version/commit/dirty/notes/assets réels
   après livraison supportée. Aucun commit/push déduit de « next ».

Le premier précontrôle d’édition CLI a supposé deux ancres alors que la source
n’en contenait qu’une ; l’assertion a arrêté avant écriture CLI/API. Source relue,
ancre unique employée, aucune compilation de l’intégration partielle. Le noyau
seul a exécuté trois tests PASS avec inventaires avant/après conservés.

Preuves privées : `evidence/20261026-inspection21-irsgmbcu/` ; résultats ci-dessous.
Revue de même session ; **0/17 invariants de bout en bout**, E2/E3 et RETEX final
restent ouverts. Prérequis IA3 EP-01/04/05/07/10, AC-02 et autorités humaines requis.

## Résultats et livraison locale

| Contrat | Résultat observé | Preuve |
| --- | --- | --- |
| Rapport pur/JSON fermé | 2 tests schémas + 3 noyau + 2 CLI + 2 HTTP PASS, cibles nommées et sources figées | `inspection-recipe-01`, intégrité vérifiée |
| CLI fichier et entrée standard | Rapports identiques ; témoin, refus JSON sans stdout ni canari et reprise exacte ; aucun Root | `stdin-result.json` sur dev100 |
| Quatre vues natives | **2 265 assertions**, 12 configurations par vue, **48 parcours exemples /144 téléchargements réels**, **87 refus natifs attendus** avec reprises | `native-recipe-01`, `native-browser-01/result.json` |
| Validation de méthode | Perte de script injectée après témoin : arrêt en erreur, sans attente indéfinie ; aucun diagnostic inattendu dans les parcours normaux | seconds pas des recettes navigateur |
| Contrôles statiques | Clippy bibliothèques et Rustdoc privé des trois crates PASS ; warnings moteur étrangers conservés | `clippy.json`, `rustdoc.json` |
| Instance réelle | **46 assertions** en lecture seule, **20 jeux de ressources protégées** exacts, menu/détail vide/exemples, Version FR/EN, notes, commit, modifications et focus | `live-recipe-01`, intégrité vérifiée |

Le navigateur complet exerce dev100 `a03420e3f31291c39786ec873ccdaee7b4109829a6ac9af4dd9cbe1452491698`.
`./swarm.sh restart --no-open` active **v0.1.1-dev.101**, PID observé723816,
SHA256 `cdbc7e176969686bd5fe64c3e8b0a661a1aaf6b9bd119fd3d3014576a05d0b60`,
commit compilé `b6d61fc7c2335f8b4d45cd21775d61e1f6ab00f4`, modifications présentes.
Les **13 entrées compilées propres à cet incrément** sont identiques au workspace,
au candidat dev100 et à l’archive effective de dev101. Profil normal dev101 :
111 artefacts, quatre crates cœur sans feature fixture et hors profil test ;
467 entrées effectives archivées. Les autres évolutions concurrentes ne sont pas
qualifiées par cette revue. `installed-candidate.json` lie ces preuves au binaire
réel ; configuration `deploy/local-web.json` inchangée. Le contrôle réel ne soumet
aucun document métier et n’utilise pas la base active comme fixture.

Deux captures ont été examinées visuellement : FR/état/390 règlement et
EN/sombre/320 Git (`native-browser-01/*-inspection-*/result.png`). Pas de débordement,
termes/sources et révisions maximales lisibles ; contrastes, clavier, chargement,
refus et reprise sont vérifiés par les assertions du parcours natif. Le rapport
exporté est exact et contient le texte canonique ; aucun second bouton de fichier
canonique n’est promis.

## Incidents conservés, revue et RETEX de l’incrément

La première compilation réussit, puis la fraîcheur du profil refuse une modification
concurrente de `native_preparation.rs` (`build-result.json`, `replan-build-01.json`).
Le serveur précédent n’est pas arrêté par cette tentative. Une nouvelle construction
stable qualifie dev100 ; le redémarrage supporté réutilise ensuite dev101 déjà
qualifié. Aucun garde contourné, aucun fichier concurrent corrigé pour forcer le
résultat. Le chemin de livraison source demandé est opérationnel pour cet incrément ;
l’ancien incident dev97 du lot20 reste dans son historique.

Revue de **40 fichiers concernés**, liée aux hashes dans `source-review.json` :
politiques (invariants v1/i64/empreintes et représentation technique, aucune nouvelle
politique opérateur), contrats des fonctions et propagation des erreurs jusqu’à la
CLI/API/UI, documentation FR/EN et recettes. Les erreurs effacent l’export et le
contenu précédent ; les causes typées restent disponibles, les diagnostics publics
restent sûrs. Les écritures/flush réutilisent le contrat CLI existant ; aucune
nouvelle injection de panne de flush n’est revendiquée. Les anciennes recettes HTTP
complètes déclarent maintenant dix tests, mais seuls les deux nouveaux tests HTTP
ont été rejoués dans cette recette ciblée. Revue de même session, sans acceptation
indépendante. Les modifications d’autrui et leur index sont préservés.

Apprentissage réutilisable : conserver les entiers signés sous forme canonique
exacte à travers producteur, transport, lecteur numérique et rendu. Un parseur JSON
arrondissant le nombre ne suffit pas ; le parcours prouve le lexème i64 maximal et
le refus explicite du lecteur incapable. Cette limite est dans le contrat FR/EN,
les docstrings et les cas maintenus, sans modifier les contextes embarqués des agents.

Incrément21 réalisé, vérifié localement et installé. Aucun GLM, délégation, commit,
push, paiement, promotion Git ni accord humain effectué. Le programme PAEA reste
**0/17 invariants protégés de bout en bout** : IA3 EP-01/04/05/07/10, AC-02,
autorités humaines, E2/E3, revue indépendante et RETEX final du programme restent
ouverts. La suite doit relire ces préconditions et le plan ; ce résultat de lecture
structurelle ne les clôt pas.
