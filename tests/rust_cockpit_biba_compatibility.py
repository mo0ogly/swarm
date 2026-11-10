#!/usr/bin/env python3
"""BIBA-IDENTITY-COMPAT — disponibilité cockpit contre gates effectifs.

Portée de toute conclusion possible (à lire avant d'interpréter un résultat) :
un miroir cockpit « start » visible, une preview ou un prepare n'impliquent
ni identité métier authentifiée ni autorisation d'exécuter. Les vrais gates
d'identité/instance/workspace et la frontière I0 Linux obligatoire restent
exigés et sont exercés ici tels quels, via la CLI publique d'un seul binaire
candidat explicite. Authentification métier/moniteur et isolation Linux sont
deux plans distincts ; ce fichier ne rend aucun verdict Biba global, ne prouve
ni authenticité cryptographique de l'agent ni médiation de toute lecture/
écriture Biba. Aucun droit, acceptation, remboursement ou rollback n'est jamais
déduit d'un JSON de statut, et aucune preuve n'est nettoyée.

Forme d'appel :
  python3 tests/rust_cockpit_biba_compatibility.py --binary <candidat> --output <neuf>

Chaque cas construit son propre Root Fixture privé via la CLI publique
seulement ; le fournisseur est le PROGRAM local fourni, qui écrit
model-received.json dans son répertoire courant — jamais un vrai modèle, un
réseau, un accès SQL, une base vivante, un retry, un watchdog ou un timeout.
Tous argv, stdout, stderr, refus JSON et leur SHA-256 exact sont retenus sous
--output (les fichiers bruts par la Fixture, les fils JSON parsés et les
registres SHA par ce test).

Le cas 1 est le témoin obligatoire : tâche bloquée, miroir start visible sans
extension/récupération, workspace I0 vide enfant du Root, preview valide,
prepare valide, puis un `agent run` qui doit réellement exécuter le stub dans
ce workspace et laisser une trace terminale enregistrée. Si le témoin échoue,
la chaîne s'arrête avec ses diagnostics : un refus commun n'est jamais accepté
comme sécurité prouvée.

Le contexte GLM initial omettait certains producteurs. Le parent a relu
cockpit_actions, swarm-biba-linux et agent_launch_preflight : le champ start
est vérifié structurellement et la substitution exige le code spécifique de
préflight dont le scope lie effectivement (device,inode), pas un refus arbitraire.
L'authentification cryptographique et la conformité Biba globale restent hors preuve.
"""
from pathlib import Path
import argparse
import json
import os
import sys
import traceback

try:
    from rust_agent_model_route import Fixture, digest, publish
except ImportError:  # importé depuis la racine du dépôt plutôt que tests/
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from rust_agent_model_route import Fixture, digest, publish

PROOF_FILENAME = 'model-received.json'
BLOCKER_TEXT = 'BIBA-IDENTITY-COMPAT: tâche bloquée pour le témoin cockpit; aucun départ autorisé.'
EXPECTED_BIBA_OVERLAP_CODE = 'biba-workspace-overlaps-root'
COCKPIT_UNPORTED_CODE = 'cockpit-task-projection-unported'
PREFLIGHT_EXPIRED_CODE = 'agent-execution-preflight-expired'


def parse_arguments():
    """Parse les deux seuls arguments Mandated : candidat explicite et sortie neuve."""
    parser = argparse.ArgumentParser(
        description='Régression BIBA-IDENTITY-COMPAT (aucun vrai fournisseur, aucun SQL, aucun retry).')
    parser.add_argument('--binary', required=True,
                        help='binaire CLI candidat explicite sous test')
    parser.add_argument('--output', required=True,
                        help='répertoire de preuves neuf (absent)')
    return parser.parse_args()


def publish_failure(output, name, wire):
    """Retenir un fil de refus parsé et retourner le SHA exact de ses octets."""
    publish(output / name, wire)
    return digest((output / name).read_bytes())


def surfaced_cause_codes(wire):
    """Collecter les codes de causes typées réellement remontés dans failure.causes.

    Seules les deux structures montrées par le matériel fourni sont lues :
    entrées textuelles (exemple de fil de refus public) et objets portant un
    champ code (le frame privé du gate projette des causes natives typées).
    Le failure.code de premier niveau est vérifié séparément, jamais mélangé ici.
    """
    causes = (wire.get('failure') or {}).get('causes')
    codes = set()
    if isinstance(causes, list):
        for cause in causes:
            if isinstance(cause, str):
                codes.add(cause)
            elif isinstance(cause, dict) and isinstance(cause.get('code'), str):
                codes.add(cause['code'])
    return codes


