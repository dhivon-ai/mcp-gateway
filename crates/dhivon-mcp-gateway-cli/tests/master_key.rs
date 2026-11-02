//! Verify freshness with two live processes using the default macOS Keychain.
#![cfg(target_os = "macos")]
use dhivon_mcp_gateway_cli::{credentials::CredentialStore, vault::Vault};
use std::{
    fs,
    io::{BufRead, Write},
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const RECORD: &str = "fixture-account-sensitive-label";
const ORIGINAL: &[u8] = b"synthetic-original-api-secret";
const REPLACED: &[u8] = b"synthetic-replacement-secret";
const ROOT: &str = "DHIVON_TEST_MASTER_KEY_ROOT";

fn fixture(root: &Path) -> Vault {
    let name = root
        .file_name()
        .and_then(|v| v.to_str())
        .expect("fixture name");
    let id = name
        .strip_prefix("dhivon-key-proof-")
        .expect("fixture prefix");
    uuid::Uuid::parse_str(id).expect("generated UUID");
    Vault::open(root, format!("com.dhivon.mcp-gateway.fixture.master.{id}")).expect("fixture store")
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fs::read_dir(dir)
        .expect("fixture directory")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.file_name().expect("file name") != "store.lock")
        .map(|p| {
            let b = fs::read(&p).expect("encrypted files");
            (p, b)
        })
        .collect()
}

#[test]
fn committed_key_rejects_full_restore_in_an_existing_reader() {
    let root = std::env::temp_dir()
        .canonicalize()
        .expect("temp")
        .join(format!("dhivon-key-proof-{}", uuid::Uuid::new_v4()));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .expect("private root");
    let vault = fixture(&root);
    vault.initialize().expect("initialize");
    vault.set(RECORD, ORIGINAL).expect("first publish");
    assert!(
        vault
            .get(RECORD)
            .expect("warm reader")
            .is_some_and(|v| v.as_slice() == ORIGINAL)
    );
    let dir = root.join("endpoint-v2");
    let stale = snapshot(&dir);
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "child_updates_key", "--ignored", "--nocapture"])
        .env(ROOT, &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run writer");
    let mut reader = std::io::BufReader::new(child.stdout.take().expect("stdout"));
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).expect("writer readiness") > 0);
        if line.contains("committed-ready") {
            break;
        }
    }
    let current = snapshot(&dir);
    let observed = vault.get(RECORD);
    for (p, _) in &current {
        fs::remove_file(p).expect("move aside current files");
    }
    for (p, b) in &stale {
        fs::write(p, b).expect("restore stale files");
        fs::set_permissions(p, fs::Permissions::from_mode(0o600)).expect("private restored file");
    }
    let replay = vault.get(RECORD);
    for (p, _) in &stale {
        fs::remove_file(p).expect("remove replay");
    }
    for (p, b) in &current {
        fs::write(p, b).expect("return current files");
        fs::set_permissions(p, fs::Permissions::from_mode(0o600)).expect("mode");
    }
    let result = vault.delete(RECORD);
    drop(child.stdin.take());
    let writer = child.wait().expect("writer exit");
    result.expect("remove record");
    vault.remove_empty().expect("remove fixture key");
    fs::remove_dir_all(&root).expect("owned fixture");
    assert!(writer.success());
    assert!(
        observed
            .expect("observe current key")
            .is_some_and(|v| v.as_slice() == REPLACED)
    );
    assert!(replay.is_err(), "reject restored directory");
    for (_, b) in stale.iter().chain(&current) {
        for needle in [ORIGINAL, REPLACED, RECORD.as_bytes()] {
            assert!(
                !b.windows(needle.len()).any(|s| s == needle),
                "private bytes in file"
            );
        }
    }
}

#[test]
#[ignore = "invoked by owning test"]
fn child_updates_key() {
    let root = PathBuf::from(std::env::var(ROOT).expect("fixture"));
    let vault = fixture(&root);
    vault.initialize().expect("resume original index");
    assert!(vault.get(RECORD).expect("original").is_some());
    vault
        .set(RECORD, REPLACED)
        .expect("publish new key and record");
    println!("committed-ready");
    std::io::stdout().flush().expect("flush");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).expect("parent");
    assert!(
        vault
            .get(RECORD)
            .expect("observe other live process")
            .is_none()
    );
}
