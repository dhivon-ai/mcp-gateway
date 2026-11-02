use dhivon_mcp_gateway_core::crypto::{KeyRing, Sealed};
use std::collections::BTreeMap;
#[test]
fn aead_rejects_tamper_owner_version_and_key_substitution() {
    let keys = KeyRing::new(
        "active".into(),
        BTreeMap::from([("active".into(), [42; 32]), ("alias".into(), [42; 32])]),
    )
    .expect("keys");
    let binding = b"secret:user-a:id-1:version-1";
    let sealed = keys.seal(binding, b"private-token").expect("encrypt");
    assert_eq!(
        keys.open(binding, &sealed).expect("decrypt"),
        b"private-token"
    );
    assert!(!String::from_utf8_lossy(&sealed.ciphertext).contains("private-token"));
    assert_ne!(
        sealed.nonce,
        keys.seal(binding, b"private-token")
            .expect("second encrypt")
            .nonce
    );
    for wrong in [
        b"secret:user-b:id-1:version-1".as_slice(),
        b"secret:user-a:id-1:version-2",
    ] {
        assert!(keys.open(wrong, &sealed).is_err());
    }
    let mut tampered = sealed.clone();
    tampered.ciphertext[0] ^= 1;
    assert!(keys.open(binding, &tampered).is_err());
    let changed = Sealed {
        key_id: "alias".into(),
        ..sealed.clone()
    };
    assert!(keys.open(binding, &changed).is_err());
    let wrong = KeyRing::new(
        "active".into(),
        BTreeMap::from([("active".into(), [1; 32])]),
    )
    .expect("wrong key");
    assert!(wrong.open(binding, &sealed).is_err());
    let missing =
        KeyRing::new("new".into(), BTreeMap::from([("new".into(), [2; 32])])).expect("new key");
    assert!(missing.open(binding, &sealed).is_err());
    let malformed = Sealed {
        format: 2,
        ..sealed.clone()
    };
    assert!(keys.open(binding, &malformed).is_err());
    let file = KeyRing::generate_key_file("generated").expect("secure key file");
    let loaded = KeyRing::from_json(&file).expect("key file");
    let blob = loaded.seal(b"roundtrip", b"value").expect("encrypt");
    assert_eq!(loaded.open(b"roundtrip", &blob).expect("decrypt"), b"value");
}