def state_probes(fixture, output, phase):
    """Relever work show et budget show en lecture sûre ; jamais de SQL ni de remboursement.

    Les erreurs tardives peuvent avoir modifié l'état, le budget ou les
    réservations : les deux fils sont enregistrés avant/après sans hypothèse
    de rollback et sans nettoyage des preuves.
    """
    work = fixture.ok(['work', 'show', fixture.work])
    budget = fixture.ok(['budget', 'show', fixture.work])
    publish(output / f'{fixture.label}-state-{phase}.json',
            dict(schema_version=1, phase=phase, work=work, budget=budget))
    return work, budget


def block_task_and_refresh(fixture):
    """Atteindre l'état bloqué par la CLI publique task update, puis rafraîchir la requête.

    La source publique accepte exactement schema_version, event_id,
    expected_revision, id, status et blocker — aucun champ reason non déclaré
    n'est jamais envoyé. Précondition documentée : révision 3 après l'init de
    la Fixture ; le blocage fait avancer le Work, puis request.expected_revision
    est rafraîchi depuis work show (work.revision), comme l'exige la recette.
    """
    before = fixture.ok(['work', 'show', fixture.work])
    assert before['work']['revision'] == 3, before['work']['revision']
    fixture.ok(['task', 'update', fixture.work], dict(
        schema_version=1,
        event_id='block-cockpit-witness',
        expected_revision=3,
        id='task',
        status='blocked',
        blocker=BLOCKER_TEXT,
    ))
    shown = fixture.ok(['work', 'show', fixture.work])
    assert shown['work']['revision'] == before['work']['revision'] + 1, shown['work']['revision']
    fixture.request['expected_revision'] = shown['work']['revision']
    return shown


def require_start_mirror(fixture, output):
    """Lire agent list : miroir start disponible, sans extension ni récupération.

    Le parent a vérifié cockpit_actions::project : le kind start doit être
    disponible et les kinds extend-attempt/authorize-recovery absents ; le
    tableau ne passe pas sur une simple recherche textuelle. Ce miroir reste
    une projection d'admissibilité, jamais une autorisation : tout lanceur
    revérifie fournisseurs, workspace, budget et identité dans sa transaction.
    """
    listed = fixture.ok(['agent', 'list', fixture.work])
    publish(output / f'{fixture.label}-agent-list.json', listed)
    assert listed['paused'] is False, listed['paused']
    assert listed['work']['revision'] == fixture.request['expected_revision'], listed['work']['revision']
    entry = listed['pilotage']['tasks']['task']
    assert entry['ready'] is True, entry
    actions = {a['kind']: a for a in listed['task_actions']['task']}
    assert actions['start']['disponible'] is True, actions['start']
    assert 'extend-attempt' not in actions and 'authorize-recovery' not in actions, actions
    return listed


def make_i0_workspace(fixture, name):
    """Créer le workspace I0 vide : enfant du Root, hors .swarm.

    La requête reçoit uniquement le chemin absolu — aucun booléen de contournement
    n'est deviné ni envoyé. L'inode créé est retourné comme témoin de base du
    cas : c'est la valeur que les gates effectifs épinglent au prepare.
    """
    workspace = fixture.root / name
    assert workspace.parent == fixture.root and workspace.name != '.swarm'
    assert not workspace.exists()
    os.mkdir(workspace, 0o700)
    assert workspace.is_dir() and not any(workspace.iterdir())
    inode = workspace.stat().st_ino
    fixture.request['workspace'] = str(workspace)
    assert Path(fixture.request['workspace']).is_absolute()
    return workspace, inode


def stub_absent_outside(fixture, allowed):
    """Exiger la preuve du stub uniquement là où l'exécution est prouvée.

    Le PROGRAM écrit model-received.json dans son répertoire courant ; toute
    autre occurrence sous le Root signale une exécution hors du workspace I0
    validé. Aucun SQL ni base vivante n'est ouvert pour ce contrôle.
    """
    intruders = [
        str(path.relative_to(fixture.root))
        for path in sorted(fixture.root.rglob(PROOF_FILENAME))
        if path != allowed
    ]
    assert not intruders, f'fichiers de preuve fournisseur inattendus: {intruders}'


