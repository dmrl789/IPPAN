
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

class BlockchainAPIHandler(BaseHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        self.blockchain_data = self.load_blockchain_data()
        super().__init__(*args, **kwargs)
    
    def load_blockchain_data(self):
        try:
            with open("blockchain_data.json", "r") as f:
                return json.load(f)
        except:
            return {"accounts": {}, "transactions": [], "blocks": []}
    
    def do_GET(self):
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        
        if path == "/api/v1/balance":
            # Extract address from query parameters
            query_params = parse_qs(parsed_path.query)
            address = query_params.get("address", [""])[0]
            
            if address in self.blockchain_data["accounts"]:
                account = self.blockchain_data["accounts"][address]
                response = {
                    "account": address,
                    "balance": account["balance"],
                    "staked": account["staked"],
                    "nonce": account["nonce"]
                }
            else:
                response = {
                    "account": address,
                    "balance": 0,
                    "staked": 0,
                    "nonce": 0
                }
            
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
            
        elif path == "/api/v1/status":
            response = {
                "node": {
                    "is_running": True,
                    "uptime_seconds": int(time.time()) - self.blockchain_data["blockchain_info"]["genesis_timestamp"],
                    "version": "1.0.0",
                    "node_id": "ippan-node-001"
                },
                "network": {
                    "connected_peers": 2,
                    "known_peers": 2,
                    "total_peers": 2
                },
                "mempool": {
                    "total_transactions": len(self.blockchain_data["transactions"]),
                    "total_senders": len(set(tx.get("from", "") for tx in self.blockchain_data["transactions"])),
                    "total_size": sum(len(json.dumps(tx)) for tx in self.blockchain_data["transactions"])
                },
                "consensus": {
                    "current_round": 1,
                    "validator_count": 3,
                    "total_stake": sum(acc["staked"] for acc in self.blockchain_data["accounts"].values())
                }
            }
            
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
            
        elif path == "/health":
            self.send_response(200)
            self.send_header("Content-type", "text/plain")
            self.end_headers()
            self.wfile.write(b"OK")
            
        else:
            self.send_response(404)
            self.end_headers()
            self.wfile.write(b"Not Found")
    
    def do_POST(self):
        if self.path == "/api/v1/transaction":
            content_length = int(self.headers["Content-Length"])
            post_data = self.rfile.read(content_length)
            
            try:
                transaction = json.loads(post_data.decode())
                
                # Process the transaction
                response = self.process_transaction(transaction)
                
                self.send_response(200)
                self.send_header("Content-type", "application/json")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
                
            except Exception as e:
                self.send_response(400)
                self.send_header("Content-type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode())
        else:
            self.send_response(404)
            self.end_headers()
            self.wfile.write(b"Not Found")
    
    def process_transaction(self, transaction):
        # Simple transaction processing
        if "Payment" in transaction.get("tx_type", {}):
            payment = transaction["tx_type"]["Payment"]
            from_addr = payment["from"]
            to_addr = payment["to"]
            amount = payment["amount"]
            fee = payment["fee"]
            
            # Check if sender has sufficient balance
            if from_addr in self.blockchain_data["accounts"]:
                sender = self.blockchain_data["accounts"][from_addr]
                if sender["balance"] >= amount + fee:
                    # Process transaction
                    sender["balance"] -= (amount + fee)
                    sender["nonce"] += 1
                    
                    if to_addr not in self.blockchain_data["accounts"]:
                        self.blockchain_data["accounts"][to_addr] = {
                            "balance": 0,
                            "staked": 0,
                            "nonce": 0,
                            "transactions": []
                        }
                    
                    self.blockchain_data["accounts"][to_addr]["balance"] += amount
                    
                    # Add transaction to history
                    tx_hash = f"tx_{int(time.time())}_{from_addr[:8]}_{to_addr[:8]}"
                    tx_record = {
                        "hash": tx_hash,
                        "from": from_addr,
                        "to": to_addr,
                        "amount": amount,
                        "fee": fee,
                        "timestamp": int(time.time()),
                        "status": "confirmed"
                    }
                    
                    self.blockchain_data["transactions"].append(tx_record)
                    self.blockchain_data["accounts"][from_addr]["transactions"].append(tx_hash)
                    self.blockchain_data["accounts"][to_addr]["transactions"].append(tx_hash)
                    
                    # Save updated data
                    with open("blockchain_data.json", "w") as f:
                        json.dump(self.blockchain_data, f, indent=2)
                    
                    return {"success": True, "message": "Transaction processed successfully", "hash": tx_hash}
                else:
                    return {"success": False, "message": "Insufficient balance"}
            else:
                return {"success": False, "message": "Sender account not found"}
        
        return {"success": False, "message": "Unsupported transaction type"}

if __name__ == "__main__":
    server = HTTPServer(("0.0.0.0", 3001), BlockchainAPIHandler)
    print("Blockchain API server running on port 3001")
    server.serve_forever()
