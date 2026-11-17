"""Pure selected-family process-tree contract guards; no IO or children.

Trace data is synthetic, shaped from retained controlled primary records.
PIDs/paths/stack values below are fixture values, not operational policy.
"""
import copy
import unittest
from unittest.mock import patch

import trace_process_tree as observer


def _hex(value):
    """Encode one synthetic string using the producer's complete -xx form."""
    return '"' + ''.join('\\x%02x' % b for b in value.encode()) + '"'


def _array(values):
    """Encode an explicit synthetic argv/environment vector without elision."""
    return '[' + ', '.join(_hex(value) for value in values) + ']'


def fixture():
    """Return fresh declared nodes, edges and synthetic complete selected lines.

    Root/child/grandchild exec identity is separate from spoofed argv[0]. Other
    syscall lines are intentionally outside the observation contract.
    """
    nodes = {pid: {'executable': '/fixture/python',
                   'argv': ['/fixture/python' if pid != 202 else 'owned-spoof',
                            '-I', '-S', '-B', '/fixture/worker', role],
                   'environment': {'LC_ALL': 'C'}, 'exit_code': 0}
             for pid, role in [(101, 'parent'), (202, 'child'), (303, 'grandchild')]}
    edges = [(101, 202), (202, 303)]
    texts = {}
    for pid, node in nodes.items():
        lines = []
        if pid != 101:
            lines.append('dup2(7, 1) = 1')
        lines.append('execve(' + _hex(node['executable']) + ', ' +
                     _array(node['argv']) + ', ' + _array(['LC_ALL=C']) + ') = 0')
        lines.append('brk(NULL) = 0x1000')
        for parent, child in edges:
            if parent == pid:
                lines.extend([
                    'clone3({flags=CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND, '
                    'exit_signal=SIGCHLD, stack=0x1000, stack_size=0x9000}, 88) = '+str(child),
                    'wait4('+str(child)+', [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], '
                    '0, NULL) = '+str(child)])
        lines.extend(['exit_group(0) = ?', '+++ exited with 0 +++'])
        texts[pid] = '\n'.join(lines)+'\n'
    return texts, nodes, edges


class DeclaredProcessTreeGuards(unittest.TestCase):
    """Verify selected observations and explicit refusal with healthy recovery."""

    def _observe(self, texts, nodes, edges):
        """Call the pure observer while refusing any attempted real child."""
        with patch('subprocess.Popen', side_effect=AssertionError('NO_CHILD')) as child:
            result = observer.parse_declared_tree(texts, nodes, edges)
            self.assertEqual(child.call_count, 0)
        self.assertIsInstance(result, dict)
        self.assertIs(result['complete_build_qualification'], False)
        self.assertIs(result['behavioral_admission'], False)
        return result

    def _refusal(self, mutate):
        """Validate first, inject only the specified contract defect, recover."""
        self._observe(*fixture())
        data = copy.deepcopy(fixture())
        mutate(*data)
        with patch('subprocess.Popen', side_effect=AssertionError('NO_CHILD')):
            with self.assertRaises(ValueError):
                observer.parse_declared_tree(*data)
        self._observe(*fixture())

    def test_declared_spoofed_argv_and_selected_tree(self):
        """A declared argv0 spoof is valid without claiming complete capture."""
        self._observe(*fixture())

    def test_wait_status_mismatch_refused(self):
        """Parent wait status cannot contradict the child's observed terminal."""
        def mutate(texts, nodes, edges):
            """Change only one observed wait status, leaving child exit zero."""
            texts[101] = texts[101].replace('WEXITSTATUS(s) == 0', 'WEXITSTATUS(s) == 7')
        self._refusal(mutate)

    def test_missing_terminal_refused(self):
        """No terminal is unknown, never a fabricated successful exit."""
        def mutate(texts, nodes, edges):
            """Remove only the leaf's terminal, retaining its exit_group call."""
            texts[303] = texts[303].replace('+++ exited with 0 +++\n', '')
        self._refusal(mutate)

    def test_unsupported_process_shape_refused(self):
        """A legacy fork shape cannot silently pass as an unexamined syscall."""
        def mutate(texts, nodes, edges):
            """Add an unsupported process-family event before normal exit."""
            texts[101] = texts[101].replace('exit_group(0)', 'vfork() = 404\nexit_group(0)')
        self._refusal(mutate)

    def test_boolean_pid_refused(self):
        """Python bool is not accepted as a declared process identifier."""
        def mutate(texts, nodes, edges):
            """Replace one trace map key with a boolean, not an integer PID."""
            texts[True] = texts.pop(101)
        self._refusal(mutate)

    def test_malformed_selected_name_refused(self):
        """A malformed selected-family prefix is not an unrelated syscall."""
        def mutate(texts, nodes, edges):
            """Add a broken clone3 record beside an otherwise valid tree."""
            texts[303] = texts[303].replace('exit_group(0)',
                                          'clone3 malformed\nexit_group(0)')
        self._refusal(mutate)

    def test_environment_name_separator_refused(self):
        """An equals sign in a declared name cannot change env mapping meaning."""
        def mutate(texts, nodes, edges):
            """Construct matching bytes whose actual first-equals name differs."""
            nodes[101]['environment'] = {'LC_ALL=C': 'D'}
            texts[101] = texts[101].replace(_array(['LC_ALL=C']),
                                          _array(['LC_ALL=C=D']))
        self._refusal(mutate)

    def test_resumed_wait_shape_refused(self):
        """A strace resumed record must refuse, rather than pass unexamined."""
        def mutate(texts, nodes, edges):
            """Add the actual resumed-prefix form beside complete valid events."""
            texts[101] = texts[101].replace('exit_group(0)',
                '<... wait4 resumed> [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], '
                '0, NULL) = 202\nexit_group(0)')
        self._refusal(mutate)


if __name__ == '__main__':
    unittest.main(verbosity=2)