def run_agent(fixture, output, tag):
    """Invoquer `agent run <id>` une seule fois et tout retenir, quel que soit l'issue.

    L'identité de lancement est l'event_id de la requête dans cette famille de
    fixtures (les tests fournis d'agent_execution_preflight observent la ligne
    queued 'session' pour la même requête). La forme argv exacte
    (`agent run <id>`, sans document d'entrée) suit le contrat de tâche ; si
    le dispatch du candidat diffère, les commands.json/stderr retenus ici sont
    la première preuve discriminante. Aucun retry, watchdog ou timeout.
    """
    agent_id = fixture.request['event_id']
    code, wire = fixture.call(['agent', 'run', agent_id])
    publish(output / f'{tag}.result.json', wire)
    state_probes(fixture, output, tag + '-after-call')
    return code, wire


def case_record(fixture, output, name, **fields):
    """Publier le registre du cas : chaque argv, exit et SHA exact des artefacts.

    La Fixture a déjà retenu le stdout/stderr brut de chaque appel et la liste
    argv complète ; ce registre lie chaque artefact à son SHA-256 pour que le
    parent rejoue les octets exacts. Aucun secret ni contenu SQL n'est copié.
    """
    calls = []
    for index, call in enumerate(fixture.calls, start=1):
        stem = f'{fixture.label}-{index:03}'
        entry = dict(
            argv=call['argv'],
            exit=call['exit'],
            stdout_sha256=digest((output / (stem + '.stdout')).read_bytes()),
            stderr_sha256=digest((output / (stem + '.stderr')).read_bytes()),
        )
        input_artifact = output / (stem + '.input.json')
        if input_artifact.is_file():
            entry['input_sha256'] = digest(input_artifact.read_bytes())
        calls.append(entry)
    publish(output / name, dict(
        schema_version=1,
        kind='biba-identity-compat-case-record',
        label=fixture.label,
        binary_sha256=digest(fixture.binary.read_bytes()),
        calls=calls,
        **fields,
    ))


def case1_witness(binary, base):
    """Cas 1 — témoin valide : la tâche bloquée s'exécute réellement dans le workspace I0.

    Établit, sur ce candidat, que le chemin compatible existe toujours :
    blocage public, miroir start visible sans extension/récupération, preview
    valide, prepare valide, puis un `agent run` avec le fournisseur local
    inerte. Critères non vacueux exigés : model-received.json à l'intérieur du
    workspace I0 (et nulle part ailleurs sous le Root), tentative réellement
    enregistrée dans le work, révision avancée, et retrait du miroir cockpit
    ensuite avec son refus explicite cockpit-task-projection-unported (la tâche
    a quitté la forme initiale supportée). Tout échec arrête la chaîne.
    """
    output = base / 'case1-witness'
    output.mkdir(parents=True, exist_ok=False)
    fixture = Fixture(binary, output, 'biba-compat-case1')
    block_task_and_refresh(fixture)
    require_start_mirror(fixture, output)
    workspace, inode = make_i0_workspace(fixture, 'biba-i0-witness')
    publish(output / 'case1-preview.json', fixture.prepare(preview=True))
    publish(output / 'case1-prepare.json', fixture.prepare())
    state_probes(fixture, output, 'before-run')
    code, wire = run_agent(fixture, output, 'case1-run')
    if code != 0:
        publish_failure(output, 'case1-run-refusal.json', wire)
        state_probes(fixture, output, 'after-refusal')
        case_record(fixture, output, 'case1-record.json',
                    status='failed', workspace=str(workspace), workspace_inode=inode,
                    conclusion='témoin refusé; chaîne arrêtée, aucun refus commun accepté comme sécurité prouvée')
        print('case1: run du témoin refusé; chaîne arrêtée', file=sys.stderr)
        return 'failed'
    assert wire['agent']['status'] == 'completed' and wire['task_accepted'] is False, wire
    proof = workspace / PROOF_FILENAME
    assert proof.is_file(), 'le fournisseur inert doit avoir exécuté dans le workspace I0 (model-received.json absent)'
    observed = json.loads(proof.read_text(encoding='utf-8'))
    assert observed['pid'] == wire['agent']['child_pid']
    assert observed['ppid'] == wire['agent']['supervisor_pid']
    assert observed['stdin_bytes'] > 0
    publish(output / 'case1-provider-proof.json', observed)
    stub_absent_outside(fixture, proof)
    work_after, _ = state_probes(fixture, output, 'after-run')
    tasks = {task['id']: task for task in work_after['work']['tasks']}
    attempts = tasks['task'].get('attempts') or []
    assert len(attempts) >= 1, 'le run doit laisser une tentative réellement enregistrée dans le work'
    assert work_after['work']['revision'] > fixture.request['expected_revision'], work_after['work']['revision']
    mirror_code, mirror_wire = fixture.call(['agent', 'list', fixture.work])
    publish(output / 'case1-post-run-agent-list.json', mirror_wire)
    assert mirror_code != 0, 'le miroir cockpit doit se retirer une fois la forme initiale quittée'
    assert mirror_wire['failure']['code'] == COCKPIT_UNPORTED_CODE, mirror_wire
    case_record(fixture, output, 'case1-record.json',
                status='passed', workspace=str(workspace), workspace_inode=inode,
                provider_proof_sha256=digest(proof.read_bytes()),
                conclusion='exécution réelle prouvée dans le workspace I0; tentative enregistrée et révision avancée observées')
    return 'passed'


