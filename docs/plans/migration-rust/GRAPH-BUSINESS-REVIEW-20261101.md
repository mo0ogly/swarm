# Revue locale B1–B4, candidat302

Revue Codex dans le même contexte, **pas une revue indépendante**, ni une
conformité globale migration/BLP. Candidat v0.1.1-dev.302, SHA256
`02e4662938e85a50e631c9a1dfd3d27916d0357358da9eca370bb90234802a6e`.
Baseline298 : `.cache/local-delivery/rust-ch511cy4/build/profile/after.json` ;
courant : `.cache/local-delivery/rust-i24x6af9/build/profile/after.json`.
Le record299 reste une observation intermédiaire, jamais une façon de masquer
les nouveaux producteurs GraphBatch. Aucun commit/push/installation revendiqué.

## Politiques et valeurs

- DTO schema1, digest SHA256 hex64, révision positive, nonce AES-GCM12octets et
  tag minimal16octets : invariants de format/crypto, pas des plafonds opérateur.
- Taille transport, nombre de critères et phases manuelles : sources existantes
  EngineSettings persistées → validation → Admin/API/CLI → mutation_limits →
  prepare. Aucun deuxième plafond de lot, timeout ou retry dans ce producteur.
- Graph rights : table historique graph_draft_authorizations, prepare/apply
  distincts, scopes et révision relus ; absence uniquement opérateur local
  existant. Ce comportement Go hérité ne prouve pas une identité distante.
- Undo limité au dernier événement normal de ce Work : invariant de provenance
  et intégrité ; jamais de restauration arbitraire ou de réanimation des preuves.
  Nouvelle tâche supprimable seulement si reconstruction vierge identique avec
  paramètres courants, aucun agent historique/attempt/review. Changement de
  paramètres peut donc refuser conservativement son retrait.
- Web Lock : exclusion technique d’un même document entre clients coopérants,
  libérée à fermeture/destruction du contexte ; aucun délai ou attente ajoutés.
  Vérification copie attendue avant remplacement local ; sans API crypto/verrou,
  refus visible. Ce mécanisme ne confine pas un script de même origine hostile
  et ne garantit pas de sauvegarde serveur ou fsync navigateur.
- Les IDs, seuils de pixels et injections quota/ack/SQL du banc sont des entrées
  de test isolées. Aucun Root de mission courante n’est modifié.

## Fonctions et documentation

GraphTaskDefinition::observed/request, decode_graph_batch, query, changes,
authorize/authorize_replay et prepare portent projection, décodage strict,
finalisation, provenance, droits et validation privée. refuse/encoded sont de
petits adaptateurs typés. RootStore::preview_graph_batch documente snapshot,
rollback et réservation ; WorkEngine::execute/apply gardent le commit/replay
commun, require_native_task_idle garde son contrat d’activité inchangé.
L’export module/type ne crée pas d’autre consommateur implicite.

HTTP graph_batch/status/public_reason documentent garde Root, acquittement et
classification ; dispatch CLI partagé conserve output/close/recorded metadata.
Les aides FR/EN décrivent graph-preview/graph-apply. Aucune commande externe
inventée pour la compensation (undo_of fait partie du même DTO natif).

Frontend : taskDefinition, graphChangesValid, businessRecordValid,
openBusinessDraft et ses helpers action/fail/run/projection/claimDraft/persist/
reviseLocal/stage/send/inspect/apply/definitionPanel/render ont des commentaires
de contrat. Le formulaire conserve l’intention créer/modifier et la saisie sur
refus. Fermer restaure focus/vue ; les dialogues s’appuient sur jetons partagés.
Documentation produit FR/EN GraphStudio : format, limites, CLI/API, erreurs,
reprise, stockage et Mermaid. Pas de changement d’instructions embarquées agents.

## Erreurs et effets

Entrée inconnue/dupliquée refuse avant mutation. SQL est paramétré et finalisé
explicitement : erreur primaire et erreur de cleanup restent dans AgentLaunchError
puis WorkEngineError::Graph::source. La matrice native existante parcourt les
wrappers AgentLaunch, IO custom et causes secondaires ; aucun nouveau parcours
opaque n’est ajouté. Snapshot preview rollback + release conservent deux causes.
Apply utilise la transaction WorkEngine existante : l’injection d’échec INSERT
événement après UPDATE Work vérifie rollback et reprise de la même intention.

Replay relit les droits courants, ne redonne pas un succès après révocation.
HTTP ne transforme en403/409/422/501 que les refus typés du nouveau parcours ;
storage/rollback/close restent serveur. Corps publics limités aux codes statiques,
aucun contenu SQL ou secret. Acquittement distingue événement/révision appliquée,
Work actuel et replay ; état connu conservé lors d’échec encode/close/output.

