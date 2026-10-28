"""Check constructor forms present in the archived upstream memory examples."""
from pathlib import Path
from deepagents.backends import StateBackend,StoreBackend
snapshot=(Path(__file__).parent/'upstream-memory-doc.md').read_text()
assert 'StateBackend(rt)' in snapshot
for label,call in [('StateBackend(rt)',lambda:StateBackend(object())),('StoreBackend(rt, namespace=...)',lambda:StoreBackend(object(),namespace=lambda _rt:('example',)))]:
    try:
        call()
    except TypeError as exc:
        print(label+': '+type(exc).__name__+': '+str(exc))
    else:
        raise AssertionError(label+' accepted unexpectedly')
print('OBSERVED: constructor forms in the archived full examples are incompatible with pinned SDK 0.7.25; no model was called')
