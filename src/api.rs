use std::sync::{Arc, Mutex};
use warp::Filter;
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;

pub async fn run_api_server(
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
) {
    let get_chain = warp::path!("chain").map({
        let blockchain = blockchain.clone();
        move || {
            let chain = blockchain.lock().unwrap();
            warp::reply::json(&chain.chain)
        }
    });

    let get_mempool = warp::path!("mempool").map({
        let mempool = mempool.clone();
        move || {
            let txs = mempool.lock().unwrap().get_transactions().clone();
            warp::reply::json(&txs)
        }
    });

    let routes = get_chain.or(get_mempool);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
