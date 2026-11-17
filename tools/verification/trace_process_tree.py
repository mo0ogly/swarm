"""Pure declared-process-tree verifier over caller-supplied strace texts.

Grammar scope (deliberately narrow, per the primary selected-process sample):
  * complete successful ``execve`` lines in strace ``-xx -v`` form (arrays of
    ``\\xNN`` byte-string tokens ending ``= 0``), reused from the existing
    ``trace_process_witness`` EXEC / hex_array helpers via real imports;
  * successful ``clone3`` with exactly the primary sample's flags
    (``CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND``, ``exit_signal=SIGCHLD``);
    the stack address, stack_size and struct-size argument are kept as
    observed data with no invented kernel bound;
  * the exact ``wait4`` exited receipt
    ``[{WIFEXITED(s) && WEXITSTATUS(s) == N}], 0, NULL``;
  * ``exit_group(N) = ?`` and the ``+++ exited with N +++`` terminal.

Contract enforced: the expected graph is one rooted tree (single-node trees
valid) with exact types (``bool`` is never ``int``), no missing/extra/
duplicate nodes or edges, no self edges, unknown refs, multiple parents,
cycles or disconnection; declared exit codes lie in the technical 0..255
normal-exit range (not operator policy); every declared PID shows exactly one
exec and one final terminal in the order exec, (clone3, wait4)*, exit_group,
terminal; observed executable/argv/environment match the declaration exactly
(argv[0] is compared only against the declared argv, so a spoofed argv[0] is
valid when declared); each declared edge is observed exactly once as a
clone3 followed in the same parent stream by exactly one matching wait4 whose
receipt equals the child's observed terminal; no extra clone child is
accepted.

Limits: pure functions only -- no CLI, no IO, no hashing, no subprocesses.
Raw-text custody, UTF-8 decoding and hash provenance belong to the caller;
nothing here asserts provenance.  Only the named family shapes are parsed;
abbreviations, unfinished/resumed markers, multiple exec/terminal events,
signal terminals and legacy fork/clone/vfork/execveat/waitid shapes are
refused (never "successful unknown"); every other line is deliberately
unexamined and only reported per PID by line number and count, with SIGCHLD
delivery lines explicitly unexamined and never interpreted.  The result never
claims a complete build qualification or behavioral admission.
"""

import re

from trace_process_witness import EXEC, hex_array

__all__ = ["parse_declared_tree"]

_NODE_KEYS = frozenset({"executable", "argv", "environment", "exit_code"})
_LEGACY_SYSCALLS = frozenset({"fork", "vfork", "clone", "execveat", "waitid"})

# Recognize the leading name even if its following syntax is malformed, so
# a broken selected family cannot silently become an unexamined line.
_SYSCALL_NAME = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\b")
_TERMINAL = re.compile(r"\+\+\+ exited with ([0-9]+) \+\+\+")
_EXIT_GROUP = re.compile(r"exit_group\(([0-9]+)\)\s+= \?")
_CLONE3 = re.compile(
    r"clone3\(\{flags=CLONE_VM\|CLONE_VFORK\|CLONE_CLEAR_SIGHAND, "
    r"exit_signal=SIGCHLD, stack=0x([0-9a-f]+), stack_size=0x([0-9a-f]+)\}, "
    r"([0-9]+)\) = ([0-9]+)"
)
_WAIT4 = re.compile(
    r"wait4\(([0-9]+), \[\{WIFEXITED\(s\) && WEXITSTATUS\(s\) == ([0-9]+)\}\], "
    r"0, NULL\) = ([0-9]+)"
)


def _fail(reason, pid=None, lineno=None):
    """Raise ValueError with a short constant reason plus optional pid/line.

    Never embeds raw trace text or child bytes in the message.
    """
    detail = reason
    if pid is not None:
        detail += " (pid=%d" % pid
        if lineno is not None:
            detail += ", line=%d" % lineno
        detail += ")"
    raise ValueError("trace-process-tree: " + detail)


