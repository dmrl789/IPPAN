#[cfg(test)]
mod tests {
    use super::super::transaction::Transaction;
    use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
    use rand::rngs::OsRng;

    #[test]
    fn test_transaction_signature() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let message = format!("{}{}{}", "alice", "bob", 100);
        let signature = signing_key.sign(message.as_bytes());

        let tx = Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            signature: signature.to_bytes().to_vec(),
        };

        assert!(tx.verify(&verifying_key));
    }
}
