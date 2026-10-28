"""Compare the candidate's authenticated UI assets with repository sources."""
from pathlib import Path
import datetime
import hashlib
import http.cookiejar
import json
import re
import subprocess
import sys
import tempfile
import urllib.request

repo = Path(__file__).resolve().parents[4]
binary = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2])
root = tempfile.mkdtemp(prefix="swarm-ui-integrity-")
subprocess.run([str(binary), "--root", root, "--json", "init"],
               stdout=subprocess.DEVNULL, check=True)
app = subprocess.Popen([str(binary), "--root", root, "web", "127.0.0.1:0"],
                       stdout=subprocess.PIPE, text=True)
paths = ["index.html", "text-reader.js", "cockpit.js", "pilot-inspector.js",
         "instruction-revisions.js", "mission.js", "pilot-actions.js",
         "swarm-design.css", "log-console.html", "log-console.js",
         "log-console.css", "journal-console.js", "i18n-en.js"]
digest = lambda data: hashlib.sha256(data).hexdigest()
rows = []
try:
    while True:
        match = re.search(r"http://\S+/session/\S+", app.stdout.readline())
        if match:
            break
        if app.poll() is not None:
            raise RuntimeError("Fixture server exited before readiness")
    session = match.group(0)
    base = session.split("/session/")[0]
    client = urllib.request.build_opener(
        urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    client.open(session).read()
    for name in paths:
        route = "/" if name == "index.html" else "/" + name
        response = client.open(base + route)
        served = response.read()
        source = (repo / "web" / name).read_bytes()
        expected = source
        transformation = "none"
        if b"SWARM_STYLE_NONCE" in source:
            # The authoritative web_terminal.go handler replaces this marker.
            nonce = re.search(rb'name="swarm-style-nonce" content="([A-Za-z0-9+/=_-]+)"', served)
            assert nonce and b"SWARM_STYLE_NONCE" not in served
            assert "'nonce-" + nonce[1].decode() + "'" in response.headers["Content-Security-Policy"]
            expected = source.replace(b"SWARM_STYLE_NONCE", nonce[1])
            transformation = "documented CSP style nonce substitution only"
        rows.append({"path": "web/" + name, "source_sha256": digest(source),
                     "served_sha256": digest(served), "expected_sha256": digest(expected),
                     "transformation": transformation, "matches": expected == served})
    receipt = {"status": "PASS" if all(r["matches"] for r in rows) else "FAIL",
               "time": datetime.datetime.now(datetime.timezone.utc).isoformat(),
               "binary": str(binary), "binary_sha256": digest(binary.read_bytes()),
               "root": root, "inputs": rows,
               "limits": "Local immutable UI candidate; no live server/provider or backend acceptance. Source resources.go embeds canonical web files. CSP nonce validated but never stored."}
    output.write_text(json.dumps(receipt, indent=2) + "\n")
    assert receipt["status"] == "PASS", "UI source mismatch; see receipt"
    print("PASS", len(rows), "served UI assets, with the documented nonce substitution")
finally:
    app.terminate()
    app.wait()