def _is_positive_pid(value):
    """True only for exact positive ints; bool is intentionally rejected."""
    return type(value) is int and value >= 1


def _decode_array(text, pid, lineno):
    """Decode one complete ``-xx`` array with the witness hex_array helper.

    Inputs: raw array text captured from an EXEC match (``[`` of ``\\xNN``
    string tokens ``]``).  Preconditions: text came from an EXEC-matched
    line; the helper re-validates the exact array grammar itself.
    Output: list of decoded bytes objects.
    Errors: the witness helper's ValueError refusal propagates unchanged; any
    other helper failure is normalized to a short-reason ValueError.
    Effects: none (pure).  Limits: no abbreviation, no partial, nested or
    non-hex string content.
    """
    try:
        return hex_array(text)
    except ValueError:
        raise
    except Exception as exc:
        raise ValueError(
            "trace-process-tree: trace-array-syntax (pid=%d, line=%d)" % (pid, lineno)
        ) from exc


def _decode_scalar(token, pid, lineno):
    """Decode one complete ``-xx`` scalar string token to bytes.

    Inputs: a quoted ``\\xNN`` token as captured by the witness EXEC regex.
    Output: single bytes value.  Errors: ValueError for any non-single result
    or syntax refusal.  Effects: none.  Limits: complete hex-escaped strings
    only.
    """
    values = _decode_array("[" + token + "]", pid, lineno)
    if len(values) != 1:
        _fail("exec-string-shape", pid=pid, lineno=lineno)
    return values[0]


def _validate_contract(trace_text_by_pid, expected_nodes, expected_edges):
    """Validate exact input types and the rooted-tree shape of expectations.

    Inputs: trace_text_by_pid (pid -> full raw text str), expected_nodes
    (pid -> record with exactly executable/argv/environment/exit_code),
    expected_edges (list of (parent, child) tuples).
    Preconditions: none beyond the arguments; all checks happen here.
    Output: mapping parent pid -> frozenset of declared child pids.
    Errors: ValueError with short reasons for wrong or empty containers,
    wrong record keys or field types, unrepresentable environment names/values
    (empty name, '=' in name, or NUL), exit codes outside 0..255, PID-set
    mismatch between texts and nodes, self/unknown/duplicate edges, multiple
    parents, absent or multiple roots, or cycles/disconnection.
    Effects: none (pure).  Limits: exact type checks (bool never accepted as
    int); no defaulting; no fixed node counts, PIDs, paths, dates, timeouts,
    volume or attempt caps.
    """
    if type(trace_text_by_pid) is not dict or not trace_text_by_pid:
        _fail("trace-text-map-type")
    if type(expected_nodes) is not dict or not expected_nodes:
        _fail("expected-nodes-type")
    if type(expected_edges) is not list:
        _fail("expected-edges-type")
    for pid in trace_text_by_pid:
        if not _is_positive_pid(pid):
            _fail("trace-pid-type")
        if type(trace_text_by_pid[pid]) is not str:
            _fail("trace-text-type", pid=pid)
    for pid in expected_nodes:
        if not _is_positive_pid(pid):
            _fail("node-pid-type")
        node = expected_nodes[pid]
        if type(node) is not dict or frozenset(node) != _NODE_KEYS:
            _fail("node-record-shape", pid=pid)
        if type(node["executable"]) is not str:
            _fail("node-executable-type", pid=pid)
        argv = node["argv"]
        if type(argv) is not list or any(type(item) is not str for item in argv):
            _fail("node-argv-type", pid=pid)
        environment = node["environment"]
        if type(environment) is not dict or any(
            type(key) is not str or type(value) is not str
            for key, value in environment.items()
        ):
            _fail("node-environment-type", pid=pid)
        # A mapping name must not change meaning when encoded as NAME=VALUE.
        # These are environment-vector representation boundaries, not policy.
        if any(not key or '=' in key or '\0' in key or '\0' in value
               for key, value in environment.items()):
            _fail("node-environment-representation", pid=pid)
        code = node["exit_code"]
        if type(code) is not int or not 0 <= code <= 255:
            _fail("node-exit-code-range", pid=pid)
    parents = {}
    adjacency = {}
    seen_edges = set()
    for edge in expected_edges:
        if type(edge) is not tuple or len(edge) != 2:
            _fail("edge-shape")
        parent, child = edge
        if not _is_positive_pid(parent) or not _is_positive_pid(child):
            _fail("edge-pid-type")
        if parent not in expected_nodes or child not in expected_nodes:
            _fail("edge-unknown-node-ref")
        if parent == child:
            _fail("edge-self")
        if edge in seen_edges:
            _fail("edge-duplicate")
        seen_edges.add(edge)
        if child in parents:
            _fail("edge-multiple-parents")
        parents[child] = parent
        adjacency.setdefault(parent, []).append(child)
    roots = [pid for pid in expected_nodes if pid not in parents]
    if len(roots) == 0:
        _fail("tree-no-root")
    if len(roots) > 1:
        _fail("tree-multiple-roots")
    root = roots[0]
    reachable = {root}
    pending = [root]
    while pending:
        for child in adjacency.get(pending.pop(), ()):
            if child not in reachable:
                reachable.add(child)
                pending.append(child)
    if len(reachable) != len(expected_nodes):
        _fail("tree-disconnected-or-cyclic")
    if set(trace_text_by_pid) != set(expected_nodes):
        _fail("trace-pid-set-mismatch")
    return {pid: frozenset(adjacency.get(pid, ())) for pid in expected_nodes}


