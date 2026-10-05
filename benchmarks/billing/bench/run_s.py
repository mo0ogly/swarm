#!/usr/bin/env python3
"""Condition S hiérarchique : préparation et règlement conduits par Swarm, fautes injectées de l'extérieur.

Chaque exécution a sa propre racine d'exécution, séparée en deux dossiers :
- `swarm/` : racine Swarm (`swarm init`) et espace de travail partagé des agents ;
- `bank/` : grand livre, jeton de règlement, `api.port`, `api.log` de l'API de paiement.
E1 par construction, pour le fournisseur scripté seulement : le jeton n'est ni dans l'espace des
agents ni passé au fournisseur `banc-prepare` ; seul `banc-settle` reçoit son chemin. Un agent réel
n'est pas isolé du système de fichiers : il pourrait lire `../bank`. Les marqueurs de fautes restent dans
`faults/` de la racine d'exécution. Chaque exécution a aussi son API et un
conducteur `web` sans adresse : il écoute sur 127.0.0.1:0 (web_server.go, serveWeb), donc aucun
conflit de port entre conducteurs ni entre exécutions.

Organisation (docs/benchmarks/billing/plan-implementation.md, « Condition S hiérarchique ») :
un responsable de mission scripté (planner_fixture) possède le périmètre racine ; le travail a deux
critères, donc deux exigences, chacune avec son contrôle déclaré à `planning enable` :
`req-1` → check_lot sur `docs/prepare.md`, `req-2` → verify_settlement. Les tâches `prepare` puis
`settle` sont créées par une décision du responsable (`planning claim` puis `planning decide`).
Les deux tâches s'exécutent dans la racine (espace partagé) : en espace propre, le moteur ne
revalide pas un rapport absent de `racine/docs` (observations.md, § 1).

Les contrôles s'exécutent dans la racine du projet (automatic_validation.go : runValidationControl
avec s.root) : `docs/prepare.md` y désigne le livrable remis. Ils peuvent s'exécuter plusieurs fois
par tentative (observations.md, § 5) ; check_lot et verify_settlement sont en lecture seule.

Succès déclaré : `settle` acceptée ET périmètre racine clos par le responsable.

Un échec du banc lui-même (CLI swarm en échec pendant l'installation, API non démarrée, conducteur
mort, contrat du moteur différent de celui observé, panne du responsable ou du vérificateur, budget
de planification épuisé, fournisseur sorti avec un code qu'aucune faute prévue n'explique,
`settle` acceptée sans clôture du périmètre) lève harness.BenchError : c'est une ERREUR,
jamais une mesure. Un dépassement de S_RUN_TIMEOUT_S donne DÉLAI. Le nettoyage arrête mission,
agents, conducteurs et API dans tous les cas, puis tue tout processus restant dont la ligne de
commande désigne la racine d'exécution.
"""
import json
import os
import sqlite3
import signal
import subprocess
import sys
import time
import uuid
from datetime import datetime
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, markers, planner_fixture

SWARM = harness.SWARM_BIN
ACTIVE = {"queued", "starting", "running", "stopping"}   # agents_store.go : activeAgent
PENDING = {"running", "submitted"}                       # tâche dont le moteur n'a pas fini de décider
LISTENING = "Cockpit local : http://127.0.0.1:"          # web_server.go : annonce d'écoute de serveWeb
LOT_REL = "docs/prepare.md"                              # livrable de prepare, relatif à la racine
PLAN_MAX_ATTEMPTS = 2                                    # planning.go : PlanMaxAttempts d'une tâche planifiée
CRITERIA = ("lot conforme au grand livre", "chaque facture payée une fois au montant exact")
# Codes de sortie du fournisseur qu'une faute prévue explique, en plus de 0. Fixés d'après l'exploration
# de la tâche 9 (docs/benchmarks/billing/observations.md, « Exploration des fautes en S ») : 137 sous F3
# (plantage injecté), 4 sous F4 (empreinte du lot). F6 : l'arrêt par la garde (status interrupted,
# stop_kind garde, code -1) est traité à part. Jamais observés, donc ERREUR : 3 sous F1/F5 (la
# coupure de F1 est absorbée par les réessais du règlement, le 503 de F5 touche un contrôle, pas le
# fournisseur) et 2, refus métier de l'API, qu'aucune faute en S ne provoque.
FAULT_EXIT = {"F3": {config.EXIT_CRASH}, "F4": {config.EXIT_DIGEST}, "F4e": {config.EXIT_DIGEST}}
HOLD_CRITERION = "verrou de séquencement levé après la modification du lot"   # F4e seulement


