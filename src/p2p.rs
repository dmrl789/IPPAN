use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt};
use tokio::sync::Mutex;
use crate::blockchain::Blockchain;

/// Run the P2P server on the given port
pub async fn run_p2p_server(blockchain: Arc<Mutex<Blockchain>>, port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind to address");
    println!("✅ P2P server running on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, _peer_addr)) => {
                let blockchain_clone = Arc::clone(&blockchain);

                let chain_data = {
                    let chain = blockchain_clone.lock().await;
                    bincode::serialize(&chain.chain).unwrap()
                };

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, chain_data).await {
                        eprintln!("❌ Error handling connection: {:?}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Failed to accept connection: {:?}", e);
            }
        }
    }
}

/// Handle sending blockchain data over the socket
async fn handle_connection(mut socket: TcpStream, data: Vec<u8>) -> tokio::io::Result<()> {
    socket.write_all(&data).await?;
    Ok(())
}
