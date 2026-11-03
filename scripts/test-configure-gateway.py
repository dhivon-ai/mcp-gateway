"""Exercise the published helper, including hard stops at private publication boundaries."""
import os
from pathlib import Path
import runpy
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent
ORIGIN = "https://gateway.example.test:18443"
EMAIL = "owner+setup@example.test"
PASSWORD = "synthetic-test-value"


class Configure(unittest.TestCase):
    def setUp(self):
        fixture = tempfile.TemporaryDirectory(dir=os.getenv("DHIVON_MCP_GATEWAY_BUILD_CACHE"))
        self.addCleanup(fixture.cleanup)
        self.fixture = Path(fixture.name)
        self.repo = self.checkout("repo")
        self.config = self.fixture / "config"
        self.private = self.config / "dhivon-mcp-gateway"

    def checkout(self, name):
        repo = self.fixture / name
        (repo / "scripts").mkdir(parents=True)
        for script in ("configure-gateway.sh", "configure-gateway.py"):
            shutil.copy2(SCRIPTS / script, repo / "scripts")
        return repo

    def process(self, repo=None, fault=None, config=None):
        repo = repo or self.repo
        command = ["bash", "scripts/configure-gateway.sh"]
        if fault:
            # Fault injection runs in the test child only. Production has no switches.
            command = [sys.executable, "-c", "import os, runpy, signal\n" + fault
                       + "\nrunpy.run_path('scripts/configure-gateway.py', run_name='__main__')"]
        return subprocess.Popen(command, cwd=repo, text=True, stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                env={**os.environ, "XDG_CONFIG_HOME": str(config or self.config)})

    def run_helper(self, origin=ORIGIN, email=EMAIL, password=PASSWORD, **kwargs):
        process = self.process(**kwargs)
        out, err = process.communicate(f"{origin}\n{email}\n{password}\n", timeout=10)
        self.assertNotIn(password, out + err)
        return process.returncode

    def assert_complete(self, email=EMAIL, password=PASSWORD):
        settings = (self.repo / ".env").read_text()
        self.assertEqual(settings, (self.private / "compose.env").read_text())
        self.assertIn(f"DHIVON_MCP_GATEWAY_PUBLIC_URL='{ORIGIN}'\n", settings)
        self.assertIn(f"DHIVON_MCP_GATEWAY_SETUP_DIR='{self.private}'\n", settings)
        self.assertEqual((self.private / "admin-email").read_text(), email + "\n")
        self.assertEqual((self.private / "admin-password").read_text(), password)
        for path in [self.private, self.repo / ".env", *self.private.iterdir()]:
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700 if path.is_dir() else 0o600)
        self.assertFalse(list(self.config.glob(".dhivon-setup-*")))

    def test_invalid_then_corrected_inputs(self):
        for email in ("a..b@example.com", ".a@host", "a.@host", "a@bad..test", "a@bad-.test",
                      "a" * 65 + "@test", "a@" + "b" * 64, "a@b@c", "a@café"):
            with self.subTest(email=email):
                self.assertNotEqual(self.run_helper(email=email), 0)
        for origin in ("https://gateway.test:99999", "https://gateway.test:0",
                       "https://gateway..test", "https://gateway.test/path", "https://a@b",
                       "https://1.2.3.999", "https://host.0x80", "http://gateway.test"):
            with self.subTest(origin=origin):
                self.assertNotEqual(self.run_helper(origin=origin), 0)
        self.assertEqual(list(self.config.iterdir()), [])
        self.assertFalse((self.repo / ".env").exists())
        self.assertEqual(self.run_helper(), 0)
        self.assert_complete()

    def test_saved_default_and_nondefault_ports(self):
        for index, (port, expected) in enumerate((("00443", ""), ("000443", ""), ("443", ""),
                                                  ("08443", ":8443"), ("8443", ":8443"))):
            repo = self.checkout(f"ports-{index}")
            config = self.fixture / f"config-{index}"
            self.assertEqual(self.run_helper(origin=f"https://Gateway.test:{port}",
                                             repo=repo, config=config), 0)
            self.assertIn(f"DHIVON_MCP_GATEWAY_PUBLIC_URL='https://gateway.test{expected}'\n",
                          (repo / ".env").read_text())

    def test_resume_settings_saved_with_an_older_port_spelling(self):
        self.assertEqual(self.run_helper(), 0)
        (self.repo / ".env").unlink()
        saved = self.private / "compose.env"
        previous = saved.read_text().replace(":18443", ":08443")
        saved.write_text(previous)
        self.assertEqual(self.run_helper(), 0)
        self.assertEqual((self.repo / ".env").read_text(), previous)
        self.assertEqual((self.private / "admin-password").read_text(), PASSWORD)

    def test_gateway_idna_rules(self):
        validate = runpy.run_path(str(SCRIPTS / "configure-gateway.py"))["origin_value"]
        self.assertEqual(validate("https://xn--fa-hia.example.test:08443"),
                         "https://xn--fa-hia.example.test:8443")
        with self.assertRaisesRegex(ValueError, "valid HTTPS hostname"):
            validate("https://xn--a.example.test")

    def test_hard_stop_before_private_commit(self):
        fault = """from pathlib import Path
write = Path.write_bytes
def interrupt(path, data):
    result = write(path, data)
    if path.name == 'admin-password':
        os.kill(os.getpid(), signal.SIGKILL)
    return result
Path.write_bytes = interrupt"""
        self.assertEqual(self.run_helper(password="old-uncommitted-password", fault=fault), -signal.SIGKILL)
        self.assertFalse(self.private.exists())
        self.assertFalse((self.repo / ".env").exists())
        self.assertEqual(self.run_helper(), 0)
        self.assert_complete()

    def test_hard_stop_after_private_commit(self):
        fault = "os.link = lambda *args: os.kill(os.getpid(), signal.SIGKILL)"
        self.assertEqual(self.run_helper(fault=fault), -signal.SIGKILL)
        self.assertFalse((self.repo / ".env").exists())
        second = self.checkout("foreign")
        self.assertNotEqual(self.run_helper(repo=second), 0)
        self.assertFalse((second / ".env").exists())
        self.assertEqual(self.run_helper(password="different-contender-password"), 0)
        self.assert_complete()
        old = (self.repo / ".env").stat().st_ino
        self.assertNotEqual(self.run_helper(), 0)
        self.assertEqual((self.repo / ".env").stat().st_ino, old)

    def test_refuses_corrupted_private_inputs_and_keeps_deployed_state(self):
        self.assertEqual(self.run_helper(), 0)
        (self.repo / ".env").unlink()
        admin_password = self.private / "admin-password"
        admin_password.write_bytes(b"sensitive-private-input\xff")
        process = self.process()
        out, err = process.communicate()
        self.assertNotEqual(process.returncode, 0)
        self.assertNotIn("sensitive-private-input", out + err)
        self.assertFalse((self.repo / ".env").exists())
        admin_password.write_text(PASSWORD)
        credential = self.private / "postgres" / "password"
        credential.write_text("existing-deployment")
        self.assertNotEqual(self.run_helper(), 0)
        self.assertEqual(credential.read_text(), "existing-deployment")
        self.assertFalse((self.repo / ".env").exists())

    def test_excludes_symlink_and_case_aliases_of_checkout(self):
        alias = self.fixture / "alias"
        alias.symlink_to(self.repo, target_is_directory=True)
        aliases = [alias]
        case_alias = self.repo.with_name(self.repo.name.upper())
        if case_alias.exists() and case_alias.samefile(self.repo):
            aliases.append(case_alias)
        for path in aliases:
            self.assertNotEqual(self.run_helper(config=path / "subdir"), 0)
            self.assertFalse((self.repo / "subdir").exists())
            self.assertFalse((self.repo / ".env").exists())

    def test_concurrent_checkouts_and_config_locations(self):
        first = self.process()
        try:
            # The holder has both locks and is waiting to receive its own inputs.
            self.assertEqual(first.stderr.read(6), "Public")
            self.assertNotEqual(self.run_helper(repo=self.checkout("foreign")), 0)
            self.assertNotEqual(self.run_helper(config=self.fixture / "other-config"), 0)
            self.assertNotEqual(self.run_helper(email="loser@example.test"), 0)
            first.communicate(f"{ORIGIN}\n{EMAIL}\n{PASSWORD}\n", timeout=10)
            self.assertEqual(first.returncode, 0)
        finally:
            if first.poll() is None:
                first.kill()
                first.communicate()
        self.assert_complete()

    def test_foreign_state_and_existing_env(self):
        self.private.mkdir(parents=True, mode=0o700)
        unrelated = self.private / "compose.env"
        unrelated.write_text("foreign settings")
        self.assertNotEqual(self.run_helper(), 0)
        self.assertEqual(unrelated.read_text(), "foreign settings")
        self.assertFalse((self.repo / ".env").exists())
        target = self.repo / ".env"
        target.symlink_to(self.fixture / "absent")
        self.assertNotEqual(self.run_helper(), 0)
        self.assertTrue(target.is_symlink())

    def test_foreign_directory_created_during_configuration(self):
        first = self.process()
        try:
            self.assertEqual(first.stderr.read(6), "Public")
            self.private.mkdir()
            inode = self.private.stat().st_ino
            first.communicate(f"{ORIGIN}\n{EMAIL}\n{PASSWORD}\n", timeout=10)
            self.assertNotEqual(first.returncode, 0)
            self.assertFalse((self.repo / ".env").exists())
            self.assertEqual(self.private.stat().st_ino, inode)
            self.assertEqual(list(self.private.iterdir()), [])
        finally:
            if first.poll() is None:
                first.kill()
                first.communicate()


if __name__ == "__main__":
    unittest.main()
