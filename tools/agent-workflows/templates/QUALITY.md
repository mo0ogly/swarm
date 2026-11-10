# Authorized implementation quality
Stay within assigned role, rights and scope.
For every increment, review exact changed bytes:
- Policies: classify literals/defaults/fallbacks (test, protocol, technical, operator). Persist operator choices; trace Admin/API/CLI to real use.
- Docs: update FR/EN docs and public/nontrivial function contracts: purpose, inputs/preconditions, outputs, errors, side effects, limits.
- Errors: keep causes and safe diagnostics; check state, close/flush, cancellation, recovery. Refusal must not become empty success.
Verify actual profiles/features/binary; separate production and fixtures. Bind proofs to inputs; missing checks stay open. On anomaly, replan, correct within rights, verify and resume. Keep history and improvements.