Frontend : pas de POST si persistance refusée, requête exacte conservée avant
envoi, résultat inconnu gelé puis reprise explicite avec même nonce. Preview
échoué et rebasage refusé invalident toujours l’ancien aperçu. IDs affichés
comme texte et jamais comme codes. Disque illisible préservé jusqu’à geste
d’écartement explicite par son propriétaire. Second onglet sans verrou ne peut
ni modifier ni effacer le brouillon. Édition acceptée seulement après sauvegarde.

## Preuves et limites

Contre-exemples originaux, progression et décisions dans
[le suivi](GRAPH-REGRESSION-20261031.md). Recettes finales :
`.cache/graph-business-20261101/final302/{business,graph,responsive}`.
Le profil natif qualifié exclut test-fixtures ; les binaires Cargo de test restent
distincts de l’image de livraison. Les tests isolés ne prouvent pas installation.
Les résultats de la recette finale et son intégrité doivent être joints après
exécution ; aucune réussite future n’est anticipée ici.

La baseline contient26entrées compilées changées, incluant des travaux parallèles
Authority/BLP. Les ajouts Graph sont revus ici ; pour les fichiers partagés, cela
ne vaut pas revue de tous les ajouts d’un autre propriétaire. Le relevé parallèle
`docs/plans/actions-confiance/verification-blp/20261101-preparation-quarantine-source-review.json`
conserve son propre périmètre, ses limites et ses preuves, sans acceptation globale.
Revue indépendante risqueA reste ouverte avant clôture ; sa méthode n’accorde
aucune autorisation de délégation.

## Empreintes examinées

- `rust/crates/swarm-engine/src/graph_batch.rs` : `a97bf8d702d7c85a828e94940c11dd9fa3167b82fed77e6b811a49855a3a1683`
- `rust/crates/swarm-engine/src/root_store/graph_batch.rs` : `f9024429a18751c7b43a3bd49b837603e4f9bb54b8f4a07229f0ca24d1a2d2ed`
- `rust/crates/swarm-engine/src/work_engine.rs` : `ccf4ebb6c83a10f798fedc27d8cf14cf1c4de35465e1c4bd425e08678f8f42d7`
- `rust/crates/swarm-engine/src/lib.rs` : `6870f564cd6de3cd82a2eb85aab681b4d56ad9e5bdec6dd2dd50d7e059249119`
- `rust/crates/swarm-engine/src/root_store.rs` : `cf1770cfc7a9657927cc954e3e472820c4683424ded72068fc88c97258f1f493`
- `rust/crates/swarm-web/src/service/graph_batch.rs` : `313c98ef9b0db7c844969e6c6affeef87ea63ccaa029299be7733f78496833c8`
- `rust/crates/swarm-web/src/service.rs` : `fe238d4fbef71c11d76662a7ac7356e7fdbfeb4848e16d96523ea6900c187d29`
- `rust/crates/swarm-cli/src/lib.rs` : `258f9bd9f2d0011ee64693a9009dd90a5d8abefe06c953e7c7006bd92752c698`
- `web/cockpit-native-graph.js` : `50dc3799b84b3c4321461f792fde7d99d83ea977fd8b68dfde0d1f4c9d50447a`
- `web/graph.css` : `560c0934ab80073a99085f50497e57e6b99328c170c816a20f280b500ae00020`
- `locales/en.json` : `f173499702db1c355a0517f1d2f8ed3aba5bad9b86f333a66a7d7a3466b1bcdb`
- `web/i18n-en.js` : `4103d588092b39060d0d6d978a8ac07fcceb663bb98ef29fa6c122a126c922af`
- `version_history.json` : `f7530ff5fb9bdb28e594859ed0015c01fcc1056f6cdae9389ccb9235752aa5f8`
- `docs/architecture/GRAPH-STUDIO.md` : `fc405908bc488d342a3d0709d7b0a00c62c5c21534aae58f4fc01b7494830513`
- `docs/en/GRAPH-STUDIO.md` : `fa9b3185fec8640a0cf6ff7d8cb202388f8f272429dd9d5a271872e3a96c76d0`
- `rust/crates/swarm-engine/tests/graph_batch.rs` : `0710ea15731e814b59bf7362953ed857eeea4af48a8797e54184d6e9417248ef`
- `rust/crates/swarm-web/tests/graph_batch.rs` : `dce2eb9177612223365fc9d1dabb5ccb8ccae5328825a6d47f4647ea44ef0405`
- `rust/crates/swarm-cli/tests/graph_batch.rs` : `933d9303877ef9476bf773d6a74b1b8145e5b93a42ca72f7ad75b87fd003f420`
- `tests/rust_graph_business.py` : `bbaff495aa15031e885dc26161adaaff33e5af9fa375118031431ac03f1b3758`
- `tests/rust_graph_business.cjs` : `23834e7c7ceda5aa4117fbb8f8c1749c4605aeb0360d083a3df58574f5528ad0`
- `tools/verification/recipes/rust-graph-business.json` : `94e85d91851534ff7b98245b065e6234f525c09521e169c048da3fec59283957`
