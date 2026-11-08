# Peter Wilcheck — rapport vérifié R1/R2 et proposition pour X

8 novembre 2026 — brouillon local à utiliser par Fabrice, rien publié.

## Ce qui a déclenché le travail

Le [message de Peter Jonathan Wilcheck, @gowilcheck](https://x.com/gowilcheck/status/2119440681475211359)
soulignait l'importance des permissions limitées, des contrôles indépendants et
d'une approbation humaine significative lorsque des agents peuvent agir.
Le texte du message a été relu dans sa capture conservée du 8 novembre ; son
article externe n'a pas été audité et la disponibilité publique actuelle du post
n'a pas été revérifiée. Aucun défaut de Swarm n'est déduit de cet échange.

Nous en avons tiré un scénario local précis : une sortie d'outil qui rapporte un
faux accord humain doit rester une donnée, sans devenir une décision autorisée.
Ce scénario vient de notre cadrage ; il n'est pas attribué à Peter comme une
attaque qu'il aurait fournie ou une vulnérabilité qu'il aurait découverte.

## Résultats à raconter

| Travail | Résultat observé | Portée exacte |
| --- | --- | --- |
| R1 : donnée et autorité | Un test Rust réussi, trois variantes, 21 marqueurs nommés ; lecture autorisée, faux accord refusé sans mutation métier ; accord exact par une identité authentifiée du banc puis un seul effet, conservé après réouverture et rejeu | API de bibliothèque Trust et registre fictif ; enveloppe outil fabriquée par le banc, aucun modèle réel ni personne humaine enrôlée |
| QW3 : preuves archivées | 23 tests Python R2 au total ; la commande archive refuse les originaux manquants, liens/répertoires, doublons sélectionnés, mauvais hashes et troncatures testées. Sur les vrais supports, le mauvais pointeur du 4 novembre est refusé ; le fichier attendu est retrouvé avec son hash dans les 13 parts du 31 octobre | Membres explicitement attendus d'un tar non compressé ; pas un hash/authentification de tout le contenu historique |
| QW4 : base Git | La commande compare branche/HEAD/suivi local/source, refuse les bases absentes, retardées, divergentes ou incompatibles testées. Sur main : commit source inclus, égalité avec le suivi local, travail non commité signalé ; HEAD et status inchangés avant/après | Observation locale sans fetch, checkout, merge, sauvegarde ou verrou d'exclusion ; le travail dirty n'est pas sauvegardé par cette commande |

Les deux précontrôles sont des outils de notre méthode de vérification, exécutés
explicitement. Ils ne constituent pas de nouveaux contrôles automatiques du
moteur et n'installent pas un mécanisme permanent dans les sessions d'agents.
Leur origine immédiate est notre reprise de preuves et notre diagnostic Git,
découverts pendant le travail inspiré par l'échange avec Peter.

Aucun fournisseur, modèle réel ou service produit n'a été lancé pour R1/R2.
Le travail est directement sur main, non commité ; aucun push ni publication.
La revue est une auto-revue. Peter n'a ni revu ni accepté ces résultats.

## Ce qu'il ne faut pas annoncer

Pas de garantie globale contre l'injection, de preuve qu'un modèle reconnaît une
fausse approbation, de contrôle de tous les outils ou shells, d'identité humaine
vérifiée, de liaison native tâche/tentative, de confinement OS, ni de conformité
globale. Le test R1 n'établit pas une protection automatique de tous les agents.
Les outils Git/archive ne garantissent ni l'absence future d'erreur ni la fraîcheur
d'un serveur Git distant. Une empreinte locale ne prouve pas l'auteur d'une preuve.

## Remerciement proposé — anglais, brouillon local

Réponse destinée au message de Peter, non envoyée :

> Thanks @gowilcheck. Your post prompted an isolated Swarm regression: 3 variants passed. Forged approval data granted no authority; an authenticated fixture approver enabled one simulated effect, with no duplicate on replay.

Suite facultative, à publier seulement si Fabrice le décide :

> Scope: a deterministic local Trust fixture, with no real model or authenticated human. We also added archive/Git preflights: 23 tests passed. Those improve our verification method; they don't establish general prompt-injection immunity.

Comptage du texte exact : 223 et 236 caractères ASCII, sans lien ni média.
Le composeur X et le rendu public n'ont pas été testés ; aucune programmation.
Les limites essentielles sont dans le texte, pas uniquement dans ce rapport.

## Pistes de preuve internes pour relire avant un envoi

| Affirmation | Passage/source lu | Preuve et limite |
| --- | --- | --- |
| Source et attribution Peter | `docs/communication/x-swarm-20261107/lot14-x-sources.json`, entrée URL Peter et snapshot ; `IDEES-DISCUSSIONS.md:643–655` | Capture historique relue, pas l'article externe ni une vérification publique actuelle |
| Faux accord sans nouveaux droits ; reprise exacte | `rust/crates/swarm-engine/tests/trust_biba.rs:118–219` et `trust/operations.rs:750–787,1111–1208` | R1 `run-01` PASS, 21 marqueurs et observations persistantes ; fixture seulement |
| Précontrôle archive | `tools/verification/archive_preflight.py`, `expected_members`, `inspect_archive`, `SplitArchive` | R2 recette PASS et `real-01/insufficient-pointer.json`, `historical-split-recovery.json` ; sélection explicite, pas authenticité indépendante |
| Précontrôle Git | `tools/verification/git_preflight.py`, `observe`, `inspect_git` | R2 tests et `real-01/main-git.json`, comparaison avant/après par collecteur ; références locales seulement |
| Aucun élargissement automatique du produit | `tools/verification/PREFLIGHTS.md` et implémentations CLI | Deux programmes d'outillage explicites ; pas de raccord moteur/session ni installation/service |

Plan, résultats et RETEX : [registre canonique](../AGENT-CONTROL-POCS.md).
Contrats et commandes R2 : [précontrôles](../../../tools/verification/PREFLIGHTS.md).
R1 : `/media/fpizzi/Lexar/swarm-evidence/peter-apex-r1-20261108/`.
R2 : index local `.cache/peter-preflights-r2-20261108/LOCATION.json`, avec référence
vers la copie USB relue. Les sorties brutes et le rapport de revue gardent les
hashes des sources exercées ; ne pas transformer ce rapport en affirmation plus
large. État éditorial : brouillon local non publié, sans autorisation d'envoi déduite.
