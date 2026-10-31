# Bell-LaPadula — APEX, 31 octobre 2026

Propriétaire : Codex racine. Autorisation : demande « Implement the plan ».
Travail direct, sans délégation ni consommation fournisseur supplémentaire.
Préserver les travaux PAEA, préparation et migration présents à la reprise.

[English](../../en/plans/actions-confiance/BLP-APEX.md).

## Rattachement au programme PAEA

Ce plan décompose techniquement la confidentialité du [plan PAEA](PAEA-APEX.md).
La section « Rattachement Bell-LaPadula » de ce dernier relie BLP-01–07 aux lots
PA-00–13, invariants I01–I17 et tests T01–T22 existants, sans les renuméroter ni
créer de second registre d'assurance. PAEA porte les critères de qualification
et leur état ; ce suivi conserve les actions techniques, preuves et reprises BLP.
La reprise GLM/SKYNET doit recevoir les deux plans et restituer les références
PAEA correspondantes. Modifier ces fichiers ne met pas à jour une session déjà
démarrée ni les instructions embarquées du moteur.

Ce rattachement documentaire ne reprend pas le développement arrêté, ne démarre
pas AC-02 complet et n'établit aucun nouvel invariant vérifié ou accepté. Les
limites du WIP ci-dessous restent historiques ; ne pas les traiter comme un
diagnostic courant de sources modifiées par une autre session sans vérification.

Revue de cet incrément : documentation seulement, sans modification des politiques
exécutées, schémas, fonctions ou chemins d'erreur. Les scénarios ajoutés sont des
critères à vérifier, pas des résultats. Les notes de version et l'installation
ne changent pas : aucun comportement produit n'est livré par ce rattachement.

## Reprise des acquis PAEA — obligation explicite

