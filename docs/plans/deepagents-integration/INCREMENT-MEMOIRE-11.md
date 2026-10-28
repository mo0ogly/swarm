# M15 — retrait et mémoire héritée d'une conversation

16 octobre 2026, reprise « next ». Codex seul ; APEX et `replan`.
MEM-U04/MEM-U08/MEM-U14 ; cas MEM-C18, avec dépendances MEM-C15/MEM-C16.

## Observation et choix

M14 contrôle les références du dernier contexte. Or reprendre une session
fournisseur conserve aussi le contexte de ses tentatives précédentes. Le code
de `projectMemoryGuard` et de `memoryUses` ne parcourt pas cette filiation ;
la transaction de reprise ne revérifie pas ses sources mémoire après la sonde.
Ces constats de lecture doivent être reproduits avant d'être déclarés défauts.

Deux voies sont possibles : reconstruire automatiquement une autre conversation
en expurgeant l'ancienne leçon, ou refuser la reprise incompatible et laisser
le responsable choisir une nouvelle conversation. La première demanderait un
contrat de projection et sa qualification fournisseur ; elle ne prouverait pas
l'effacement de la session précédente. Le présent lot retient le refus explicite,
sans purge, résumé automatique, arrêt d'une conversation active ou appel réel.

Le moteur doit suivre les références réellement reçues dans la chaîne conservée,
les vérifier avant la reprise, après la sonde dans sa transaction, puis avant
départ. Une référence héritée doit encore appartenir à la sélection courante.
Le catalogue et l'aperçu de retrait doivent signaler les usages hérités actifs.
Une chaîne incohérente ou inaccessible ne devient pas un succès vide. Les
versions, prompts et checkpoints historiques restent intacts ; aucune nouvelle
politique, limite ou migration n'est introduite.

| Lot | Propriétaire, dépendance | Critère et vérification |
| --- | --- | --- |
| M15.1 | Codex ; état M14 conservé | Contre-exemples de désélection, filiation héritée et retrait pendant la sonde, sans mutation après refus. |
| M15.2 | Codex ; M15.1 | Lecteur partagé, sélection actuelle, versions actives, transaction et départ ; cycles/chaînes manquantes refusés, usages hérités attribués. |
| M15.3 | Codex ; M15.2 | Vraies CLI/API/interface isolées : diagnostic bloquant après désélection, reprise avec sélection rétablie, retrait conservant les originaux. FR/EN, thèmes et clavier selon les surfaces affectées. |
| M15.4 | Codex ; preuves précédentes | Tests ciblés et concurrence pertinente, revue locale, documentation FR/EN, cas d'évaluation append-only et RETEX. |

Les tests utilisent des fournisseurs scriptés et des racines temporaires. Le
serveur utilisateur, la mission native en pause et les huit évaluations réelles
M6/M7 sont préservés. La suppression globale, les résumés, les sessions réelles
et l'acceptation indépendante restent hors de cette vérification locale.

## Résultat local

M15.1 à M15.4 sont réalisés et vérifiés localement : deux contre-exemples
conservés, cinq nouveaux tests dans un lot de 33 noms distincts, dont quatre
réexécutés avec détection de concurrence. Le parcours compilé CLI/API/navigateur
passe neuf contrôles : refus après désélection, reprise après sélection rétablie,
retrait conservant les originaux et nouveau refus sans invocation supplémentaire.

Voir [RESULTATS-MEMOIRE-11.md](RESULTATS-MEMOIRE-11.md) pour les preuves,
la documentation FR/EN et le RETEX. MEM-C18 reste PARTIAL : les références
versionnées sont contrôlées ; aucune purge générale ou qualification réelle
du fournisseur n'est déclarée. Le serveur utilisateur reste inchangé.