def pending_events(planning, scope_id):
    """Événements du périmètre non encore pris en compte par une décision (planning.go : PlanningEvent.Decision)."""
    return [e for e in planning.get("inbox") or [] if e.get("scope") == scope_id and not e.get("decision")]


def infrastructure_fault(work, agents, fault):
    """Motif d'une panne qui interdit toute mesure, ou None.

    Champs de `work show` (planning.go : PlanningState ; independent_review.go : ReviewerConfig) :
    - `planning.failure` : passe de planification suspendue (planning_runner.go : planningFailure) ;
    - `planning.reviewer.failure` : vérificateur indépendant en échec (independent_review_runtime.go) ;
    - `planning.decisions` ≥ `max_decisions` ou `activations` ≥ `max_activations` : planningStep
      s'arrête sans rien consigner (planning_runner.go, garde d'entrée) ; panne si un événement attend.
    Agents (agents_store.go : Agent.ExitCode, `exit_code`) : un code ni nul ni expliqué par la faute.
    """
    planning = work.get("planning") or {}
    if planning.get("failure"):
        return f"planification suspendue : {planning['failure']}"
    reviewer = planning.get("reviewer") or {}
    if reviewer.get("failure"):
        return f"vérificateur indépendant en échec : {reviewer['failure']}"
    waiting = [s["id"] for s in planning.get("scopes") or []
               if s.get("state") != "closed" and pending_events(planning, s["id"])]
    if waiting:
        for used, limit, label in (("decisions", "max_decisions", "décisions"),
                                   ("activations", "max_activations", "activations")):
            if planning.get(limit) and planning.get(used, 0) >= planning[limit]:
                return f"budget de {label} épuisé ({planning[used]}/{planning[limit]}) avec événements en attente : {waiting}"
    allowed = {0} | FAULT_EXIT.get(fault, set())
    wrong = [(a.get("id"), a["exit_code"]) for a in agents
             if a.get("exit_code") is not None and a["exit_code"] not in allowed and not budget_stop(a, fault)]
    if wrong:
        return "fournisseur sorti avec un code inattendu : " + ", ".join(f"{i} code {c}" for i, c in wrong)
    return None


BUDGET_REASON = "Limite d'appels d'outils atteinte"   # loop_guard.go (raison), agents_process.go (activité)


def budget_stop(agent, fault):
    """F6 : seul l'arrêt de `prepare` par la garde pour le budget d'appels est attendu.

    agents_process.go : à l'arrêt par la garde, status `interrupted`, stop_kind `garde`, activité
    « <raison> ; fin du processus confirmée », la raison venant de loop_guard.go.
    """
    return (fault == "F6" and agent.get("task_id") == "prepare" and agent.get("status") == "interrupted"
            and agent.get("stop_kind") == "garde" and (agent.get("activity") or "").startswith(BUDGET_REASON))


def settle_outcome(agents, fault):
    """Lancements du règlement et, sous F4, qui a arrêté le paiement.

    `agent list` renvoie le plus récent d'abord (agents_store.go : ORDER BY rowid DESC) : les codes sont
    remis dans l'ordre des lancements. `stopped_by` (F4 seulement) : `engine` si settle n'a jamais été
    lancée (fraîcheur des preuves avant départ), `settlement` si le règlement est sorti en 4 (empreinte),
    `none` sinon.
    """
    codes = [a.get("exit_code") for a in reversed(agents) if a.get("task_id") == "settle"]
    stopped = None
    if fault in ("F4", "F4e"):
        stopped = "engine" if not codes else ("settlement" if config.EXIT_DIGEST in codes else "none")
    return {"settle_launched": bool(codes), "settle_exit_codes": codes, "stopped_by": stopped}


