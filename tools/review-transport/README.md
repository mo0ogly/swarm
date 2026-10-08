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

## Token counts (optional diagnostic)

Create a disposable virtual environment and install `token-requirements.txt`, then
run `python token_probe.py input.txt --model gpt-5.6-sol`. Run all diagnostic tests
with `python -m unittest discover -s tools/review-transport -p 'test_*.py'` from
the repository root using that environment. The tokenizer may download its public
vocabulary on first use; input text stays local. No AI request is made.

The output contains sizes and a digest, never the evidence text. Unknown model
mappings and invalid UTF-8 fail closed. Special-token-looking source text is
counted as ordinary data. Version 0.12.0 lacks the requested model mapping;
0.14.0 is pinned and maps it to `o200k_base`.

These are text token counts, **not complete provider request counts**. Client
instructions, message framing, output/reasoning reserves and effective model
configuration still need to be accounted for. This tool never changes the
engine's byte cap, authorizes admission, or reserves a review call.