def case2_root_default_workspace(binary, base):
    """Cas 2 — le workspaceRoot par défaut de la Fixture doit être refusé par le gate Biba.

    Même état bloqué et miroir visible, mais la requête garde le défaut de la
    Fixture : aucune clé workspace, c'est-à-dire l'ancien comportement
    workspaceRoot que le gate Biba attendu refuse avec
    biba-workspace-overlaps-root avant toute exécution fournisseur. La cause
    typée spécifique est exigée dans le refus structuré ou le résultat natif
    de tentative échouée (un CLI exit0 ne prouve pas un fournisseur démarré) ; si preview ou
    prepare refusent plus tôt, le cas est enregistré non qualifié — la garde
    Biba attendue n'a pas été exercée et aucune validation Biba n'est prétendue.
    Aucun model-received.json ne doit exister sous le Root.
    """
    output = base / 'case2-overlaps-root'
    output.mkdir(parents=True, exist_ok=False)
    fixture = Fixture(binary, output, 'biba-compat-case2')
    block_task_and_refresh(fixture)
    require_start_mirror(fixture, output)
    assert 'workspace' not in fixture.request, 'le défaut workspaceRoot de la Fixture doit rester intact'
    for stage in ('preview', 'prepare'):
        code, wire = fixture.call(['agent', stage, fixture.work], fixture.request)
        publish(output / f'case2-{stage}.result.json', wire)
        if code != 0:
            publish_failure(output, f'case2-{stage}-refusal.json', wire)
            state_probes(fixture, output, f'after-{stage}-refusal')
            case_record(fixture, output, 'case2-record.json',
                        status='unqualified-early-refusal', refused_at=stage,
                        observed_code=(wire.get('failure') or {}).get('code'),
                        biba_guard_exercised=False,
                        conclusion='refus avant le gate de run; la garde Biba attendue non exercée, aucune validation prétendue')
            print(f'case2: refus à {stage}, avant exec; garde Biba attendue non exercée', file=sys.stderr)
            return 'unqualified-early-refusal'
    state_probes(fixture, output, 'before-run')
    code, wire = run_agent(fixture, output, 'case2-run')
    if code == 0:
        # The CLI successfully returns an observed failed attempt. This is not
        # provider success: the native supervisor retains its private pre-exec
        # frame separately from the provider exit status.
        agent = wire['agent']
        native = agent['native_execution_result']
        assert agent['status'] == 'failed' and wire['task_accepted'] is False
        assert native['provider_may_have_started'] is False
        assert native['private_gate_invalid'] is False and native['kernel_reaped'] is True
        assert native['workspace_release_confirmed'] is True
        assert native['exit_code'] is None
        failure = native['pre_exec_failure']
        assert failure['kind'] == 'pre_exec_failure'
        assert failure['code'] == EXPECTED_BIBA_OVERLAP_CODE, failure
        surfaced = {label for item in failure['causes'] for label in item['labels']}
        assert EXPECTED_BIBA_OVERLAP_CODE in surfaced
    else:
        failure = wire.get('failure')
        assert isinstance(failure, dict) and failure.get('code') == EXPECTED_BIBA_OVERLAP_CODE, wire
        surfaced = surfaced_cause_codes(wire)
        if failure.get('causes'):
            assert EXPECTED_BIBA_OVERLAP_CODE in surfaced, surfaced
    stub_absent_outside(fixture, None)
    state_probes(fixture, output, 'after-refusal')
    case_record(fixture, output, 'case2-record.json',
                status='biba-guard-validated', biba_guard_exercised=True,
                observed_code=EXPECTED_BIBA_OVERLAP_CODE, surfaced_causes=sorted(surfaced),
                conclusion='refus Biba typé attendu avant exec; aucune exécution fournisseur; état relu sûrement, sans rollback supposé')
    return 'biba-guard-validated'