def epoch(stamp):
    """Horodatage Go RFC3339Nano (UTC) en secondes depuis l'époque ; None si absent."""
    return datetime.fromisoformat(stamp).timestamp() if stamp else None


def f4e_order(accepted_at, tampered_at, settle_started):
    """F4e : acceptation de prepare < modification du lot < départ de settle (ou aucun départ)."""
    accepted, started = epoch(accepted_at), epoch(settle_started)
    proven = (accepted is not None and tampered_at is not None and accepted < tampered_at
              and (started is None or tampered_at < started))
    return {"proven": proven, "prepare_accepted_at": accepted, "tampered_at": tampered_at, "settle_started_at": started}


def f4_order(settle_started, tampered_at):
    """F4 : le lot est modifié après le départ de settle (sinon ce serait F4e, ou aucune faute)."""
    started = epoch(settle_started)
    proven = started is not None and tampered_at is not None and started < tampered_at
    return {"proven": proven, "settle_started_at": started, "tampered_at": tampered_at}


WEB_CONDUCTOR = "serveur web"   # mission.go : missionLoop, source du bail d'un conducteur `web`


def f7_order(takeover_at, settle_started, launcher, old_holder, new_holder, new_source):
    """F7 : settle lancée après la prise de main, par le nouveau détenteur du bail.

    `launcher` : `conductor_id` du départ de settle (Launch, agents_store.go, persisté dans
    agents.request) ; `old_holder`/`new_holder` : `mission_supervision.conductor_id` avant le gel et
    après la prise de main (mission_supervision.go). automaticLaunchGuard (mission.go) n'accepte un
    départ que du détenteur vivant du bail. Le nouveau détenteur doit être un conducteur `web`
    (`mission_supervision.source` = « serveur web ») : un superviseur prend aussi des baux éphémères
    « release-conductor-… », de source « libération de ressource » (dispatcher.go :
    dispatchAfterSettle). Le banc ne lance que deux conducteurs `web` : un détenteur `web` différent
    de l'ancien est donc le second.
    """
    started = epoch(settle_started)
    proven = (takeover_at is not None and started is not None and takeover_at < started
              and new_holder is not None and new_holder != old_holder and new_source == WEB_CONDUCTOR
              and launcher == new_holder)
    return {"proven": proven, "takeover_at": takeover_at, "settle_started_at": started,
            "settle_conductor": launcher, "holder_before": old_holder, "holder_after": new_holder,
            "holder_after_source": new_source}


def engine_stop_consistent(stopped_by, validation_state):
    """« Arrêté par le moteur » exige que la preuve de prepare soit réellement périmée (`stale`)."""
    return stopped_by != "engine" or validation_state == "stale"


def state_rows(root, query, args=()):
    """Lecture seule de la base Swarm, pour les seuls faits que la CLI n'expose pas (preuves F7).

    Ouverte en `mode=ro` : le banc n'écrit jamais dans `.swarm/state.db`.
    """
    conn = sqlite3.connect(f"file:{Path(root) / '.swarm' / 'state.db'}?mode=ro", uri=True, timeout=5)
    try:
        return conn.execute(query, args).fetchall()
    finally:
        conn.close()


def supervision_holder(root, wid):
    """Détenteur du bail de supervision : (mission_supervision.conductor_id, source), ou (None, None)."""
    rows = state_rows(root, "SELECT conductor_id, source FROM mission_supervision WHERE work_id=?", (wid,))
    return tuple(rows[0]) if rows else (None, None)


def settle_launcher(root, wid):
    """`conductor_id` du premier départ de settle, lu dans la requête de lancement persistée."""
    rows = state_rows(root, "SELECT request FROM agents WHERE work_id=? AND task_id='settle' ORDER BY rowid LIMIT 1",
                      (wid,))
    return json.loads(rows[0][0]).get("conductor_id") if rows else None


def prepare_validation_state(sw, wid):
    """Fraîcheur réelle de prepare : `mission status`, tasks[].result.validation_state (result_presentation.go)."""
    tasks = sw.cli(["mission", "status", wid]).get("tasks") or []
    return next(((t.get("result") or {}).get("validation_state") for t in tasks if t.get("id") == "prepare"), None)


