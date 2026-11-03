"""Publish complete inputs for the root Compose deployment on Linux or macOS."""
import ctypes
import fcntl
import getpass
import ipaddress
import os
from pathlib import Path
import re
import stat
import sys
import tempfile
import warnings
from contextlib import contextmanager

RECOVERY = "Unrecognized private setup. Restore the matching .env and private directory, or choose a fresh XDG_CONFIG_HOME."
EXISTING = "Keeping the existing .env. To start its deployment, run docker compose up -d --build --wait."
LABEL = r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?"


def domain_ok(host):
    return len(host) <= 253 and all(re.fullmatch(LABEL, part) for part in host.split("."))


def email_value(raw):
    # Match identity::email, including its ASCII dot-atom and DNS label limits.
    value = raw.strip()
    local, _, domain = value.partition("@")
    if (len(value) > 254 or not domain_ok(domain) or len(local) > 64
            or not re.fullmatch(r"[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+)*", local)):
        raise ValueError("Enter a valid administrator email.")
    return value.lower()


def origin_value(raw):
    match = re.fullmatch(r"https://([A-Za-z0-9.-]+)(?::([0-9]{1,5}))?", raw)
    if match:
        host, port = match.groups()
        if domain_ok(host) and (port is None or 1 <= int(port) <= 65535):
            # WHATWG URL parsing treats numeric final labels (including hex) as IPv4.
            # Require canonical dotted IPv4 there so the server uses the same host.
            if re.fullmatch(r"[0-9]+|0[xX][0-9a-fA-F]*", host.split(".")[-1]):
                try:
                    if str(ipaddress.IPv4Address(host)) != host:
                        raise ValueError
                except ValueError:
                    raise ValueError("Enter a valid IPv4 address.") from None
            # Reject invalid ASCII-encoded internationalized domain names.
            try:
                host.lower().encode("ascii").decode("idna")
            except UnicodeError:
                raise ValueError("Enter a valid HTTPS hostname.") from None
            return raw
    raise ValueError("Enter an HTTPS hostname and optional port (1–65535), without a path.")


def prompt(message):
    print(message, file=sys.stderr, end="", flush=True)
    return input()


def private(path, mode):
    try:
        info = path.lstat()
    except FileNotFoundError:
        raise ValueError(RECOVERY) from None
    if info.st_uid != os.getuid() or info.st_mode != mode:
        raise ValueError(RECOVERY)


@contextmanager
def lock(path):
    # Kernel-held locks on existing directory inodes survive neither exit nor SIGKILL.
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError("Another setup is running. Retry after it finishes.") from None
        yield
    finally:
        os.close(fd)


def checkout_id(root):
    info = root.stat()
    return f"{info.st_dev}-{info.st_ino}"


def owned(path, owner):
    private(path, stat.S_IFDIR | 0o700)
    receipt = path / "checkout-id"
    private(receipt, stat.S_IFREG | 0o600)
    if receipt.read_text() != owner:
        raise ValueError(RECOVERY)


def reset_stage(stage, owner):
    # Only the uncommitted files this checkout can create may be discarded.
    owned(stage, owner)
    for entry in stage.iterdir():
        if entry.name in ("admin-email", "admin-password", "compose.env"):
            private(entry, stat.S_IFREG | 0o600)
        elif entry.name in ("postgres", "gateway"):
            private(entry, stat.S_IFDIR | 0o700)
            if any(entry.iterdir()):
                raise ValueError(RECOVERY)
        elif entry.name != "checkout-id":
            raise ValueError(RECOVERY)
    for entry in stage.iterdir():
        if entry.is_dir():
            entry.rmdir()
        elif entry.name != "checkout-id":
            entry.unlink()


def settings(origin, directory):
    return (f"DHIVON_MCP_GATEWAY_PUBLIC_URL='{origin}'\n"
            f"DHIVON_MCP_GATEWAY_SETUP_DIR='{directory}'\n"
            f"DHIVON_MCP_GATEWAY_UID={os.getuid()}\nDHIVON_MCP_GATEWAY_GID={os.getgid()}\n")


def publish_directory(source, destination):
    # os.rename may replace a foreign empty directory. Both supported platforms
    # provide an atomic rename that refuses every existing destination.
    libc = ctypes.CDLL(None, use_errno=True)
    src, dst = os.fsencode(source), os.fsencode(destination)
    if sys.platform == "darwin":
        result = libc.renamex_np(src, dst, 4)  # RENAME_EXCL
    elif sys.platform.startswith("linux") and hasattr(libc, "renameat2"):
        result = libc.renameat2(-100, src, -100, dst, 1)  # AT_FDCWD, RENAME_NOREPLACE
    else:
        raise ValueError("This system lacks exclusive directory renames required for setup.")
    if result:
        err = ctypes.get_errno()
        raise OSError(err, os.strerror(err), destination)


