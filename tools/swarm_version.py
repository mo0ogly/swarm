"""Prepare numbered local deliveries without changing published release tags."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile
import http.cookiejar
import urllib.request


def json_stream(raw):
    decoder = json.JSONDecoder()
    offset = 0
    while offset < len(raw):
        while offset < len(raw) and raw[offset].isspace():
            offset += 1
        if offset == len(raw):
            return
        value, offset = decoder.raw_decode(raw, offset)
        yield value


def package_inputs(source, packages, overlay):
    source = source.resolve()
    rows = {}
    for package in packages:
        directory = Path(package["Dir"]).resolve()
        if not directory.is_relative_to(source):
            continue
        for group in ("GoFiles", "CgoFiles", "EmbedFiles"):
            for name in package.get(group, []):
                original = directory / name
                replacement = overlay.get(str(original), str(original))
                if not replacement:
                    raise ValueError("An overlay removes a required build input: " + str(original.relative_to(source)))
                rows[original.relative_to(source).as_posix()] = hashlib.sha256(Path(replacement).read_bytes()).hexdigest()
    return rows


def source_identity(source):
    environment = {**os.environ, "CGO_ENABLED": "0"}
    configuration = json.loads(subprocess.check_output(["go", "env", "-json", "GOVERSION", "GOOS", "GOARCH", "GOFLAGS"], cwd=source, env=environment, text=True))
    flags = shlex.split(configuration["GOFLAGS"])
    overlay = {}
    for i, flag in enumerate(flags):
        if flag == "-overlay" or flag.startswith("-overlay="):
            name = flags[i + 1] if flag == "-overlay" else flag.split("=", 1)[1]
            path = Path(name)
            if not path.is_absolute():
                path = source / path
            for original, replacement in json.loads(path.read_text()).get("Replace", {}).items():
                original = str((source / original).resolve())
                overlay[original] = str((source / replacement).resolve()) if replacement else ""
    raw = subprocess.check_output(["go", "list", "-deps", "-json", "./cmd/swarm"], cwd=source, env=environment, text=True)
    rows = package_inputs(source, json_stream(raw), overlay)
    for name in ("go.mod", "go.sum", "build.sh", "tools/swarm_version.py", "tools/swarm_local.py"):
        rows[name] = hashlib.sha256((source / name).read_bytes()).hexdigest()
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    modified = bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=source, text=True))
    for key in ("SWARM_COMMIT", "SWARM_MODIFIED", "SWARM_BUILD_DATE", "SOURCE_DATE_EPOCH"):
        configuration[key] = os.environ.get(key)
    # Package concurrency changes build scheduling, never the produced program.
    effective_flags = []
    skip = False
    for flag in flags:
        if skip:
            skip = False
        elif flag == "-p":
            skip = True
        elif not flag.startswith("-p="):
            effective_flags.append(flag)
    configuration["GOFLAGS"] = shlex.join(effective_flags)
    digest = hashlib.sha256(json.dumps({"commit": commit, "modified": modified, "configuration": configuration, "inputs": rows}, sort_keys=True).encode()).hexdigest()
    return digest, rows


def select_version(published, digest, previous):
    match = re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:[-+].*)?", published)
    if not match:
        raise ValueError("A valid published base version is required")
    major, minor, patch = map(int, match.groups())
    prefix = f"v{major}.{minor}.{patch + 1}-dev."
    previous = previous or {}
    prior = previous.get("version", "")
    if prior.startswith(prefix) and prior[len(prefix):].isdigit():
        if previous.get("source_digest") == digest:
            return prior
        return prefix + str(int(prior[len(prefix):]) + 1)
    return prefix + "1"


def verify_candidate(candidate, version, notes):
    print("Vérification isolée du numéro et des évolutions…", flush=True)
    with tempfile.TemporaryDirectory(prefix="swarm-local-build-") as root:
        subprocess.run([str(candidate), "--root", root, "init"], check=True, stdout=subprocess.DEVNULL)
        app = subprocess.Popen([str(candidate), "--root", root, "web", "127.0.0.1:0"], stdout=subprocess.PIPE, text=True)
        try:
            while True:
                line = app.stdout.readline()
                match = re.search(r"http://\S+/session/\S+", line)
                if match:
                    break
                if app.poll() is not None:
                    raise RuntimeError("Candidate server exited before readiness; previous server retained")
            session = match.group(0)
            client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
            client.open(session).read()
            health = json.loads(client.open(session.split("/session/")[0] + "/api/v1/runtime-health").read())
            actual = health["version"]
            if actual["binary"]["version"] != version or not actual["history"]["available"] or actual["history"].get("local_changes") != notes:
                raise ValueError("Candidate version/evolutions rejected by the real server; previous server retained")
        finally:
            app.terminate()
            app.wait()


def prepare(source, binary):
    """Caller holds the shared build lock. Live binaries are never overwritten."""
    manifest = json.loads((source / "version_history.json").read_text())
    notes = manifest.get("local_changes")
    valid_notes = isinstance(notes, dict) and isinstance(notes.get("items"), list) and bool(notes["items"])
    if valid_notes:
        valid_notes = all(isinstance(note, dict) and all(isinstance(note.get(lang), str) and note[lang].strip() for lang in ("fr", "en")) for note in [notes.get("summary"), *notes["items"]])
    if not valid_notes:
        raise ValueError("Update version_history.json local_changes in French and English before a local delivery")
    digest, inputs = source_identity(source)
    state_path = source / "deploy/local-build.json"
    previous = json.loads(state_path.read_text()) if state_path.exists() else None
    published = manifest["releases"][0]["version"] if manifest.get("releases") else "v0.0.0"
    version = os.environ["SWARM_VERSION"] if "SWARM_VERSION" in os.environ else select_version(published, digest, previous)
    if previous and previous.get("source_digest") == digest and previous.get("version") == version:
        for artifact in (binary, Path(previous.get("artifact", ""))):
            if artifact.is_file() and hashlib.sha256(artifact.read_bytes()).hexdigest() == previous.get("binary_sha256"):
                print("Version locale inchangée : " + version, flush=True)
                return artifact, previous
    binary.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=binary.name + ".candidate-", dir=binary.parent)
    os.close(fd)
    candidate = Path(name)
    try:
        subprocess.run(["sh", str(source / "build.sh"), str(candidate)], cwd=source,
                       env={**os.environ, "SWARM_VERSION": version}, check=True)
        after, _ = source_identity(source)
        if after != digest:
            raise RuntimeError("Build inputs changed during compilation; running server retained. Recheck the changes before retrying.")
        metadata = json.loads(subprocess.check_output([str(candidate), "--json", "version"], cwd=source, text=True))["binary"]
        if metadata["version"] != version:
            raise RuntimeError("Built binary version differs from the requested delivery")
        verify_candidate(candidate, version, notes)
        state = {"schema_version": 1, "version": version, "source_digest": digest,
                 "inputs": inputs, "binary_sha256": hashlib.sha256(candidate.read_bytes()).hexdigest(),
                 "artifact": str(candidate), "binary": metadata,
                 "prepared_at": datetime.datetime.now(datetime.timezone.utc).isoformat()}
        state_path.parent.mkdir(parents=True, exist_ok=True)
        temporary = state_path.with_suffix(".tmp")
        temporary.write_text(json.dumps(state, indent=2) + "\n")
        temporary.replace(state_path)
        print("Livraison locale préparée : " + version, flush=True)
        return candidate, state
    except BaseException:
        candidate.unlink(missing_ok=True)
        raise