def _observe_pid(pid, text, node, declared_children):
    """Observe one declared PID's raw text against its declared record.

    Inputs: pid (positive int), text (caller-decoded full raw strace text),
    node (declared record), declared_children (frozenset of declared child
    pids for this pid).
    Preconditions: contract already validated by _validate_contract; text is
    the caller's full raw text (no IO or decoding happens here).
    Output: dict with "exec"/"exit_group"/"terminal" evidence (raw line +
    line number, observed exit codes where applicable), "clone_children"
    (one record per declared edge from this pid: clone3 and wait4 evidence,
    observed stack/stack_size/size tokens, wait receipt code), and examined
    plus unexamined line-number lists.
    Errors: ValueError with a short reason (never raw text) for any
    execve/clone3/wait4/exit_group/terminal line not matching the accepted
    shapes, unfinished/resumed markers, multiple exec/terminal, signal
    terminals, legacy fork/clone/vfork/execveat/waitid, ordering violations,
    executable/argv/environment mismatch, undeclared/duplicate/missing clone
    children, unexpected wait4, or exit-code mismatches.
    Effects: none (pure).  Limits: only the named successful-family shapes
    are parsed; unexamined lines are recorded, never interpreted; the
    per-PID required order is exec, (clone3, wait4)*, exit_group, terminal;
    declared strings are compared as UTF-8 encodings of the observed bytes.
    """
    exec_evidence = None
    exit_evidence = None
    terminal_evidence = None
    clones = []
    open_clone = None
    examined = []
    unexamined = []
    exec_seen = False
    exit_seen = False

    for lineno, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line:
            unexamined.append(lineno)
            continue
        if line.startswith("+++"):
            match = _TERMINAL.fullmatch(line)
            if match is None:
                _fail("unsupported-terminal-shape", pid=pid, lineno=lineno)
            if terminal_evidence is not None:
                _fail("multiple-terminal", pid=pid, lineno=lineno)
            if not exit_seen:
                _fail("terminal-before-exit-group", pid=pid, lineno=lineno)
            terminal_evidence = {
                "line_number": lineno,
                "raw_line": raw,
                "observed_exit_code": int(match.group(1)),
            }
            examined.append(lineno)
            continue
        if terminal_evidence is not None:
            _fail("post-terminal-line", pid=pid, lineno=lineno)
        # strace puts the syscall name between "<..." and "resumed>".
        # All continuation-prefixed records are outside this complete-line
        # grammar, including malformed continuation prefixes.
        if "<unfinished ...>" in line or line.startswith("<...") or "... resumed>" in line:
            _fail("unfinished-or-resumed-line", pid=pid, lineno=lineno)
        name_match = _SYSCALL_NAME.match(line)
        name = name_match.group(1) if name_match is not None else None
        if name == "execve":
            examined.append(lineno)
            match = EXEC.match(line)
            if match is None:
                _fail("unsupported-execve-shape", pid=pid, lineno=lineno)
            if exec_seen:
                _fail("multiple-exec", pid=pid, lineno=lineno)
            observed_exec = _decode_scalar(match.group(1), pid, lineno)
            if observed_exec != node["executable"].encode("utf-8"):
                _fail("exec-executable-mismatch", pid=pid, lineno=lineno)
            observed_argv = _decode_array(match.group(2), pid, lineno)
            declared_argv = [item.encode("utf-8") for item in node["argv"]]
            if len(observed_argv) != len(declared_argv) or any(
                observed != declared
                for observed, declared in zip(observed_argv, declared_argv)
            ):
                _fail("exec-argv-mismatch", pid=pid, lineno=lineno)
            observed_env = _decode_array(match.group(3), pid, lineno)
            if len(set(observed_env)) != len(observed_env):
                _fail("exec-environment-duplicate", pid=pid, lineno=lineno)
            declared_env = {
                (key + "=" + value).encode("utf-8")
                for key, value in node["environment"].items()
            }
            if set(observed_env) != declared_env:
                _fail("exec-environment-mismatch", pid=pid, lineno=lineno)
            exec_evidence = {"line_number": lineno, "raw_line": raw}
            exec_seen = True
        elif name == "clone3":
            examined.append(lineno)
            if not exec_seen:
                _fail("clone3-before-exec", pid=pid, lineno=lineno)
            if exit_seen:
                _fail("clone3-after-exit-group", pid=pid, lineno=lineno)
            if open_clone is not None:
                _fail("clone3-before-wait4", pid=pid, lineno=lineno)
            match = _CLONE3.fullmatch(line)
            if match is None:
                _fail("unsupported-clone3-shape", pid=pid, lineno=lineno)
            child = int(match.group(4))
            if child < 1:
                _fail("clone3-child-pid", pid=pid, lineno=lineno)
            if child not in declared_children:
                _fail("undeclared-clone3-child", pid=pid, lineno=lineno)
            if any(record["child_pid"] == child for record in clones):
                _fail("duplicate-clone3-child", pid=pid, lineno=lineno)
            open_clone = {
                "child_pid": child,
                "clone3_line": {"line_number": lineno, "raw_line": raw},
                "clone3_stack": "0x" + match.group(1),
                "clone3_stack_size": "0x" + match.group(2),
                "clone3_size_arg": match.group(3),
            }
        elif name == "wait4":
            examined.append(lineno)
            if open_clone is None:
                _fail("unexpected-wait4", pid=pid, lineno=lineno)
            match = _WAIT4.fullmatch(line)
            if match is None:
                _fail("unsupported-wait4-shape", pid=pid, lineno=lineno)
            if int(match.group(1)) != open_clone["child_pid"]:
                _fail("wait4-child-mismatch", pid=pid, lineno=lineno)
            if int(match.group(3)) != open_clone["child_pid"]:
                _fail("wait4-result-mismatch", pid=pid, lineno=lineno)
            open_clone["wait4_line"] = {"line_number": lineno, "raw_line": raw}
            open_clone["wait4_receipt_code"] = int(match.group(2))
            clones.append(open_clone)
            open_clone = None
        elif name == "exit_group":
            examined.append(lineno)
            if not exec_seen:
                _fail("exit-group-before-exec", pid=pid, lineno=lineno)
            if open_clone is not None:
                _fail("exit-group-before-wait4", pid=pid, lineno=lineno)
            if exit_seen:
                _fail("multiple-exit-group", pid=pid, lineno=lineno)
            match = _EXIT_GROUP.fullmatch(line)
            if match is None:
                _fail("unsupported-exit-group-shape", pid=pid, lineno=lineno)
            exit_evidence = {
                "line_number": lineno,
                "raw_line": raw,
                "observed_exit_code": int(match.group(1)),
            }
            exit_seen = True
        elif name in _LEGACY_SYSCALLS:
            _fail("legacy-syscall-shape", pid=pid, lineno=lineno)
        else:
            unexamined.append(lineno)

    if not exec_seen:
        _fail("missing-exec", pid=pid)
    if open_clone is not None:
        _fail("missing-wait4", pid=pid)
    if not exit_seen:
        _fail("missing-exit-group", pid=pid)
    if terminal_evidence is None:
        _fail("missing-terminal", pid=pid)
    if exit_evidence["observed_exit_code"] != node["exit_code"]:
        _fail("exit-group-code-mismatch", pid=pid)
    if terminal_evidence["observed_exit_code"] != node["exit_code"]:
        _fail("terminal-code-mismatch", pid=pid)
    if {record["child_pid"] for record in clones} != set(declared_children):
        _fail("declared-edge-clone3-missing", pid=pid)
    return {
        "pid": pid,
        "exec": exec_evidence,
        "exit_group": exit_evidence,
        "terminal": terminal_evidence,
        "clone_children": clones,
        "examined_line_numbers": examined,
        "unexamined_line_numbers": unexamined,
    }


