"""Stream qualification output to fresh evidence files before process completion."""
from pathlib import Path
import subprocess


def capture(args, *, cwd, env, data, stdout_path, stderr_path, pass_fds=()):
    """Run exact authorized fixture arguments with visible, untruncated output.

    Evidence paths must be new: exclusive creation preserves earlier failures.
    Return CompletedProcess with final byte contents, including nonzero exits;
    callers still validate the outcome. File/launch/read errors propagate. There
    is no global watchdog, shell, retry, truncation or invented execution policy.
    Optional POSIX descriptors support explicit fixture handshakes only; normal
    callers pass none. This helper grants no authority to execute its arguments.
    """
    stdout_path, stderr_path = Path(stdout_path), Path(stderr_path)
    with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
        completed = subprocess.run(args, cwd=cwd, env=env, input=data,
                                   stdout=stdout, stderr=stderr, pass_fds=pass_fds)
    return subprocess.CompletedProcess(args, completed.returncode,
                                       stdout_path.read_bytes(), stderr_path.read_bytes())
