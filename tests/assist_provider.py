#!/usr/bin/python3
"""Fixture provider for the page assistant recette.

Reads the exact prompt on stdin, grounds its reply in that prompt (context hash,
template id, fact identifiers, allowed actions) and emits one provider event.
The behaviour is chosen by argv[1] so the browser recette can exercise a valid
answer, an unknown reference, an action outside the catalogue and malformed JSON
without any billed model call.
"""
import json
import re
import sys

mode = sys.argv[1] if len(sys.argv) > 1 else "ok"
prompt = sys.stdin.read()
if mode in ("slow", "restart", "timeout"):
    import time
    time.sleep(10 if mode=="restart" else 30)

template = re.search(r'"template_id":"([^"]+)"', prompt)
digest = re.search(r'"context_hash":"([^"]+)"', prompt)
facts = re.findall(r'"id": "(f\d+)"', prompt)
catalogue = re.search(r"catalogue : ([^\n]+)", prompt)
actions = [a.strip() for a in catalogue.group(1).split(",")] if catalogue else []
actions = [a for a in actions if a and "aucune action" not in a]

answer = {
    "version": 1,
    "template_id": template.group(1) if template else "understand_page.v1",
    "context_hash": digest.group(1) if digest else "",
    "facts": [{"text": "Fixture de recette : lecture des faits transmis.",
               "source_ids": facts[:2] or ["f1"]}],
    "interpretation": "Reponse de fixture, aucune conclusion metier.",
    "missing_information": ["Contenu des preuves non transmis."],
    "next_steps": ([{"action_id": actions[0], "why": "Verifier la preuve avant toute acceptation.",
                     "source_ids": facts[:1]}] if actions else []),
    "limitations": ["Cette fixture ne demontre rien sur le livrable."],
    "questions": [],
}

if mode == "unknown_ref":
    answer["facts"][0]["source_ids"] = ["f999"]
if mode == "unknown_action":
    answer["next_steps"] = [{"action_id": "task.delete", "why": "Action inventee.", "source_ids": facts[:1]}]
if mode == "stale":
    answer["context_hash"] = "empreinte-perimee"

if mode == "bad_json":
    body = "Je ne peux pas produire de JSON ici."
elif mode == "fenced":
    body = "Voici ma reponse :\n```json\n" + json.dumps(answer, ensure_ascii=False) + "\n```\nFin."
else:
    body = json.dumps(answer, ensure_ascii=False)

print(json.dumps({"type": "result", "subtype": "success", "result": body,
                  "usage": {"input_tokens": 120, "output_tokens": 60}}), flush=True)
