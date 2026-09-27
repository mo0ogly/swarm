# Offline lossless transport probe

This experiment measures JSON escaping and exact repetition in an exported review
context. It does not open the Swarm database, call a provider, reserve budget, send
a prompt, or write back to its input. It is **not a production wire protocol**.

```sh
python3 tools/review-transport/probe.py /path/to/exported-context.json
python3 -m unittest discover -s tools/review-transport -v
```

Long strings are moved into byte-length-framed UTF-8 blocks. Exact duplicates share
a block; each replacement retains its original JSON path. Decode validates block
hashes, unique paths, complete references, and the reconstructed canonical digest.
This preserves values and contents, not the input JSON file's whitespace.

A positive byte saving does not prove fewer model calls: prompt instructions,
schema, per-packet boundaries, reply limits and final evidence selection still
have to fit. Small inputs may become larger. Existing journals and their prompt
hashes must retain their original rendering if this idea is later integrated.

A blocked, unpaid review context can be exported through the read-only
`planning review-dossier WORK --input request.json` operation (`{"task_id":"TASK"}`)
or the authenticated planning API's `action=review-dossier`. The response's
`context` property is the canonical document to measure. Keep these exports local:
they contain original source text and reports. The export is not a review verdict.