def order_proof(fault, work, agents, faults, root=None, wid=None):
    """Preuve d'ordre exigée par F4, F4e et F7 ; None pour les autres fautes."""
    starts = sorted(a["started"] for a in agents if a.get("task_id") == "settle" and a.get("started"))
    first = starts[0] if starts else None
    found = markers.read_all(faults)
    if fault == "F4":
        return f4_order(first, found.get("lot-tampered", {}).get("at"))
    if fault == "F4e":
        prepare = next((t for t in (work or {}).get("tasks", []) if t["id"] == "prepare"), {})
        validation = prepare.get("automatic_validation") or {}
        accepted = validation.get("at") if validation.get("state") == "accepted" else None
        return f4e_order(accepted, found.get("lot-tampered-early", {}).get("at"), first)
    if fault == "F7":
        stall = found.get("owner-stalled", {})
        launcher = settle_launcher(root, wid) if first else None
        return f7_order(stall.get("takeover_at"), first, launcher, stall.get("holder_before"),
                        stall.get("holder_after"), stall.get("holder_after_source"))
    return None


def planning_busy(work):
    """Passe de planification en cours ou due : détenteur présent, ou événement en attente sur un périmètre ouvert.

    planning_runner.go : planningStep choisit un périmètre non clos, sans bail actif, qui a un événement
    sans décision ; le détenteur (`holder`) est posé au claim et effacé à la décision.
    """
    planning = work.get("planning") or {}
    return any(s.get("state") != "closed" and (s.get("holder") or pending_events(planning, s["id"]))
               for s in planning.get("scopes") or [])


class Swarm:
    def __init__(self, binary, root):
        self.binary, self.root = str(binary), root

    def cli(self, args, data=None):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args]
                           + (["--input", "-"] if data is not None else []),
                           input=json.dumps(data) if data is not None else None,
                           text=True, capture_output=True, timeout=config.S_CLI_TIMEOUT_S)
        if p.returncode:
            raise RuntimeError(f"swarm {' '.join(args[:2])} : code {p.returncode} : {(p.stdout + p.stderr).strip()}")
        try:
            return json.loads(p.stdout) if p.stdout.strip() else None
        except ValueError as e:   # sortie non JSON : erreur lisible, rattrapée par le nettoyage comme les autres
            raise RuntimeError(f"swarm {' '.join(args[:2])} : sortie non JSON ({e}) : "
                               f"{p.stdout.strip()[:config.ERROR_TEXT_MAX]!r} {p.stderr.strip()[:config.ERROR_TEXT_MAX]!r}") from e

    def mutate(self, args, work, **fields):
        out = self.cli(args, dict(schema_version=1, event_id=uuid.uuid4().hex,
                                  expected_revision=work.get("revision", 0), **fields))
        return out.get("work", out)

    def work(self, wid):
        out = self.cli(["work", "show", wid])
        return out.get("work", out)

    def agents(self, wid):
        return [x.get("agent", x) for x in self.cli(["agent", "list", wid])["agents"]]

    def conductor(self, log):
        return subprocess.Popen([self.binary, "--root", str(self.root), "web"], stdout=log, stderr=subprocess.STDOUT)


def profile(root, provider):
    return {"provider": provider, "role": "worker", "workspace": str(root), "capture_output": True,
            "timeout_seconds": 120}


def root_scope(work):
    return next(s for s in work["planning"]["scopes"] if s["id"] == "root")


def control(cid, script, api_url, *extra, why):
    # Programme `python3` (liste blanche validationPrograms) ; répertoire : la racine du projet.
    return {"id": cid, "command": ["python3", harness.SCRIPTS[script], "--api", api_url, *extra],
            "criteria": [1], "justification": why, "timeout_seconds": 15}


