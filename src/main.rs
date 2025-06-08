use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

use ippan_core::blockchain::Blockchain;
use ippan_core::p2p::run_p2p_server;
use ippan_core::transaction::Transaction;

#[tokio::main]
async fn main() {
    let blockchain = Arc::new(Mutex::new(Blockchain::new("GENESIS_MINER".to_string())));
    let blockchain_filter = warp::any().map({
        let blockchain = Arc::clone(&blockchain);
        move || Arc::clone(&blockchain)
    });

    // Start P2P in background
    tokio::spawn({
        let blockchain_clone = Arc::clone(&blockchain);
        async move {
            run_p2p_server(blockchain_clone, 9000).await;
        }
    });

    // Submit a new transaction: POST /tx {from, to, amount, signature}
    let submit_tx = warp::path("tx")
        .and(warp::post())
        .and(warp::body::json())
        .and(blockchain_filter.clone())
        .and_then(handle_submit_tx);

    // Get blockchain: GET /chain
    let get_chain = warp::path("chain")
        .and(warp::get())
        .and(blockchain_filter.clone())
        .and_then(handle_get_chain);

    println!("✅ Node HTTP server on http://localhost:8080 ...");
    warp::serve(submit_tx.or(get_chain)).run(([127, 0, 0, 1], 8080)).await;
}

async fn handle_submit_tx(
    tx: Transaction,
    blockchain: Arc<Mutex<Blockchain>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut chain = blockchain.lock().await;
    chain.add_transaction(tx);
    Ok(warp::reply::json(&"Transaction submitted"))
}

async fn handle_get_chain(
    blockchain: Arc<Mutex<Blockchain>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let chain = blockchain.lock().await;
    Ok(warp::reply::json(&chain.chain))
}
