use ippan_wallet::cli::run_cli;

#[tokio::main]
async fn main() {
    env_logger::init();
    
    if let Err(e) = run_cli().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}