def setup_work(sw, run_dir, api_url, fault, prepare_provider):
    criteria = list(CRITERIA) + ([HOLD_CRITERION] if fault == "F4e" else [])
    w = sw.mutate(["work", "create"], {}, title="Banc de facturation",
                  objective="Payer chaque facture due une seule fois, au montant exact",
                  scope="banc synthétique isolé", criteria=criteria, next="organiser")
    wid = w["id"]
    checks = {
        "req-1": [control("check-lot", "check_lot", api_url, "--lot", LOT_REL,
                          why="Compare chaque ligne du lot remis au grand livre : facture, montant, "
                              "bénéficiaire, plafond, complétude.")],
        "req-2": [control("verify-settlement", "verify_settlement", api_url,
                          why="Relit les paiements du grand livre : aucun doublon, aucun écart, "
                              "aucune facture impayée.")],
    }
    if fault == "F4e":
        checks["req-3"] = [{"id": "verrou-rapport", "command": ["python3", "-c",
                            "import pathlib,sys; sys.exit(0 if pathlib.Path('docs/verrou.md').read_text().strip() else 1)"],
                            "criteria": [1], "justification": "Le verrou de séquencement a remis un rapport non vide.",
                            "timeout_seconds": 15}]
    w = sw.mutate(["planning", "enable", wid], w, provider=planner_fixture.PROVIDER,
                  max_tasks=10, max_decisions=30, max_activations=40, checks=checks)
    scope = root_scope(w)
    w = sw.mutate(["planning", "claim", wid], w, scope="root", scope_revision=scope["revision"],
                  holder="banc-installation", lease_seconds=60)
    scope = root_scope(w)
    tasks = [dict(id="prepare", title="prepare", requirements=["req-1"], deliverable=LOT_REL,
                  criteria=[CRITERIA[0]], depends=[], next="Proposer le lot de paiement dans docs/prepare.md"),
             dict(id="settle", title="settle", requirements=["req-2"], deliverable="docs/settle.md",
                  criteria=[CRITERIA[1]], depends=["prepare"],
                  next="Régler le lot remis par prepare et écrire le bilan dans docs/settle.md")]
    if fault == "F4e":   # sans dépendance, même espace : part dès la fin de prepare et retient settle
        tasks.append(dict(id="verrou", title="verrou", requirements=["req-3"], deliverable="docs/verrou.md",
                          criteria=[HOLD_CRITERION], depends=[], next="Lever le verrou après la modification du lot"))
    w = sw.mutate(["planning", "decide", wid], w, scope="root", scope_revision=scope["revision"],
                  holder=scope["holder"], generation=scope["generation"],
                  input_events=[e["id"] for e in w["planning"]["inbox"]
                                if e["scope"] == "root" and not e.get("decision")],
                  reason="Créer prepare puis settle, chacune avec son exigence et son contrôle explicite.",
                  operations=[dict(kind="task", **t) for t in tasks])
    found = {t["id"]: t.get("plan_max_attempts") for t in w["tasks"]}
    if found != {t["id"]: PLAN_MAX_ATTEMPTS for t in tasks}:
        raise harness.BenchError(f"tentatives maximales inattendues : {found}", run_dir)
    profiles = [("prepare", prepare_provider), ("settle", "banc-settle")] + (
        [("verrou", "banc-prepare")] if fault == "F4e" else [])
    for tid, provider in profiles:
        launch = profile(sw.root, provider)
        if fault == "F6" and tid == "prepare":
            # Le contrat d'une tâche planifiée est immuable (store.go : « contrat hiérarchique immuable ») ;
            # le budget passe par le profil de lancement (model.go : LaunchProfile.Limits, run_limits.go :
            # RunLimits.max_tool_calls), transmis au départ (dispatcher.go) et resserré (agents_store.go).
            launch["limits"] = {"max_tool_calls": config.S_BUDGET_TOOL_CALLS}
        sw.cli(["profile", wid, tid], launch)
    return sw.work(wid)


def log_tail(path):
    try:
        return Path(path).read_text(errors="replace").strip()[-config.ERROR_TEXT_MAX:]
    except OSError as e:
        return f"journal illisible : {e}"


def await_listening(conductors, log_path, run_dir):
    """Chaque conducteur lancé doit annoncer son écoute loopback ; sinon le banc a échoué."""
    deadline = time.monotonic() + config.S_CONDUCTOR_START_S
    while True:
        dead = [c.pid for c in conductors if c.poll() is not None]
        if dead:
            raise harness.BenchError(f"conducteur arrêté au démarrage {dead} : {log_tail(log_path)}", run_dir)
        if Path(log_path).read_text(errors="replace").count(LISTENING) >= len(conductors):
            return
        if time.monotonic() > deadline:
            raise harness.BenchError(f"conducteur muet après {config.S_CONDUCTOR_START_S} s : {log_tail(log_path)}",
                                     run_dir)
        time.sleep(0.05)