def prepare(stage, directory, owner):
    if os.path.lexists(stage):
        reset_stage(stage, owner)
    origin = origin_value(prompt("Public HTTPS origin (for example https://gateway.example.com): "))
    email = email_value(prompt("First administrator email: "))
    message = "First administrator password (12–128 bytes, hidden): "
    with warnings.catch_warnings():
        warnings.simplefilter("error", getpass.GetPassWarning)
        try:
            password = getpass.getpass(message) if sys.stdin.isatty() else prompt(message)
        except getpass.GetPassWarning:
            raise ValueError("Use a terminal that can hide the administrator password.") from None
    if not 12 <= len(password.encode("utf-8")) <= 128:
        raise ValueError("Password must contain 12–128 bytes.")
    if not stage.exists():
        # Establish ownership before writing any secret. A hard stop during this
        # first rename can leave only an empty directory or a non-secret receipt.
        with tempfile.TemporaryDirectory(prefix=".dhivon-owner-", dir=directory.parent) as initial:
            (Path(initial) / "checkout-id").write_text(owner)
            publish_directory(initial, stage)
    for name in ("postgres", "gateway"):
        (stage / name).mkdir()
    (stage / "admin-email").write_text(email + "\n")
    (stage / "admin-password").write_bytes(password.encode("utf-8"))
    (stage / "compose.env").write_text(settings(origin, directory))
    publish_directory(stage, directory)


def resume(directory, owner):
    # Only recover the gap between the private rename and publishing checkout .env.
    owned(directory, owner)
    if {p.name for p in directory.iterdir()} != {
            "checkout-id", "admin-email", "admin-password", "compose.env", "postgres", "gateway"}:
        raise ValueError(RECOVERY)
    for name in ("postgres", "gateway"):
        private(directory / name, stat.S_IFDIR | 0o700)
        if any((directory / name).iterdir()):
            raise ValueError(RECOVERY)
    for name in ("admin-email", "admin-password", "compose.env"):
        private(directory / name, stat.S_IFREG | 0o600)
    email_value((directory / "admin-email").read_text())
    password = (directory / "admin-password").read_bytes()
    password.decode("utf-8")
    raw = (directory / "compose.env").read_text()
    first = re.fullmatch(r"DHIVON_MCP_GATEWAY_PUBLIC_URL='([^']+)'", raw.partition("\n")[0])
    if not (12 <= len(password) <= 128 and not password.endswith((b"\r", b"\n"))
            and first and raw == settings(origin_value(first[1]), directory)):
        raise ValueError(RECOVERY)
    return raw


def main():
    os.umask(0o077)
    if os.getuid() == 0:
        raise ValueError("Run as the unprivileged deployment owner.")
    root = Path(__file__).resolve().parent.parent
    config = Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config")
    if not config.is_absolute():
        raise ValueError("Use an absolute XDG_CONFIG_HOME outside the checkout.")
    config = config.resolve()
    directory = config / "dhivon-mcp-gateway"
    # samefile also catches case and Unicode aliases of the checkout on macOS.
    inside = any(p.exists() and p.samefile(root) for p in (directory, *directory.parents))
    if inside or any(c in str(directory) for c in "'\\\n\r"):
        raise ValueError("Choose a private config location outside this checkout with a plain path.")
    target = root / ".env"
    with lock(root):
        if os.path.lexists(target):
            raise ValueError(EXISTING)
        config.mkdir(parents=True, exist_ok=True)
        info = config.stat()
        if info.st_uid != os.getuid() or info.st_mode & 0o022:
            raise ValueError("Use a config directory owned by you and writable only by you.")
        with lock(config):
            owner = checkout_id(root)
            if not os.path.lexists(directory):
                prepare(config / (".dhivon-setup-" + owner), directory, owner)
            raw = resume(directory, owner)
            # The checkout and config can be on separate filesystems.
            with tempfile.NamedTemporaryFile(mode="w", dir=root, prefix=".env.configure.") as temp:
                temp.write(raw)
                temp.flush()
                os.link(temp.name, target)
    print(f"Private deployment directory: {directory}\nRun from the repository root: docker compose up -d --build --wait")


if __name__ == "__main__":
    try:
        main()
    except (EOFError, KeyboardInterrupt):
        sys.exit("Setup interrupted. Rerun the configuration command.")
    except UnicodeError:
        sys.exit("Setup inputs must be valid UTF-8. Check the private inputs before retrying.")
    except (ValueError, OSError) as error:
        sys.exit(str(error))
