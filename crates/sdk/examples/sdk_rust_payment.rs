use ippan_sdk::{IppanClient, PaymentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api =
        std::env::var("IPPAN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/api/".to_string());
    let from = std::env::var("IPPAN_FROM_ADDRESS")
        .expect("set IPPAN_FROM_ADDRESS (hex string) before running the example");
    let to = std::env::var("IPPAN_TO_ADDRESS").unwrap_or_else(|_| from.clone());
    let signing_key = std::env::var("IPPAN_SIGNING_KEY")
        .expect("set IPPAN_SIGNING_KEY (hex-encoded private key) before running the example");

    let client = IppanClient::new(api)?;

    let account = client.get_account(&from).await?;
    println!(
        "Current balance for {from}: {} (nonce {})",
        account.balance, account.nonce
    );

    let request = PaymentRequest::new(
        from.clone(),
        to.clone(),
        10_000_000_000_000_000u128,
        signing_key,
    )
    .with_nonce(account.nonce + 1)
    .with_memo(Some("sdk example".into()));

    let receipt = client.submit_payment(request).await?;
    println!("Submitted tx {} -> {}", receipt.from, receipt.to);
    println!("Tx hash: {}", receipt.tx_hash);
    println!("Status: {:?}", receipt.status);

    Ok(())
}