def stall_and_take_over(sw, wid, faults, conductors, log):
    """F7 : pendant prepare, le premier conducteur est figé au-delà du bail, un second prend la main.

    Le marqueur est posé au gel (prepare l'attend pour terminer : le gel tombe pendant prepare), puis
    réécrit avec `takeover_at`, le lancement du second conducteur après S_STALL_S, et `woke_at`.
    """
    first = conductors[0]
    before, _ = supervision_holder(sw.root, wid)
    os.kill(first.pid, signal.SIGSTOP)
    frozen = time.time()
    marker = faults / "owner-stalled.json"
    markers.mark(marker, pid=first.pid, stalled_for_s=config.S_STALL_S, frozen_at=frozen, holder_before=before)
    time.sleep(config.S_STALL_S)
    conductors.append(sw.conductor(log))
    takeover = time.time()
    after, source = before, None
    deadline = time.monotonic() + config.S_TAKEOVER_TIMEOUT_S
    # le second conducteur `web` doit obtenir le bail (CAS) ; un bail éphémère de superviseur ne compte pas
    while (after == before or source != WEB_CONDUCTOR) and time.monotonic() < deadline:
        time.sleep(0.1)
        after, source = supervision_holder(sw.root, wid)
    os.kill(first.pid, signal.SIGCONT)
    taken = after != before and source == WEB_CONDUCTOR
    markers.mark(marker, pid=first.pid, stalled_for_s=config.S_STALL_S, frozen_at=frozen, takeover_at=takeover,
                 holder_before=before, holder_after=after if taken else None,
                 holder_after_source=source if taken else None, woke_at=time.time())


def watch(sw, wid, run_dir, fault, conductors, log):
    """Surveille jusqu'à `settle` acceptée et périmètre clos, au calme, ou au délai. Renvoie (travail, délai).

    Une panne d'infrastructure (infrastructure_fault) ou une clôture non obtenue au calme lève BenchError.
    """
    faults = run_dir / "faults"
    deadline = time.monotonic() + config.S_RUN_TIMEOUT_S
    work, last, quiet_since, injected = None, None, None, False
    while time.monotonic() < deadline:
        if all(c.poll() is not None for c in conductors):
            raise harness.BenchError(f"plus aucun conducteur actif : {log_tail(run_dir / 'conductor.log')}", run_dir)
        work, agents = sw.work(wid), sw.agents(wid)
        reason = infrastructure_fault(work, agents, fault)
        if reason:
            raise harness.BenchError(reason, run_dir)
        tasks = {t["id"]: t["status"] for t in work["tasks"]}
        scope_state = root_scope(work)["state"]
        if not injected and fault == "F7" and tasks.get("prepare") == "running":
            stall_and_take_over(sw, wid, faults, conductors, log)
            injected = True
        settle_active = any(a.get("task_id") == "settle" and a["status"] in ACTIVE for a in agents)
        if not injected and tasks.get("prepare") == "accepted" and (sw.root / LOT_REL).exists() \
                and (fault == "F4e" or (fault == "F4" and settle_active)):
            # F4 : seulement une fois settle vue active (elle attend tamper-done) ; F4e : le verrou la retient.
            name = harness.EXPECTED_MARKER[fault]
            harness.tamper(sw.root / LOT_REL, faults / f"{name}.json", fault=name)
            (faults / "tamper-done").write_text("1")
            injected = True
        if tasks.get("settle") == "accepted" and scope_state == "closed":
            return work, False
        busy = (any(a["status"] in ACTIVE for a in agents) or any(s in PENDING for s in tasks.values())
                or planning_busy(work))
        state = (work["revision"], scope_state, tuple(sorted(tasks.items())))
        if not busy and state == last:
            quiet_since = quiet_since or time.monotonic()
            if time.monotonic() - quiet_since >= config.S_QUIET_S:
                if tasks.get("settle") == "accepted":   # aucune faute exercée n'explique une clôture manquante
                    raise harness.BenchError(f"clôture non obtenue : settle acceptée, périmètre {scope_state}", run_dir)
                return work, False
        else:
            quiet_since = None
        last = state
        time.sleep(config.S_POLL_S)
    return work, True


