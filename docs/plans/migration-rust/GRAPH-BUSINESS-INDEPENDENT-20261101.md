# Revue indépendante Graph B1–B4

Autorisation utilisateur « gfo », après demande explicite de sous-agent lecture
seule. Relecteur : `/root/graph_independent_review`, skill code-reviewer, contexte
distinct. Aucun fichier modifié, aucun test exécuté par ce relecteur.

## Candidat302 : request changes

Version v0.1.1-dev.302, SHA256
`02e4662938e85a50e631c9a1dfd3d27916d0357358da9eca370bb90234802a6e`.
Finding confirmé P2 : GraphTaskDefinition complète passe par Request partiel ;
titre/livrable vide signifie inchangé dans update_task_definition, mais preview
et événement conservent le faux after vide. Le Work ne correspond pas à l’aperçu
et la compensation peut refuser graph-definition-stale. Emplacement302 :
graph_batch.rs420–447 ; task_definition.rs242–253. Reproduction CLI302 lue dans
`.cache/graph-business-20261101/before-full-definition/result.json`.

Correction proposée par le relecteur : refuser les définitions complètes avec
titre/livrable vides ou blancs, préserver task.update historique partiel,
vérifier la projection réelle du candidat contre after ; tests refus sans
mutation, reprise valide et compensation. Aucun autre défaut confirmé dans le
périmètre. Approbation à renouveler sur le nouveau candidat.

Contrôles indépendants réalisés : binaire et13entrées compilées/UI/notes conformes
aux empreintes302 ; inventaires de recettes35/35,23/23,14/14 inchangés ;
result.json et logs originaux des trois recettes lus, dont13tests nommés.
Inspection des chemins transaction/replay/droits/API/CLI/staging/verrou/erreurs
et docs FR/EN. Aucun test exécuté par la revue. Verdict302 conservé, aucune
installation ou conformité globale BLP/Authority attestée.

## Candidat303 : approve

Verdict renouvelé du même relecteur indépendant pour Graph B1–B4, version
v0.1.1-dev.303, SHA256
`5f22359a9b0acfed477b0e2e95b80d814a81b08d885362f6ac060be8164f518a`.
Finding302 corrigé : titre/livrable vides ou blancs refusés avant adaptateur ;
projection réellement préparée égale à after, compensation incluse. Refus attendus
422, dérive interne500, task.update historique inchangé. Aucun nouveau défaut
confirmé. Le verdict302 request changes demeure une preuve historique.

Vérifications lecture seule du relecteur : diff302→303 lu directement dans les
deux archives effectives, unique entrée compilée graph_batch.rs ; empreinte
binaire et profil sans fixtures ; régression initialement FAIL puis PASS ciblé ;
test HTTP étendu ; recette303 finale14tests nommés +246assertions navigateur PASS
sur ressources embarquées303 ;35entrées déclarées toujours identiques aux sources.
1409assertions graphe et145responsive de302 restent réutilisées pour producteurs
inchangés, sans prétendre qu’elles ont été réexécutées sur303. Aucun test exécuté
ni fichier modifié par le relecteur. L’approbation porte sur Graph examiné,
pas sur installation ni conformité globale des travaux parallèles.