Les actions PAEA déjà réalisées, y compris livrées ou déclarées terminées,
doivent être examinées à travers les sept lots BLP-01–07. La section « Reprise
obligatoire des actions déjà réalisées » du [plan PAEA](PAEA-APEX.md#reprise-obligatoire-des-actions-déjà-réalisées)
définit le relevé requis par action : applicabilité justifiée, producteurs et
consommateurs, candidat/profil, preuves réutilisables, écarts, propriétaire et
condition de reprise/clôture. Ce relevé reste dans le suivi existant.

Conserver les preuves et résultats historiques ; ne pas leur attribuer une
garantie de confidentialité non établie. Réouvrir les critères concernés et
reprendre les seules vérifications affectées, en réutilisant les preuves dont
les entrées et la portée restent valides. BLP-07 ne clôt pas sans réexamen des
acquis et traitement des écarts applicables.

État actualisé : [réexamen d'applicabilité réalisé](paea/BLP-REASSESSMENT-20261031.md),
115 actions et 805 décisions dans le [relevé](paea/BLP-REASSESSMENT-20261031-ANNEX.md).
Les huit écarts BLP-RET-01–08 ont un propriétaire et une condition de reprise ;
ils rouvrent ou maintiennent ouverts les critères BLP applicables, sans effacer
les acquis historiques. Les 150 intrants initiaux de revue sont archivés sous
empreintes ; les preuves de livraison/UI ajoutées concurremment sont relevées
séparément dans le rapport. Cette revue ne réinstalle rien et ne réécrit pas les
preuves de livraison du socle de l’autre session. BLP-07 transversal ne peut clôturer avant les
raccords, contrôles et écarts applicables, notamment BLP-01/04/05/06, même si le
socle d'administration est livré et son RETEX rédigé. Réception de la revue par
la session de reprise, corrections et vérifications de bout en bout restent ouvertes.

## Contrat adopté et ordre

Confidentialité transversale Projets/Processus, complémentaire de Biba ; migration
explicite de l'existant, déclassification humaine exacte, sous-ensemble borné des
barrières fichiers/réseau AC-02. Le reste d'AC-02 reste différé. Aucun rôle client,
label désérialisé ou champ de provenance ne constitue une autorité.

| Lot | Livrable et critère observable | Dépendance | État |
|---|---|---|---|
| BLP-01 | Inventaire sources, consommateurs, sorties et frontières, modèle de menace | — | EN COURS |
| BLP-02 | Moniteur de dominance/combinaison, domaines fixes, erreurs sûres, indépendance Biba | 01 | À FAIRE |
| BLP-03 | Configuration persistée révisionnée, migration explicite, Admin/API/CLI | 02 | À FAIRE |
| BLP-04 | Attribution par producteurs, contexte/mémoire/sorties et replays | 03 | À FAIRE |
| BLP-05 | Médiation réelle fichiers/réseau/fournisseurs/descendants, refus sans repli | 04 | À FAIRE |
| BLP-06 | Identité humaine établie, validation des octets/destinataires, nouvelle copie | 03–05 | À FAIRE |
| BLP-07 | Recettes, revue du candidat, livraison locale, ressources servies et RETEX | 01–06 | À FAIRE |

Chaque lot inclut documentation FR/EN, contrats des fonctions et revue explicite
politiques/fonctions/erreurs liée aux octets. Aucun critère critique non vérifié
ne devient conforme. La revue dans ce contexte n'est pas indépendante.

## Analyse et décisions

Deux voies : contrôle des seuls prompts/outils déclarés, ou moniteur commun et
médiation des accès réels. Choix : seconde voie ; la première permettrait les
lectures et sorties directes par le fournisseur et ses descendants.

Classification = niveau ordonné + compartiments. Dominance = niveau supérieur
ou égal et sur-ensemble des compartiments. Combinaison = maximum des niveaux
et union des compartiments. Aucun lien implicite avec les niveaux Biba.
Un domaine d'exécution est fixe ; aucune baisse après exposition au secret.
Une source non classifiée refuse les nouveaux usages protégés. Un résumé,
une sortie vide ou un filtrage LLM n'est pas une déclassification.

Catalogue initial proposé : Public, Interne, Confidentiel, Restreint ; son
édition est une politique persistée, jamais une constante d'autorisation.
Les destinataires effectifs sont liés au service/compte/endpoint, pas au seul
nom du modèle. Toute décision doit être revalidée à l'engagement.

## Reprise observée

- Branche `codex/clear-product-method-names`, changements tiers conservés.
- `swarm.sh status` : aucun serveur géré actif ; aucune version installée
  déduite d'un ancien rapport.
- Disque : environ 14 Gio disponibles sur le volume du dépôt ; aucun nettoyage
  ou copie complète autorisé implicitement. Préserver les originaux, écrire
  atomiquement ; aucune donnée de projet dans `/tmp`.
- Le contrat `swarm-protection-contract::SourceRef` représente des labels,
  sans établir provenance ni contrôle effectif.
- Le moniteur Biba pur ne gère pas la confidentialité. La frontière Linux
  autorise des lectures hors racine privée ; elle ne confine pas Internet.

## Inventaire initial des frontières

| Frontière | Producteur/consommateur inspecté | Preuve ou travail restant |
|---|---|---|
| Étiquettes et propositions | `swarm-protection-contract` | Représentation non autorisante ; adaptateur typé à établir |
| Autorité/configuration | `root_store/trust_store`, `trust/authentication` | Réservation Trust, racine ouverte, identité courante ; habilitations BLP absentes |
| Fournisseurs CLI | `native_provider_exec` et `native_provider_biba` | stdin scellé, environnement sélectionné, frontière Biba ; médiation BLP absente |
| API, SSE, préparation | `root_store/preparation_call`, `preparation_call_cli` | Inventorier les transmissions effectives et reprises avant raccord |
| Mémoire et fichiers | `memory_files`, `project_memory_source`, sources de préparation | Attribution, versions, imports et lecteurs à raccorder |
| HTTP et opérateur | `service/trust`, CLI `trust` | Session/CSRF distincts de l'identité métier ; pas d'habilitation BLP actuelle |
| Déclassification humaine | Enrôlement/approbations PAEA existants | Un rôle I3 seul ne prouve pas une personne ; raccord exact à établir |

## Preuves et clôture

Recette permanente sous `tools/verification/recipes/`, preuves neuves dans
`docs/plans/actions-confiance/verification-blp/`. Témoins valides avant chaque
injection, refus sans divulgation observé côté récepteur, reprise et persistance.
Conserver empreintes, argv, sortie originale, profil/features et image exercée.
Séparer tests de contrat, consommateurs publics, confinement, installation et
acceptation indépendante. Les lots ouverts ci-dessus interdisent une clôture globale.

## Arrêt demandé et sauvegarde WIP — 31 octobre

L'utilisateur a demandé l'arrêt du développement, un prompt de reprise GLM/SKYNET,
puis un commit. Ce commit sauvegarde un **travail incomplet**, sans qualification,
installation, activation transversale ni acceptation indépendante.

- BLP-02 : crate `swarm-confidentiality` écrite ; dix tests ont passé **avant**
  l'ajout ultérieur de la liaison de révision du domaine. Aucun PASS sur les octets
  finaux n'est établi.
- BLP-03 : historique persisté, opérations `blp-show`, `blp-policy-preview`,
  `blp-policy-apply`, `blp-check`, tests d'intégration et section Admin écrits,
  non qualifiés. Le diagnostic est explicitement non autorisant et l'interface
  annonce que la protection transversale n'est pas active.
- Une compilation moteur a échoué (E0308, String au lieu de &str pour
  `CompactJson::new`). Correction écrite, non recompilée.
- La revue du diff avant commit relève un autre défaut ouvert : les noms
  d'opérations BLP ont été insérés comme une expression `|` dans la branche
  GET/HEAD de `service/trust.rs`, au lieu de la liste `matches!` des opérations.
  Le raccord HTTP n'est donc pas compilable et les opérations POST BLP restent
  absentes de la liste autorisée. Corriger puis vérifier au prochain incrément.
- BLP-01 reste partiel ; BLP-04 à BLP-07 restent ouverts. Migration des objets,
  attribution réelle, médiation fichiers/réseau, déclassification humaine,
  documentation technique FR/EN, traductions, aide CLI, évolutions, recette et
  revues restent à réaliser/vérifier. Aucune conformité globale n'est revendiquée.
- Propriétaire de reprise : session outillée GLM/SKYNET choisie par l'utilisateur.
  Relire ce suivi, les sources et le prompt transmis ; préserver les changements
  tiers. Aucune délégation n'a été lancée par Codex.

La commande de commit est une sauvegarde Git demandée ; elle ne reprend pas le
développement arrêté et ne déclenche ni build, ni installation, ni publication.
