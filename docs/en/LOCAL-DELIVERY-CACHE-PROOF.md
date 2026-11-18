# Local delivery qualification evidence

The launcher validates retained proof before reusing an unchanged image. `_read_prepared` shares explicit `--prepared-build` checks: schema, digest, normal profile without fixtures, metadata and exact input archive. Missing files require a fresh build; incoherent evidence refuses before stopping the server. Compiler success or a surviving binary alone does not establish qualification.

Explicit selection does not rebuild newer sources and refuses version downgrades. Primary and cleanup causes remain distinct. This grants no publication or data migration authority. Retain evidence outside the cache before cleanup.

Recipes: `local-build-cache-proof.json` and `tests/test_swarm_prepared_build.py`. The latter requires `SWARM_TEST_BUILD_STATE` for its isolated real lifecycle; a missing prerequisite is a skip, never PASS. The permanent recipe covers three cache cases, seven stability guards and five selection/lifecycle checks. No independent or global acceptance follows.

The qualification area requests Unix mode 0700. A mkdir or chmod error propagates
with its original errno and object before any native child; no cleanup, retry or
alternative location follows refusal. The area may already exist or have been
created before failure: no rollback is inferred. Two inert guards in
`tests/test_swarm_version_fixture_area.py` cover simulated checkout and installed
launcher layouts, with a valid control, refusal reaching `verify_candidate` and
healthy recovery. Maintained recipe:
`tools/verification/recipes/local-delivery-fixture-area.json`. Existing source/home
selection stays unchanged; this check does not qualify use of the persisted
`runtime_temporary_directory` setting, a build or an installation.
