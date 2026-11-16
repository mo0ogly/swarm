# POC06 stdio — sauvegarde WIP / WIP safeguard

Base exacte : `df1f38b78af8324fb7927ffef8f1ce8a4f14f30d` (Biba5 préservé).
Sources retenues : paquets04 new6pj, caller r8, harness typé et handler guards.
Caller : `939fa5a52a62cea28ec68855cdff70bddee88b4d649a90e7fbec50b7ef9e3349`.
Harness : `054e376a04a9bf1e2b43e0b74e2bcfe26b0d4f4c9d34475ad3014b399733f9a9`.
Handler : `2a6f5721976a5356b0d3c9b73fdd3fb7829d6fc21238d4bbab9027dae6a9f0e2`.
Le producteur27b2 existe déjà dans230d/8d/df1f ; il est inchangé, hors commit.
L'ancienne indication d'absence dans les paquets est corrigée explicitement.

Les documents BIBA-RUST conservent intégralement les octets df1f et ajoutent
seulement les suffixes04 extraits après leur préfixe exact8d, avec un en-tête
signalant la portée historique. Préfixes, suffixes et nouveaux hashes composés
sont liés dans le reçu de sauvegarde. Aucun fichier Rust Biba ni hunk Biba5
n'est remplacé. Les autres recettes/docs/test postimages sont exactes aux paquets.

Revues source bornées (pas acceptation produit) :
- 6pj/02 : `38ea9eb165d2bfcbb3418a229d8406632e4b76fb3541b1382bf5922f7aaa12a6`.
- Caller939/09 : `e4a4f8a307fc430755df7939c421b002ed7e8b7bd1673057a72b177cb2965ebe`.
- Harness054e/09 : `30f314cf10e2fa9fc2316e13ce52c774b292a5558cd2cce4d6eaef75f1019821`.
- Handler/02 : `556b599b6f5b9c66e1561fb6d7bbc6e926db2a98d1922b2806ead357e7e4e846`.

Preuves réutilisées sans replay : huit méthodes inertes054e réussies, reçues09,
puis huit méthodes inertes handler2a6 réussies, reçues02. Ce ne sont pas seize
contrôles Biba réels. Revue runtime09 :
`135bc23f83858a2ff610707311abbef73bfc2e6d658c8f25f5c59a2a186fafbf` ; réception02 :
`6a9af20830e0c1da8ece59048baa0cf90f7b4b8ca2ca33ea0b27a844de2841c9`.
Un témoin Git réel étroit est observé PASS dans le résultat root :
`/run/media/fpizzi/SWARM/swarm-evidence/biba-git-admission-root-20261116-2vq6mlzw/real-first-run/root-result.json`,
SHA `0e403ba9a2401a55933f47636f18d35156238923b7383d9e74670302c06459e0`.
Sa réception indépendante09 est distincte et n'est pas déclarée acquise ici.
La preuve porte sur une fixture composant ; ni close lowFD réel, ni projection
publique moteur, ni profil produit complet, installation ou acceptation globale.

Sauvegarde WIP seulement, sans PR ni fusion. Aucun des tests, Git fixtures,
drivers, builds, providers ou services n'est lancé par ce commit. Les mentions
historiques « non exécuté » des annexes sont celles de leurs paquets immuables,
pas une négation des preuves ultérieures reçues ci-dessus. Root séquence toute
qualification restante. Restaurer061b propre et42helpers après sauvegarde.

English: source-only WIP on exactdf1f, preserving every Biba5 source/doc byte.
Append only the04 suffixes after their exact8d prefixes; record all composition
hashes and explicitly qualify historical unexecuted/producer-absence statements.
Producer27b2 already exists and stays unchanged. Keep exact caller939,harness054e,
handler2a6 and recipes, with bounded source approvals linked above. Reuse received
8+8 inert methods without replay; they are not16realBiba checks. Root recorded one
narrow actualGit fixture PASS; independent09 reception is not claimed here.
Component-fixture evidence establishes no real lowFD close, publicengine flow,
complete production profile, installation or global acceptance. No PR/mainmerge,
new test/build/driver/Gitfixture/provider/service execution. Restore clean061b
and its42helpers; root owns remaining gates and source/target allocation.
