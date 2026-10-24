#!/usr/bin/env python3
"""Qualify planning snapshots or financial configuration on effective inputs.

Pricing also covers producer-initialized JSON and local catalogue commands.
Read-only compiler inventories, isolated tests and archived same-session review.
No global watchdog, providers, live roots, server install or independent acceptance.
Source/profile edits during execution invalidate this result. Fresh output only.
"""
import argparse
from collections import Counter
import difflib
import json
import os
from pathlib import Path
import sys
import tarfile

from qualification_inputs import collect
from recorded_process import capture
from review_increment import publish, digest, candidates, retained_parity, go_test_inventory

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT/'docs/plans/migration-rust/evidence/20261019-66/documentation-final/inputs-after.json'


def qualify(output, parity, component):
    """Freeze inputs, run checks and require exact-byte four-axis review.

    The selected component parity binaries must rebuild identically. Missing review
    returns3 (REVIEW_REQUIRED), never a successful qualification. Compilers/tools and
    source archives are retained. PASS means this component only; caller owns any
    future integration, transaction freshness, authority, reservation and cutover.
    """
    output.mkdir(mode=0o700)
    baseline, expected_count, probe, fixture_name = {
        'budgets': (BASE,49,'planning_budget_probe','native-budget-fixture'),
        'rejection': (ROOT/'docs/plans/migration-rust/evidence/20261019-67/qualified01/inputs-after.json',62,'planning_rejection_probe','native-rejection-fixture'),
        'pricing': (ROOT/'docs/plans/migration-rust/evidence/20261019-69/qualified02/inputs-after.json',101,'pricing_probe','native-pricing-fixture'),
        'financial': (ROOT/'docs/plans/migration-rust/evidence/20261019-68/qualified01/inputs-after.json',67,'financial_budget_probe','native-financial-fixture'),
    }[component]
    reference=dict(schema_version=1,path=str(parity.relative_to(ROOT)),result_sha256=digest(parity/'result.json'))
    parity=retained_parity(ROOT,reference)
    observations=json.loads((parity/'result.json').read_bytes())
    if observations.get('status')!='PASS' or observations.get('count')!=expected_count:
        raise ValueError('parity-prerequisite-invalid')
    publish(output/'parity-reference.json',reference)
    commands=[]
    env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'rust/target/r25'),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',RUSTDOCFLAGS='-D warnings',SWARM_MIGRATION_REVIEW_BASELINE=str(baseline.relative_to(ROOT)))
    def run(label,args,data=None):
        """Record original finite output and refuse continuation on any failed check."""
        args=list(map(str,args))
        result=capture(args,cwd=ROOT,env=env,data=data,stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        commands.append(dict(label=label,args=args,exit=result.returncode))
        publish(output/(label+'.command.json'),commands[-1])
        print(label,result.returncode,flush=True)
        if result.returncode:
            raise RuntimeError(label+': failed; original outputs retained')
        return result.stdout
    try:
        extras=[ROOT/p for p in ['AGENTS.md','CLAUDE.md','docs/plans/migration-rust/PLAN.md','docs/architecture/RUST-MIGRATION.md','docs/en/RUST-MIGRATION.md','rust/README.md','docs/plans/migration-rust/REPLAYS.md','docs/en/RUST-MIGRATION-REPLAYS.md','tools/verification/README.md','tools/rust-migration/README-planning-budgets.md']]
        extras += [ROOT/name for name in ['tools/verification/recipe.py','tools/verification/test_recipe.py','tools/verification/recipes/runner.json','tools/verification/recipes/planning-budgets.json','tools/verification/recipes/planning-rejection.json','tools/verification/recipes/financial-budget.json','tools/verification/recipes/pricing-accounting.json','tools/verification/recipes/pricing-native.json','.claude/skills/pricing-discovery/SKILL.md','.claude/skills/pricing-discovery/agents/openai.yaml','docs/BUDGETS.md','docs/en/BUDGETS.md','tools/verification/close_qualification.py','tools/verification/test_close_qualification.py','tools/verification/QUALIFICATION-CLOSURE.md'] if (ROOT/name).exists()]
        before,snapshot,registry,close=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine','./tools/rust-migration/work-layout'],allow_root=True,extra_files=extras)
        cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline']
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
        run('build',['cargo','build',*cargo,'--workspace','--all-features','--all-targets'])
        run('clippy',['cargo','clippy',*cargo,'--workspace','--all-features','--all-targets','--','-D','warnings'])
        tests=['cargo','test',*cargo,'--workspace','--all-features']
        actual_raw=run('native-tests',tests).decode()
        listed_raw=run('native-inventory',tests+['--','--list']).decode()
        actual=Counter(l[5:-7] for l in actual_raw.splitlines() if l.startswith('test ') and l.endswith(' ... ok'))
        listed=Counter(l[:-6] for l in listed_raw.splitlines() if l.endswith(': test'))
        if not actual or actual!=listed or ' ... ignored' in actual_raw:
            raise ValueError('native-test-inventory-mismatch')
        publish(output/'native-test-names.json',list(actual.elements()))
        defaults=['cargo','test',*cargo,'-p','swarm-cli','--test','cli','--test','gate','--test','work_list','--test','work_view','--test','planning','--test','agent_workflow','--test','planning_report','--test','budget','--test','pricing']
        raw=run('default-cli-tests',defaults).decode()
        inventory=run('default-cli-inventory',defaults+['--','--list']).decode()
        default=Counter(l[5:-7]for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
        if not default or default!=Counter(l[:-6]for l in inventory.splitlines()if l.endswith(': test')):
            raise ValueError('default-cli-inventory-mismatch')
        publish(output/'default-cli-test-names.json',list(default.elements()))
        run('rustdoc',['cargo','doc',*cargo,'--workspace','--all-features','--no-deps'])
        run('runner-tests',['python3','-m','unittest','discover','-s','tools/verification','-p','test_recipe.py','-v'])
        run('review-guards',['python3','-m','unittest','discover','-s','tools/rust-migration','-p','test_review_increment.py','-v'])
        go_cases=run('go-budgets',['go','test','./internal/engine','-v','-count=1','-timeout=0','-run','^(TestCostSummary.*|TestCostText.*|TestBudget.*|TestPricing.*|TestCommandErrorPreservesPrivateCause)$' if component=='pricing' else '^(TestBudgetPublicCommands|TestBudgetReservationReplayAndExhaustion|TestBudgetExposesReportedCostOnlyWhenKnown)$' if component=='financial' else '^TestPlanningScopeBudgetRejectsNegativeFields$'])
        go_pattern=commands[-1]['args'][-1]
        go_list=run('go-test-inventory',['go','test','./internal/engine','-list',go_pattern,'-timeout=0'])
        go_names=go_test_inventory(go_cases.decode(),go_list.decode())
        publish(output/'go-test-names.json',go_names)
        run('go-vet',['go','vet','./cmd/swarm','./internal/engine'])
        run('methods',['python3','tools/agent-workflows/check.py'])
        run('diff',['git','diff','--check'])
        parity=retained_parity(ROOT,reference)
        observations=json.loads((parity/'result.json').read_bytes())
        if observations['status']!='PASS' or observations['count']!=expected_count:
            raise ValueError('parity-prerequisite-invalid')
        run('production-profile-build',['cargo','build',*cargo,'-p','swarm-cli','--bin','swarm-rust'])
        run('go-profile-build',['go','test','-c','-o',output/'go-planning','./internal/engine'])
        if digest(output/'go-planning')!=observations['binaries']['go-planning'] or digest(ROOT/'rust/target/r25/debug/swarm-rust')!=observations['binaries']['native-production-cli']:
            raise ValueError('parity-production-inputs-changed')
        tree=run('production-features',['cargo','tree',*cargo,'-p','swarm-cli','--format','{p} features=[{f}]','--edges','normal'])
        if b'test-fixtures' in tree:raise ValueError('fixture-feature-in-production')
        run('fixture-profile-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--example',probe])
        if digest(ROOT/('rust/target/r25/debug/examples/'+probe))!=observations['binaries'][fixture_name]:
            raise ValueError('parity-fixture-inputs-changed')
        publish(output/'inputs-after.json',snapshot())
        if before!=snapshot():raise ValueError('source-changed')
        print('REVIEW_READY',output,flush=True)
        # Semantic decisions are retained separately, never inferred from candidates.
        review_path=output/'source-review.json'
        if not review_path.exists():
            publish(output/'pending-review.json',dict(status='REVIEW_REQUIRED',native_tests=sum(actual.values()),default_cli_tests=sum(default.values()),go_regression_tests=len(go_names),inputs=len(before),registry_packages=len(registry)))
            return 3
        close()
        if before!=snapshot():raise ValueError('source-changed-after-closure')
        publish(output/'result.json',dict(component_status='PASS',status='PARTIAL',component=component,pricing_catalogue_cli_ported=component=='pricing',native_tests=sum(actual.values()),default_cli_tests=sum(default.values()),go_regression_tests=len(go_names),go_native_comparisons=expected_count,inputs=len(before),registry_packages=len(registry),source_stable=True,production_cli_fixture_features=False,claim_ported=False,identity=False,financial_reservation_api_verified=component=='financial',budget_configuration_ported=component=='financial',claim_financial_integration=False,budget_views_ported=False,native_http=False,deployment=False,independent_review=False,global_watchdog=None))
        print('PASS snapshot component; PARTIAL migration',flush=True)
        return 0
    except BaseException as error:
        publish(output/'failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__))
        raise


def main():
    """Accept explicit fresh proof and existing parity paths; never replace history."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--parity',type=Path,required=True)
    parser.add_argument('--component',choices=['budgets','rejection','financial','pricing'],default='budgets')
    args=parser.parse_args()
    return qualify(args.output.resolve(),args.parity.resolve(),args.component)


if __name__=='__main__':raise SystemExit(main())