def case3_inode_replacement(binary, base):
    """Cas 3 — un workspace remplacé par un autre inode doit refuser avant le stub.

    Même montage témoin valide que le cas 1 (workspace I0 vide, preview valide,
    prepare valide) ; puis le répertoire workspace est remplacé par un autre
    inode via un rename retenu plus mkdir — le répertoire original est conservé
    sous un nom retenu, jamais supprimé. Un `agent run` doit alors refuser
    avant toute exécution fournisseur : le témoin inode/identité épinglé ne
    correspond plus. Le code agent-execution-preflight-changed est exigé après
    vérification du producteur réel qui lie device/inode au scope enregistré.
    Ce cas qualifie ce gate de préflight, pas un passage dans la sandbox Linux. Un refus que les
    sources fournies expliquent par le TTL de préparation
    (agent-execution-preflight-expired) ne peut pas passer pour la garde inode.
    Après refus spécifique, restaurer le vrai inode doit permettre au même
    intent de s'exécuter, sans reset ni nouveau droit.
    """
    output = base / 'case3-inode-replacement'
    output.mkdir(parents=True, exist_ok=False)
    fixture = Fixture(binary, output, 'biba-compat-case3')
    block_task_and_refresh(fixture)
    require_start_mirror(fixture, output)
    workspace, inode_before = make_i0_workspace(fixture, 'biba-i0-witness')
    publish(output / 'case3-preview.json', fixture.prepare(preview=True))
    publish(output / 'case3-prepare.json', fixture.prepare())
    assert not any(workspace.iterdir()), 'le prepare ne doit rien laisser dans le workspace témoin'
    state_probes(fixture, output, 'before-run')
    retained = fixture.root / 'biba-i0-witness-previous-inode'
    os.rename(workspace, retained)
    os.mkdir(workspace, 0o700)
    inode_after = workspace.stat().st_ino
    assert inode_after != inode_before, 'le remplacement doit être un vrai changement d inode'
    assert retained.is_dir(), 'le répertoire remplacé doit rester retenu comme preuve'
    publish(output / 'case3-inode-replacement.json', dict(
        schema_version=1, workspace=str(workspace), inode_before=inode_before,
        inode_after=inode_after, retained_path=str(retained)))
    code, wire = run_agent(fixture, output, 'case3-run')
    if code == 0:
        stub_files = [str(path.relative_to(fixture.root))
                      for path in sorted(fixture.root.rglob(PROOF_FILENAME))]
        state_probes(fixture, output, 'after-run')
        case_record(fixture, output, 'case3-record.json',
                    status='failed', inode_before=inode_before, inode_after=inode_after,
                    stub_files=stub_files,
                    conclusion='le retour CLI ne porte pas le refus de préflight spécifique attendu; résultat et état conservés')
        print('case3: refus de préflight attendu absent du retour CLI; ne pas déduire une exécution du seul exit0', file=sys.stderr)
        return 'failed'
    failure = wire.get('failure')
    assert isinstance(failure, dict) and isinstance(failure.get('code'), str) and failure['code'], wire
    observed_code = failure['code']
    # Actual producer binds device/inode into preflight.scope; inspect compares
    # that retained scope before any exec. Require that precise public refusal.
    assert 'agent-execution-preflight-changed' in ({observed_code} | surfaced_cause_codes(wire)), wire
    stub_absent_outside(fixture, None)
    state_probes(fixture, output, 'after-refusal')
    # Restore the exact retained inode, not a recreated lookalike, then let the
    # same public intent run. This is a changed precondition, never a retry of
    # the rejected workspace or a reset of budget/history.
    rejected = fixture.root / 'biba-i0-rejected-inode'
    workspace.rename(rejected)
    retained.rename(workspace)
    assert workspace.stat().st_ino == inode_before
    recovery_code, recovery = run_agent(fixture, output, 'case3-restored-run')
    assert recovery_code == 0 and recovery['agent']['status'] == 'completed', recovery
    assert recovery['agent']['id'] == fixture.request['event_id']
    assert recovery['task_accepted'] is False
    proof = workspace / PROOF_FILENAME
    observed = json.loads(proof.read_bytes())
    assert observed['pid'] == recovery['agent']['child_pid']
    assert observed['ppid'] == recovery['agent']['supervisor_pid']
    stub_absent_outside(fixture, proof)
    case_record(fixture, output, 'case3-record.json',
                recovery_same_intent=True, restored_original_inode=True,
                status='workspace-identity-refused', observed_code=observed_code,
                causal_code_qualified=True,
                qualified_boundary='retained_preflight_workspace_identity_before_exec',
                inode_before=inode_before, inode_after=inode_after,
                conclusion='refus spécifique avant exec sur inode remplacé; même intention exécutée après restauration du vrai inode')
    return 'workspace-identity-refused'


