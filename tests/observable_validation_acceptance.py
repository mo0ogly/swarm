"""Targeted candidate build, Go contracts, real browser in isolated synthetic stores."""
import os,pathlib,subprocess,tempfile

def run(args,env=None):
    print('COMMAND',args,flush=True)
    subprocess.run(args,check=True,env=env)

run(['go','test','.','-run','^(TestEvidenceContractCurrentVerdictHistoryAndMeasuredTopFive|TestEvidenceContractExecutedControlAndStaleness|TestEvidenceContractSameTruthCLIAndWeb|TestValidationRecheck.*)$','-count=1','-timeout=120s'])
run(['node','tests/evidence_contract_test.cjs'])
run(['npm','run','test:i18n'])
with tempfile.TemporaryDirectory(prefix='swarm-observable-') as tmp:
    binary=str(pathlib.Path(tmp)/'swarm')
    run(['sh','build.sh',binary])
    env=dict(os.environ,SWARM_T2_RECHECK_EVIDENCE=tmp)
    run(['go','test','.','-run','^TestValidationRecheckPreservesAttemptAndRequiresCompletedProducer$','-count=1'],env)
    run(['node','tests/t3_validation_ui.cjs',binary,tmp])
print('PASS observable validation and public recheck: FR/EN, dark/light, isolated stores; synthetic evidence, not provider autonomy.',flush=True)
