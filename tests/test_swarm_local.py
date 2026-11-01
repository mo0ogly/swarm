"""Real isolated native server lifecycle. No live mission database or provider calls."""
import hashlib
import http.cookiejar
import json
import os
import shutil
from pathlib import Path
import socket
import subprocess
import tempfile
from fixture_area import fixture_area
import threading
import unittest
import urllib.error
import urllib.request
import urllib.parse
from http.server import HTTPServer, BaseHTTPRequestHandler

SOURCE = Path(__file__).resolve().parent.parent
# Operator /tmp interdiction (2026-10-31): no project data under /tmp, and no
# default binary path pointing there either. The candidate binary now lives in
# the repository delivery area; override with SWARM_TEST_BINARY when needed.
BINARY = os.environ.get("SWARM_TEST_BINARY", str(SOURCE / "bin" / "swarm"))


class LocalWeb(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="swarm-local-site-", dir=fixture_area())
        self.root = Path(self.temp.name) / "project with spaces"
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.address = f"127.0.0.1:{sock.getsockname()[1]}"
        self.url = "http://" + self.address
        self.args = [str(SOURCE / "swarm.sh"), "--root", str(self.root),
                     "--address", self.address, "--binary", BINARY, "--no-open", "--no-mask-cifs"]

    def run_cli(self, action, code=0, extra=()):
        result = subprocess.run([*self.args, action, *extra], text=True, capture_output=True)
        self.assertEqual(result.returncode, code, result.stderr)
        return result.stdout

    def tearDown(self):
        subprocess.run([*self.args, "stop"], capture_output=True)
        self.temp.cleanup()

    def test_local_opener_gets_native_login_without_terminal_disclosure(self):
        self.run_cli("start")
        received = Path(self.temp.name) / "opener-url"
        opener = Path(self.temp.name) / "browser-opener"
        opener.write_text('#!/bin/sh\nprintf "%s" "$1" > "' + str(received) + '"\n')
        opener.chmod(0o700)
        args = [arg for arg in self.args if arg != "--no-open"]
        result = subprocess.run([*args, "open"], env={**os.environ, "SWARM_BROWSER": str(opener)},
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        key = (self.root / ".swarm" / ("web-session-" + hashlib.sha256(self.address.encode()).hexdigest()[:16])).read_text().strip()
        # Fresh Root carries the v50 ticket lifetime: open hands the opener a
        # single-use link (never printed), which signs in once and then refuses.
        link = received.read_text()
        self.assertTrue(link.startswith(self.url + "/login?ticket=ticket-"), link[:60])
        self.assertTrue(link.endswith("&next=/live.html"))
        ticket = link.split("ticket=", 1)[1].split("&", 1)[0]
        self.assertNotIn(key, link)
        self.assertNotIn(key, result.stdout + result.stderr)
        self.assertNotIn(ticket, result.stdout + result.stderr)
        self.assertIn(self.url + "/", result.stdout)
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, *args, **kwargs):
                return None
        client = urllib.request.build_opener(NoRedirect)
        with self.assertRaises(urllib.error.HTTPError) as first:
            client.open(link)
        self.assertEqual(first.exception.code, 303)
        self.assertEqual(first.exception.headers["Location"], "/live.html")
        self.assertIn("HttpOnly", first.exception.headers["Set-Cookie"])
        with self.assertRaises(urllib.error.HTTPError) as replay:
            client.open(link)
        self.assertEqual(replay.exception.code, 401)

    def switch_to_authority(self):
        """Adopt `authority` access mode on the isolated Root via the CLI."""
        run = lambda *a: json.loads(subprocess.run([BINARY, "--root", str(self.root), *a], text=True,
                                                   capture_output=True, check=True).stdout)
        shown = run("engine-settings", "show", "--json")
        values = {**shown["values"], "web_access_mode": "authority",
                  "web_authority_public_key": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"}
        change = {"schema_version": shown["schema_version"], "event_id": "launcher-authority-front",
                  "expected_revision": shown["revision"], "values": values,
                  "reason": "Launcher authority front test", "preview_token": ""}
        request = Path(self.temp.name) / "authority-change.json"
        request.write_text(json.dumps(change))
        change["preview_token"] = run("engine-settings", "preview", "--input", str(request), "--json")["preview_token"]
        request.write_text(json.dumps(change))
        run("engine-settings", "apply", "--input", str(request), "--json")

    def test_open_in_authority_mode_targets_daemon_sign_in(self):
        self.run_cli("start")
        self.switch_to_authority()
        received = Path(self.temp.name) / "opener-url"
        opener = Path(self.temp.name) / "browser-opener"
        opener.write_text('#!/bin/sh\nprintf "%s" "$1" > "' + str(received) + '"\n')
        opener.chmod(0o700)
        unit = Path(self.temp.name) / "swarm-authority.service"
        args = [arg for arg in self.args if arg != "--no-open"]
        env = {**os.environ, "SWARM_BROWSER": str(opener), "SWARM_AUTHORITY_UNIT": str(unit)}
        # Witness of the refusal first: no unit -> open refuses, no owner ticket.
        missing = subprocess.run([*args, "open"], env=env, text=True, capture_output=True)
        self.assertEqual(missing.returncode, 2, missing.stdout)
        self.assertIn("sans démon installé", missing.stderr)
        self.assertFalse(received.exists())
        status = subprocess.run([*self.args, "start"], env=env, text=True, capture_output=True)
        self.assertEqual(status.returncode, 0, status.stderr)
        self.assertIn("Avertissement", status.stderr)
        unit.write_text("[Service]\nExecStart=/x serve --pin P --listen 127.0.0.1:38999 --engine " + self.address + "\n")
        result = subprocess.run([*args, "open"], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(received.read_text(), "http://127.0.0.1:38999/login")
        self.assertIn("http://127.0.0.1:38999/", result.stdout)
        unit.write_text("[Service]\nExecStart=/x serve --listen 0.0.0.0:1\n")
        bad = subprocess.run([*args, "open"], env=env, text=True, capture_output=True)
        self.assertEqual(bad.returncode, 2)
        self.assertIn("introuvable", bad.stderr)

    def test_rotate_replaces_key_and_refuses_old_cookie(self):
        """rotate: owned server stopped, key replaced, server ready again; the
        old cookie and a ticket issued before rotation are refused, the new key
        works, and neither key appears in launcher output."""
        self.run_cli("start")
        name = "web-session-" + hashlib.sha256(self.address.encode()).hexdigest()[:16]
        old = (self.root / ".swarm" / name).read_text().strip()
        cookie = lambda key: {"Cookie": "swarm_session_" + hashlib.sha256(self.address.encode()).hexdigest()[:12] + "=" + key}
        probe = lambda key: urllib.request.urlopen(urllib.request.Request(self.url + "/api/v1/native-version", headers=cookie(key))).status
        self.assertEqual(probe(old), 200)
        stale = json.loads(subprocess.check_output([BINARY, "--root", str(self.root), "web-ticket", self.address, "--json"]))["url"]
        before = json.loads(self.run_cli("status"))["pid"]
        output = self.run_cli("rotate")
        new = (self.root / ".swarm" / name).read_text().strip()
        self.assertNotEqual(old, new)
        self.assertNotIn(old, output); self.assertNotIn(new, output)
        status = json.loads(self.run_cli("status"))
        self.assertTrue(status["ready"]); self.assertNotEqual(before, status["pid"])
        with self.assertRaises(urllib.error.HTTPError) as refused:
            probe(old)
        self.assertEqual(refused.exception.code, 403)
        self.assertEqual(probe(new), 200)
        with self.assertRaises(urllib.error.HTTPError) as orphan:
            urllib.request.urlopen(stale)
        self.assertEqual(orphan.exception.code, 401)

    def test_restart_retains_data_session_and_single_instance(self):
        first = self.run_cli("start")
        self.assertNotIn("/session/session-", first)
        before = json.loads(self.run_cli("status"))
        self.run_cli("start")
        self.assertEqual(before["pid"], json.loads(self.run_cli("status"))["pid"])
        key = (self.root / ".swarm" / ("web-session-" + hashlib.sha256(self.address.encode()).hexdigest()[:16])).read_text()
        jar = http.cookiejar.CookieJar()
        client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
        page = client.open(self.url + "/").read().decode()
        self.assertIn("Connexion à Swarm", page)
        self.assertNotIn(key, page)
        login=urllib.request.Request(self.url+'/login',data=urllib.parse.urlencode({'token':key,'next':'/'}).encode(),headers={'Origin':self.url})
        client.open(login).close()
        cookie = list(jar)[0]
        self.assertIsNotNone(cookie.expires)
        self.assertTrue(cookie.has_nonstandard_attr("HttpOnly"))
        request_path = self.root / "work.json"
        request_path.write_text(json.dumps({"schema_version": 1, "event_id": "stable-site-create", "expected_revision": 0,
                                          "title": "Persistent site", "objective": "Preserve", "scope": "Isolated lifecycle test", "criteria": ["State retained"]}))
        result = subprocess.run([BINARY, "--root", str(self.root), "--json", "work", "create", "--input", str(request_path)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        work = json.loads(result.stdout)["work"]
        self.run_cli("restart")
        after = json.loads(self.run_cli("status"))
        self.assertNotEqual(before["pid"], after["pid"])
        self.assertTrue(after["ready"])
        data = json.loads(client.open(self.url + "/api/v1/works").read())
        self.assertIn(work["id"], [w["id"] for w in data])
        self.assertNotIn(key, self.run_cli("logs"))
        self.run_cli("stop")
        self.run_cli("status", 1)
        self.assertTrue((self.root / ".swarm").is_dir())

    def test_native_version_is_protected_and_matches_compiled_cli(self):
        """Read compiled metadata/FR+EN notes with a cookie; no launch/mutation."""
        self.run_cli("start")
        key=(self.root/'.swarm'/('web-session-'+hashlib.sha256(self.address.encode()).hexdigest()[:16])).read_text().strip()
        import urllib.error
        with self.assertRaises(urllib.error.HTTPError):urllib.request.urlopen(self.url+'/api/v1/native-version')
        headers={'Cookie':'swarm_session_'+hashlib.sha256(self.address.encode()).hexdigest()[:12]+'='+key}
        request=urllib.request.Request(self.url+'/api/v1/native-version',headers=headers)
        actual=json.load(urllib.request.urlopen(request))
        cold=Path(self.temp.name)/'absent-root'
        cli=json.loads(subprocess.check_output([BINARY,'--root',str(cold),'--json','version']))
        self.assertEqual(actual['binary'],cli['binary'])
        notes=json.loads((SOURCE/'version_history.json').read_text())['local_changes']
        self.assertEqual(actual['history']['local_changes'],notes)
        self.assertFalse(cold.exists())
        head=urllib.request.Request(request.full_url,headers=headers,method='HEAD')
        self.assertEqual(urllib.request.urlopen(head).read(),b'')
        self.assertNotIn(key,self.run_cli('logs'))

    def test_existing_root_requires_explicit_native_initialization(self):
        """Missing native config refuses; public init repairs fixture without reset."""
        self.run_cli('start');self.assertTrue(json.loads(self.run_cli('status'))['ready'])
        self.run_cli('stop')
        settings=self.root/'.swarm/engine-settings.json';lock=self.root/'.swarm/.engine-settings.lock'
        settings.unlink();lock.unlink()
        self.run_cli('start',2)
        self.assertFalse(settings.exists());self.assertFalse(lock.exists())
        subprocess.run([BINARY,'--root',str(self.root),'init'],check=True,capture_output=True)
        self.run_cli('start');self.assertTrue(json.loads(self.run_cli('status'))['ready'])

    def test_legacy_automatic_lease_refuses_before_http_success(self):
        """Owned SQL fault after valid control; public pause preserves legacy rows."""
        self.run_cli('start');self.assertTrue(json.loads(self.run_cli('status'))['ready']);self.run_cli('stop')
        request=self.root/'legacy-create.json'
        request.write_text(json.dumps(dict(schema_version=1,event_id='legacy-create',expected_revision=0,title='Legacy startup fixture',objective='Refuse unsafe adoption',scope='Owned fixture',criteria=['No false readiness'])))
        created=json.loads(subprocess.check_output([BINARY,'--root',str(self.root),'--json','work','create','--input',str(request)]))['work']
        import sqlite3,datetime
        now=datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='seconds').replace('+00:00','Z')
        connection=sqlite3.connect(self.root/'.swarm/state.db')
        connection.execute("INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES(?,0,'autonome',1)",[created['id']])
        connection.execute("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'mission-policy',?)",[created['id'],now,json.dumps(dict(enabled=True,actor='owned-fixture',at=now))])
        connection.execute("INSERT INTO mission_supervision(work_id,conductor_id,source,started_at,heartbeat_at,reconciled_at,last_check_at,last_error,next_check_at,stopped_at) VALUES(?,'legacy-fixture','owned-fixture',?,?,'','','','','')",[created['id'],now,now]);connection.commit()
        original=connection.execute('SELECT * FROM mission_supervision').fetchall()
        self.run_cli('start',2)
        self.assertFalse(json.loads(self.run_cli('status',1))['running'])
        self.assertIn('mission-legacy-lease-requires-migration',self.run_cli('logs'))
        control=json.loads(subprocess.check_output([BINARY,'--root',str(self.root),'--json','mission','show',created['id']]))
        pause=self.root/'legacy-pause.json';pause.write_text(json.dumps(dict(schema_version=1,event_id='legacy-pause',expected_revision=control['revision'],reason='Explicit fixture operator pause')))
        subprocess.run([BINARY,'--root',str(self.root),'--json','mission','pause',created['id'],'--input',str(pause)],check=True,capture_output=True)
        self.run_cli('start');self.assertTrue(json.loads(self.run_cli('status'))['ready'])
        self.assertEqual(original,connection.execute('SELECT * FROM mission_supervision').fetchall());connection.close()

    def test_foreign_listener_is_not_stopped(self):
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200); self.end_headers(); self.wfile.write(b"unrelated")
            def log_message(self, *args):
                pass
        host, port = self.address.split(":")
        server = HTTPServer((host, int(port)), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            self.run_cli("start", 2)
            self.assertEqual(urllib.request.urlopen(self.url).read(), b"unrelated")
        finally:
            server.shutdown(); server.server_close(); thread.join()

    def test_restart_after_atomic_binary_replacement(self):
        binary = Path(self.temp.name) / "candidate"
        shutil.copy2(BINARY, binary)
        self.args[self.args.index("--binary") + 1] = str(binary)
        self.run_cli("start")
        replacement = binary.with_suffix(".new")
        shutil.copy2(BINARY, replacement)
        replacement.replace(binary)
        self.assertTrue(json.loads(self.run_cli("status"))["running"])
        self.run_cli("restart")
        self.assertTrue(json.loads(self.run_cli("status"))["ready"])

    def test_configure_retains_installed_binary_without_rebuilding(self):
        checkout = Path(self.temp.name) / "launcher checkout"
        (checkout / "tools").mkdir(parents=True)
        shutil.copy2(SOURCE / "swarm.sh", checkout / "swarm.sh")
        shutil.copy2(SOURCE / "tools/swarm_local.py", checkout / "tools/swarm_local.py")
        shutil.copy2(SOURCE / "tools/swarm_version.py", checkout / "tools/swarm_version.py")
        configured = [str(checkout / "swarm.sh")]
        result = subprocess.run([*configured, "configure", "--root", str(self.root),
                                 "--address", self.address, "--binary", BINARY],
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        settings = json.loads((checkout / "deploy/local-web.json").read_text())
        self.assertEqual(settings["binary"], str(Path(BINARY).resolve()))
        try:
            for action in ["start", "restart"]:
                result = subprocess.run([*configured, action, "--no-open"], text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                status = json.loads(subprocess.check_output([*configured, "status"], text=True))
                self.assertTrue(status["ready"])
                self.assertEqual(Path(f'/proc/{status["pid"]}/exe').resolve(), Path(BINARY).resolve())
        finally:
            subprocess.run([*configured, "stop"], capture_output=True)

    def test_authority_enable_disable_switches_engine_and_daemon(self):
        """`./swarm.sh authority` on an isolated checkout: refusal witness for an
        outdated installed daemon (no effect), then enable/disable through the
        public commands; sudo is a logging stub, never the real one."""
        checkout = Path(self.temp.name) / "launcher checkout"
        (checkout / "tools").mkdir(parents=True)
        for name in ["swarm.sh", "tools/swarm_local.py", "tools/swarm_version.py"]:
            shutil.copy2(SOURCE / name, checkout / name)
        launcher = [str(checkout / "swarm.sh")]
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            internal = f"127.0.0.1:{sock.getsockname()[1]}"
        fake = Path(self.temp.name) / "authority"
        (fake / "bin").mkdir(parents=True)
        sudo_log = fake / "sudo.log"
        (fake / "bin" / "sudo").write_text('#!/bin/sh\necho "$*" >> "' + str(sudo_log) + '"\n')
        (fake / "bin" / "sudo").chmod(0o700)
        unit = fake / "swarm-authority.service"
        unit.write_text(f"[Service]\nExecStart=/x serve --pin P --listen {self.address} --engine {internal}\n")
        (fake / "etc").mkdir()
        (fake / "etc" / "public_key").write_text("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8\n")
        installed = fake / "installed"
        installed.write_bytes(b"daemon v1")
        built = checkout / "rust/target/release/swarm-authority"
        built.parent.mkdir(parents=True)
        built.write_bytes(b"daemon v2")
        env = {**os.environ, "PATH": str(fake / "bin") + os.pathsep + os.environ["PATH"], "SWARM_AUTHORITY_UNIT": str(unit),
               "SWARM_AUTHORITY_ETC": str(fake / "etc"), "SWARM_AUTHORITY_INSTALLED": str(installed)}
        run = lambda *a, code=0: self.assertEqual((r := subprocess.run([*launcher, *a], env=env, text=True, capture_output=True)).returncode, code, r.stderr) or r
        mode = lambda: json.loads(subprocess.check_output([BINARY, "--root", str(self.root), "engine-settings", "show", "--json"]))["values"]["web_access_mode"]
        run("configure", "--root", str(self.root), "--address", self.address, "--binary", BINARY, "--no-mask-cifs")
        try:
            run("start", "--no-open")
            refused = run("authority", "enable", "--no-open", code=2)
            self.assertIn("diffère du dernier build", refused.stderr)
            self.assertEqual((mode(), sudo_log.exists()), ("owner-key", False), "refusal had no effect")
            self.assertTrue(json.loads(run("status").stdout)["ready"])
            built.write_bytes(b"daemon v1")
            run("authority", "enable", "--no-open")
            self.assertEqual(mode(), "authority")
            self.assertEqual(sudo_log.read_text().splitlines(), ["systemctl enable --now swarm-authority"])
            status = json.loads(run("status").stdout)
            self.assertEqual((status["url"], status["ready"]), (f"http://{internal}/", True))
            report = json.loads(run("authority", "status").stdout)
            self.assertEqual((report["engine_mode"], report["engine_address"], report["installed_matches_build"]), ("authority", internal, True))
            run("authority", "enable", "--no-open")  # rerun: completed steps are no-ops
            self.assertEqual(mode(), "authority")
            restored = run("authority", "accept-restore", "--no-open")
            self.assertIn("Restauration acceptée", restored.stdout)
            self.assertTrue((self.root / ".swarm" / "web-attribution.jsonl").exists(), "reset recorded in the Root ledger")
            self.assertTrue(json.loads(run("status").stdout)["ready"])
            run("authority", "disable", "--no-open")
            self.assertEqual(mode(), "owner-key")
            self.assertEqual(sudo_log.read_text().splitlines()[-1], "systemctl disable --now swarm-authority")
            status = json.loads(run("status").stdout)
            self.assertEqual((status["url"], status["ready"]), (f"http://{self.address}/", True))
        finally:
            subprocess.run([*launcher, "stop"], env=env, capture_output=True)

    def test_stale_pid_does_not_kill_unrelated_process(self):
        sleeper = subprocess.Popen(["sleep", "60"])
        try:
            directory = self.root / ".swarm" / "local-web" / hashlib.sha256(self.address.encode()).hexdigest()[:16]
            directory.mkdir(parents=True)
            (directory / "server.json").write_text(json.dumps({"pid": sleeper.pid, "identity": {"pid": sleeper.pid, "start": "invalid", "exe": "wrong", "cmd": "wrong"}}))
            self.run_cli("stop")
            self.assertIsNone(sleeper.poll())
        finally:
            sleeper.terminate(); sleeper.wait()

    def test_explicit_adoption_checks_exact_server_and_replaces_it(self):
        self.root.mkdir(parents=True)
        subprocess.run([BINARY,"--root",str(self.root),"init"],check=True,capture_output=True)
        old = subprocess.Popen([BINARY,"--root",str(self.root),"web",self.address],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        try:
            import time
            for _ in range(100):
                try:
                    urllib.request.urlopen(self.url,timeout=.2).close(); break
                except OSError:
                    time.sleep(.05)
            self.run_cli("adopt",2,extra=("--pid",str(os.getpid())))
            self.assertIsNone(old.poll())
            self.run_cli("adopt",extra=("--pid",str(old.pid)))
            self.run_cli("restart")
            old.wait()
            self.assertTrue(json.loads(self.run_cli("status"))["ready"])
        finally:
            if old.poll() is None:
                old.send_signal(__import__("signal").SIGINT); old.wait()

    @unittest.skipUnless(shutil.which("bwrap"), "bwrap not installed")
    def test_mount_safe_restart_releases_listener(self):
        self.run_cli("start", extra=("--mask-cifs",))
        self.assertTrue(json.loads(self.run_cli("status"))["ready"])
        self.run_cli("restart", extra=("--mask-cifs",))
        self.assertTrue(json.loads(self.run_cli("status"))["ready"])


if __name__ == "__main__":
    unittest.main()