def stop(sw, wid, conductors):
    """Arrêt ordonné : mission en pause, agents arrêtés et attendus, puis conducteurs. Renvoie les erreurs."""
    errors = []
    if wid is not None:
        try:
            sw.cli(["mission", "stop", wid])
        except (RuntimeError, subprocess.TimeoutExpired) as e:
            errors.append(f"mission stop : {e}")
        deadline = time.monotonic() + config.S_CLEANUP_TIMEOUT_S
        while True:
            try:
                active = [a["id"] for a in sw.agents(wid) if a["status"] in ACTIVE]
            except (RuntimeError, subprocess.TimeoutExpired, KeyError, ValueError) as e:
                errors.append(f"agent list : {e}")
                break
            if not active or time.monotonic() > deadline:
                if active:
                    errors.append(f"agents encore actifs après {config.S_CLEANUP_TIMEOUT_S} s : {active}")
                break
            for aid in active:
                try:
                    sw.cli(["agent", "stop", aid])
                except (RuntimeError, subprocess.TimeoutExpired) as e:
                    if "déjà terminé" not in str(e):
                        errors.append(f"agent stop {aid} : {e}")
            time.sleep(config.S_POLL_S)
    for c in conductors:
        try:
            os.kill(c.pid, signal.SIGCONT)   # F7 : un conducteur figé ne traite pas SIGTERM
        except ProcessLookupError:
            pass
        harness.stop(c)
    return errors