def parse_declared_tree(trace_text_by_pid, expected_nodes, expected_edges):
    """Verify caller-supplied per-PID strace texts against one declared tree.

    Inputs:
      * ``trace_text_by_pid``: dict mapping each positive int PID to that
        process's full raw strace text (str).  Raw-byte custody, UTF-8
        decoding and any hash provenance belong to the caller; this function
        performs no IO and asserts no provenance.
      * ``expected_nodes``: dict mapping each positive int PID to a record
        with exactly ``executable: str``, ``argv: list[str]``,
        ``environment: dict[str, str]`` with nonempty names containing neither
        '=' nor NUL and values without NUL, and ``exit_code: int`` in the
        technical 0..255 normal-exit range (not operator policy).
      * ``expected_edges``: list of ``(parent_pid, child_pid)`` tuples of
        positive ints.

    Preconditions: exact types throughout (``bool`` never accepted as
    ``int``); nonempty node and trace maps with identical PID sets; the
    expected graph is one rooted tree (a single-node tree is valid) with no
    missing/extra/duplicate nodes or edges, self edges, unknown refs,
    multiple parents, cycles or disconnection.

    Output: dict with ``observation_scope`` (parsed grammar families,
    unexamined-line policy, custody note, per-PID examined/unexamined counts
    and totals), ``nodes`` (per declared PID: declared fields echoed plus
    exec, exit_group and terminal evidence as raw line + line number + PID),
    ``edges`` (per declared edge in caller order: clone3 and wait4 evidence
    with raw lines and line numbers, observed child PID, observed stack,
    stack_size and size argument kept as data, wait receipt code),
    ``unexamined_lines`` (per PID: count and line numbers only; original
    bytes stay with the caller), and the fixed flags
    ``complete_build_qualification: False`` and
    ``behavioral_admission: False``.

    Errors: ValueError with a short safe reason (pid/line numbers only,
    never raw trace text) for any contract violation, and for any unsupported
    or malformed shape in the named families: non-accepted execve/clone3/
    wait4/exit_group/terminal shapes, abbreviations, unfinished/resumed
    lines, multiple exec or terminal events, signal terminals, legacy
    fork/clone/vfork/execveat/waitid call shapes, ordering violations,
    executable/argv/environment mismatch, undeclared/duplicate/missing clone
    children, unexpected wait4, exit-code mismatch, or a wait receipt not
    equal to the child's observed terminal.

    Effects: none (pure; no CLI, no IO, no hashing, no subprocesses).

    Limits: only the named successful-family shapes are parsed; every other
    line is deliberately unexamined and only reported per PID, so no
    completeness claim is ever made; argv[0] is compared only against the
    declared argv (a spoofed argv[0] is valid when declared); declared
    strings are compared as their UTF-8 encodings against the observed
    ``-xx`` bytes; the result never asserts build qualification or
    behavioral admission.
    """
    children = _validate_contract(trace_text_by_pid, expected_nodes, expected_edges)
    observations = {
        pid: _observe_pid(
            pid, trace_text_by_pid[pid], expected_nodes[pid], children[pid]
        )
        for pid in expected_nodes
    }
    edges_evidence = []
    for parent, child in expected_edges:
        records = [
            record
            for record in observations[parent]["clone_children"]
            if record["child_pid"] == child
        ]
        if len(records) != 1:
            _fail("edge-clone3-observation-count", pid=parent)
        record = records[0]
        child_terminal = observations[child]["terminal"]["observed_exit_code"]
        if record["wait4_receipt_code"] != child_terminal:
            _fail("wait-receipt-child-terminal-mismatch", pid=parent)
        edges_evidence.append(
            {
                "parent_pid": parent,
                "child_pid": child,
                "clone3_line": record["clone3_line"],
                "wait4_line": record["wait4_line"],
                "clone3_child_pid": record["child_pid"],
                "clone3_stack": record["clone3_stack"],
                "clone3_stack_size": record["clone3_stack_size"],
                "clone3_size_arg": record["clone3_size_arg"],
                "wait4_receipt_code": record["wait4_receipt_code"],
            }
        )
    nodes_evidence = {}
    unexamined_lines = {}
    line_counts = {}
    examined_total = 0
    unexamined_total = 0
    for pid in expected_nodes:
        observation = observations[pid]
        node = expected_nodes[pid]
        nodes_evidence[pid] = {
            "pid": pid,
            "executable": node["executable"],
            "argv": list(node["argv"]),
            "environment": dict(node["environment"]),
            "exit_code": node["exit_code"],
            "exec_line": observation["exec"],
            "exit_group_line": observation["exit_group"],
            "terminal_line": observation["terminal"],
        }
        examined_count = len(observation["examined_line_numbers"])
        unexamined_count = len(observation["unexamined_line_numbers"])
        examined_total += examined_count
        unexamined_total += unexamined_count
        line_counts[pid] = {"examined": examined_count, "unexamined": unexamined_count}
        unexamined_lines[pid] = {
            "count": unexamined_count,
            "line_numbers": list(observation["unexamined_line_numbers"]),
        }
    scope = {
        "grammar_families": {
            "execve": "complete successful 'execve -xx -v' line ending '= 0'",
            "clone3": (
                "successful clone3 with flags=CLONE_VM|CLONE_VFORK|"
                "CLONE_CLEAR_SIGHAND and exit_signal=SIGCHLD; stack, "
                "stack_size and size argument kept as observed data"
            ),
            "wait4": "exact exited receipt '[{WIFEXITED(s) && WEXITSTATUS(s) == N}], 0, NULL'",
            "exit_group": "'exit_group(N) = ?'",
            "terminal": "'+++ exited with N +++'",
        },
        "unexamined_policy": (
            "every other line is reported per PID by line number and count "
            "only; raw bytes stay with the caller; SIGCHLD delivery lines "
            "are deliberately unexamined and not interpreted"
        ),
        "trace_text_custody": (
            "caller supplies full raw per-PID texts; this function performs "
            "no IO and asserts no hash provenance"
        ),
        "pids": sorted(expected_nodes),
        "line_counts_by_pid": line_counts,
        "examined_line_count": examined_total,
        "unexamined_line_count": unexamined_total,
    }
    return {
        "observation_scope": scope,
        "nodes": nodes_evidence,
        "edges": edges_evidence,
        "unexamined_lines": unexamined_lines,
        "complete_build_qualification": False,
        "behavioral_admission": False,
    }
