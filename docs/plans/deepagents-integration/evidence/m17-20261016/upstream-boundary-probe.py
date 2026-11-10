"""Execute one pinned, dependency-free AST method; no middleware/model run."""
import ast
import hashlib
import json
from pathlib import Path

out = Path(__file__).resolve().parent
source = out.parent / "m16-20261016/upstream-summarization.py.txt"
metadata = json.loads((out.parent / "m16-20261016/upstream.json").read_text())
assert hashlib.sha256(source.read_bytes()).hexdigest() == metadata["sha256"]
module = ast.parse(source.read_text())
cls = next(node for node in module.body if isinstance(node, ast.ClassDef) and any(isinstance(n, ast.FunctionDef) and n.name == "_apply_event_to_messages" for n in node.body))
method = next(n for n in cls.body if isinstance(n, ast.FunctionDef) and n.name == "_apply_event_to_messages")
method.decorator_list = []
method.returns = None
for arg in method.args.args:
    arg.annotation = None
warnings = []
class Logger:
    def warning(self, message, *args):
        warnings.append(message % args)
scope = {"logger": Logger()}
exec(compile(ast.fix_missing_locations(ast.Module(body=[method], type_ignores=[])), str(source), "exec"), scope)
run = scope["_apply_event_to_messages"]
messages = ["m1", "m2", "m3"]
observations = []
for cutoff in [2, -1, 4, True]:
    actual = run(messages, {"summary_message": "summary", "cutoff_index": cutoff})
    observations.append({"cutoff": cutoff, "actual": actual})
assert observations[0]["actual"] == ["summary", "m3"]
assert observations[1]["actual"] == ["summary", "m3"]
assert observations[2]["actual"] == ["summary"]
assert observations[3]["actual"] == ["summary", "m2", "m3"]
result = {"status": "PASS", "scope": "Pinned static method extracted from AST, not the installed middleware", "upstream_commit": metadata["commit"], "upstream_sha256": metadata["sha256"], "observations": observations, "warnings": warnings, "model_calls": 0}
(out / "upstream-boundary.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