def reap(run_dir):
    """Tue tout processus restant dont la ligne de commande désigne la racine (superviseurs, fournisseurs).

    Les superviseurs `_supervise` sont détachés (Setsid, agents_process.go : spawnAgent) : ni le conducteur
    ni ce processus ne les attendent. Renvoie les lignes de commande des processus tués.
    """
    needle, me, found = str(run_dir).encode(), os.getpid(), {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit() or int(entry.name) == me:
            continue
        try:
            cmdline = (entry / "cmdline").read_bytes()
        except OSError:
            continue
        if any(arg == needle or arg.startswith(needle + b"/") for arg in cmdline.split(b"\0")):
            found[int(entry.name)] = cmdline.replace(b"\0", b" ").decode("utf-8", "replace").strip()
    for sig in (signal.SIGCONT, signal.SIGTERM):
        for pid in found:
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline and any(Path(f"/proc/{pid}").exists() for pid in found):
        time.sleep(0.05)
    for pid in found:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    return sorted(found.values())


def outcome_of(work):
    """Résumé mesurable du travail final : statuts, tentatives acceptées, remises, état du périmètre."""
    if work is None:
        return {"task_status": {}, "scope_state": None, "accepted_attempts": {}, "handoffs": []}
    return {"task_status": {t["id"]: t["status"] for t in work["tasks"]},
            "scope_state": root_scope(work)["state"],
            "accepted_attempts": {t["id"]: (t.get("automatic_validation") or {}).get("attempt_id")
                                  for t in work["tasks"]
                                  if (t.get("automatic_validation") or {}).get("state") == "accepted"},
            "handoffs": [{"task": e.get("task"), "attempt": e.get("attempt"), "artifacts": e.get("artifacts")}
                         for e in work["planning"]["inbox"] if e.get("kind") == "handoff"]}


def run(key_mode, fault, seed, binary=SWARM, real_agent=None):
    """`real_agent` : commande d'un agent réel pour la préparation (tâche 11), sinon préparateur scripté."""
    if key_mode not in harness.KEY_MODES or fault not in harness.FAULTS:
        raise ValueError((key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir("banc-s-")
    try:
        return _run(run_dir, key_mode, fault, seed, started, Path(binary), real_agent)
    except harness.BenchError:
        raise
    except Exception as e:   # toute autre exception porte aussi la racine conservée
        raise harness.BenchError(f"exécution interrompue : {type(e).__name__}: {e}", run_dir) from e


def _run(run_dir, key_mode, fault, seed, started, binary, real_agent):
    if not binary.is_file():
        raise harness.BenchError(f"binaire swarm absent : {binary}", run_dir)
    faults, root, bank = run_dir / "faults", run_dir / "swarm", run_dir / "bank"
    root.mkdir()
    bank.mkdir()
    harness.init_ledger(bank / "ledger.db", seed)
    token_file = bank / "settle.token"
    token_file.write_text(uuid.uuid4().hex)
    sw = Swarm(binary, root)
    conductors, work, timed_out, agents, wid, validation = [], None, False, [], None, None
    cleanup = {"errors": [], "reaped": []}
    api_options = {"lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "fail_snapshot": faults / "snapshot-503.json" if fault == "F5" else None,
                   "token_file": token_file}
    log_path = run_dir / "conductor.log"
    try:
        with harness.Api(bank, **api_options) as api, open(log_path, "w") as log:
            try:
                sw.cli(["init"])
                (root / "docs").mkdir(exist_ok=True)
                common = [harness.SCRIPTS["provider_s"], str(root), str(faults), str(binary), api.url, key_mode, fault]
                providers = {"banc-prepare": {"command": sys.executable, "env_allow": [], "args": common},
                             "banc-settle": {"command": sys.executable, "env_allow": [],
                                             "args": [*common, str(token_file)]},
                             planner_fixture.PROVIDER: planner_fixture.install(run_dir)}
                if real_agent:
                    providers["banc-reel"] = {"command": sys.executable, "env_allow": ["HOME", "PATH"],
                                              "args": [harness.SCRIPTS["provider_real"], str(root), str(binary),
                                                       api.url, "--", *real_agent]}
                (root / ".swarm" / "providers.json").write_text(
                    json.dumps({"schema_version": 1, "providers": providers}))
                w = setup_work(sw, run_dir, api.url, fault, "banc-reel" if real_agent else "banc-prepare")
                wid = w["id"]
                sw.cli(["autonomy", wid, "autonome", "2"])
                sw.cli(["mission", "start", wid], profile(root, "banc-prepare"))
                conductors.append(sw.conductor(log))
                if fault == "F2":
                    conductors.append(sw.conductor(log))
                await_listening(conductors, log_path, run_dir)
                if fault == "F2":
                    markers.mark(faults / "dual-launch.json", conductors=[c.pid for c in conductors])
                work, timed_out = watch(sw, wid, run_dir, fault, conductors, log)
                agents = sw.agents(wid)
                if fault in ("F4", "F4e"):
                    validation = prepare_validation_state(sw, wid)
            finally:
                cleanup["errors"] = stop(sw, wid, conductors)
    finally:   # API arrêtée par le bloc with ; tout reste lié à la racine est tué
        cleanup["reaped"] = reap(run_dir)
        (run_dir / "cleanup.json").write_text(json.dumps(cleanup, ensure_ascii=False, indent=2))
    reason = infrastructure_fault(work, agents, fault) if work is not None else None
    if reason:   # dernier état observé avant l'arrêt : une panne n'est jamais une mesure
        raise harness.BenchError(reason, run_dir)
    proof = order_proof(fault, work, agents, faults, root, wid)
    summary = outcome_of(work)
    declared = summary["task_status"].get("settle") == "accepted" and summary["scope_state"] == "closed"
    result = harness.finish(run_dir, condition="S", key_mode=key_mode, fault=fault, seed=seed, started=started,
                          declared_success=declared, timed_out=timed_out, bank_dir=bank,
                          extra={"launches": len(agents), **summary, **settle_outcome(agents, fault),
                                 "agent_status": [a["status"] for a in agents], "conductors": len(conductors),
                                 "cleanup": cleanup, "real_agent": real_agent[0] if real_agent else None,
                                 "order_proof": proof})
    if fault in ("F4", "F4e"):
        result["prepare_validation_state"] = validation
        if not engine_stop_consistent(result["stopped_by"], validation) and result["status"] == "OK":
            result["status"] = "INVALIDE"   # « arrêté par le moteur » sans preuve périmée : attribution infondée
    if proof is not None and not proof["proven"] and result["status"] == "OK":   # ordre non prouvé : faute non injectée
        result["status"] = "INVALIDE"
    return result
