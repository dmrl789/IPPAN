import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_send_transaction_endpoint_processes_transaction_correctly():
    url = f"{BASE_URL}/wallet/send"
    headers = {
        "Content-Type": "application/json"
    }

    # Sample payload with required fields 'to' and 'amount', and optional 'fee'
    payload = {
        "to": "0xabc123def4567890abcdef1234567890abcdef12",
        "amount": "1000",
        "fee": "10"
    }

    try:
        response = requests.post(url, json=payload, headers=headers, timeout=TIMEOUT)
    except requests.RequestException as e:
        assert False, f"Request to {url} failed with exception: {e}"

    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"

    try:
        data = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    # Validate response contains 'transaction_hash' and 'status'
    assert "transaction_hash" in data, "Response JSON missing 'transaction_hash'"
    assert isinstance(data["transaction_hash"], str) and len(data["transaction_hash"]) > 0, "'transaction_hash' must be a non-empty string"
    assert "status" in data, "Response JSON missing 'status'"
    assert data["status"] == "success" or data["status"] == "pending" or data["status"] == "confirmed", \
        "Status should be one of 'success', 'pending' or 'confirmed'"

test_send_transaction_endpoint_processes_transaction_correctly()