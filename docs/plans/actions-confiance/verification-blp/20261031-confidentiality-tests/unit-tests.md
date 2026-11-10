# Preuves tests unitaires swarm-confidentiality — 31 octobre 2026

Commande : `TMPDIR=/home/fpizzi/Documents/ChatGPT/swarm/.swarm-fixtures cargo test -p swarm-confidentiality --lib`
(depuis `rust/`) ; rtk condense la sortie, sortie brute via `rtk proxy`.

Résultat : **10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out**.

Tests nommés (liste `-- --list`) :
- tests::catalog_and_labels_reject_unknowns_duplicates_and_malformed_input
- tests::dominance_exhaustive_independent_matrix
- tests::fixed_domain_does_not_use_maximum_clearance_for_later_reads
- tests::join_preserves_all_exposed_compartments_and_maximum_level
- tests::no_read_up_no_write_down_and_no_compartment_loss
- tests::policy_change_and_revocation_require_new_execution
- tests::same_level_distinct_compartments_are_incomparable
- tests::successor_preserves_historical_catalog_semantics
- tests::unknown_or_empty_context_never_becomes_public
- tests::unrecognized_principal_learns_no_object_status_and_errors_are_content_free

Portée : octets actuels incluant la liaison de révision de ExecutionDomain
(limite du PASS hérité antérieur levée). Preuves data-only : n'établit ni
producteur protégé, ni acteur authentifié, ni confinement OS, ni transport.

Correctifs inclus avant ce PASS :
1. E0308 CompactJson::new (&str vs String) — correction par from_utf8 déjà
   écrite par le prédécesseur, recompilée et vérifiée ici.
2. Routage HTTP service/trust.rs : expression `|` invalide dans la branche
   GET/HEAD remplacée par la sémantique originale (GET/HEAD → "show") ;
   opérations BLP ajoutées à la liste matches! des POST autorisés.
