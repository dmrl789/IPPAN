use ed25519_dalek::{Signer, SigningKey, Verifier};
use ippan_crypto::KeyPair;

const DETERMINISTIC_SEED: [u8; 32] = [42u8; 32];

#[test]
fn ed25519_signature_roundtrip_succeeds() {
    let key_pair = KeyPair::generate();
    let message = b"ippan::crypto::signature::roundtrip";

    let signature = key_pair.sign(message);
    assert!(key_pair.verify(message, &signature).is_ok());
}

#[test]
fn ed25519_signature_rejects_tampered_signature() {
    let key_pair = KeyPair::generate();
    let message = b"ippan::crypto::signature::tamper-check";

    let mut signature = key_pair.sign(message);
    signature[0] ^= 0xFF;

    assert!(key_pair.verify(message, &signature).is_err());
}

#[test]
fn deterministic_keypair_generation_is_reproducible() {
    let signing_key = SigningKey::from_bytes(&DETERMINISTIC_SEED);
    let verifying_key = signing_key.verifying_key();

    let signing_key_again = SigningKey::from_bytes(&DETERMINISTIC_SEED);
    let verifying_key_again = signing_key_again.verifying_key();

    assert_eq!(signing_key.to_bytes(), signing_key_again.to_bytes());
    assert_eq!(verifying_key.to_bytes(), verifying_key_again.to_bytes());

    let message = b"ippan::crypto::deterministic-keypair";
    let signature = signing_key.sign(message);
    let signature_again = signing_key_again.sign(message);
    assert_eq!(signature.to_bytes(), signature_again.to_bytes());
    assert!(verifying_key.verify(message, &signature).is_ok());

    let alternative_seed = [7u8; 32];
    let alternative_key = SigningKey::from_bytes(&alternative_seed);
    assert_ne!(
        verifying_key.to_bytes(),
        alternative_key.verifying_key().to_bytes()
    );
}