def safe_case(name, function, binary, base):
    """Exécuter un cas ; lier toute exception à un échec retenu et imprimé."""
    try:
        return dict(status=function(binary, base))
    except Exception:
        traceback.print_exc()
        print(f'{name}: échec non géré; artefacts bruts retenus sous --output', file=sys.stderr)
        return dict(status='failed', error='échec non géré; traceback sur stderr, artefacts retenus')


def main():
    """Dérouler la chaîne témoin-d'abord et imprimer la conclusion scopée.

    Code 0 du script exigé seulement si : le témoin du cas 1 est entièrement passé, le
    cas 2 a exercé le refus Biba attendu avec son code typé, et le cas 3 a
    observé le refus spécifique après remplacement puis la reprise du même intent
    après restauration de l’inode original. Tout
    résultat non qualifié ou échoué sort en 1 avec les raisons par cas ; rien
    n'est jamais retenté, annulé, remboursé ou nettoyé.
    """
    arguments = parse_arguments()
    binary = Path(arguments.binary).resolve(strict=True)
    base = Path(arguments.output)
    assert binary.is_file(), f'binaire candidat introuvable: {binary}'
    binary_before = digest(binary.read_bytes())
    base.mkdir(parents=True, exist_ok=False, mode=0o700)
    outcomes = dict(case1=safe_case('case1', case1_witness, binary, base))
    if outcomes['case1']['status'] == 'passed':
        outcomes['case2'] = safe_case('case2', case2_root_default_workspace, binary, base)
        outcomes['case3'] = safe_case('case3', case3_inode_replacement, binary, base)
    else:
        stopped = dict(status='not-run',
                       reason='témoin du cas 1 échoué ou refusé; chaîne arrêtée, aucune revendication de sécurité')
        outcomes['case2'] = dict(stopped)
        outcomes['case3'] = dict(stopped)
    qualified = (
        outcomes['case1']['status'] == 'passed'
        and outcomes['case2']['status'] == 'biba-guard-validated'
        and outcomes['case3']['status'] == 'workspace-identity-refused'
    )
    assert digest(binary.read_bytes()) == binary_before, 'candidate bytes changed during verification'
    summary = dict(
        schema_version=1,
        kind='biba-identity-compat-summary',
        binary_sha256=digest(binary.read_bytes()),
        qualified=qualified,
        outcomes=outcomes,
        scope=('compatibilité du cockpit avec les gates effectifs identité/instance/workspace et la '
               'frontière I0 obligatoire seulement; aucune identité authentifiée, aucune autorisation '
               'd exécution, aucun verdict Biba global, aucune authenticité cryptographique, aucun droit '
               'déduit d un JSON de statut'),
    )
    publish(base / 'biba-identity-compat-summary.json', summary)
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 0 if qualified else 1


if __name__ == '__main__':
    sys.exit(main())
