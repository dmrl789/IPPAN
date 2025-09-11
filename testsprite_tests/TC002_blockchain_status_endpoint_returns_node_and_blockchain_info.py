import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30
HEADERS = {
    "Accept": "application/json"
}

def test_blockchain_status_endpoint_returns_node_and_blockchain_info():
    url = f"{BASE_URL}/status"
    try:
        response = requests.get(url, headers=HEADERS, timeout=TIMEOUT)
        response.raise_for_status()
    except requests.exceptions.RequestException as e:
        assert False, f"Request to {url} failed: {e}"

    assert response.status_code == 200, f"Expected status code 200 but got {response.status_code}"
    try:
        data = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    # Validate presence and types of required fields
    required_fields = {
        "node_id": str,
        "status": str,
        "current_block": int,
        "total_transactions": int,
        "network_peers": int,
        "uptime_seconds": int,
        "version": str
    }

    for field, expected_type in required_fields.items():
        assert field in data, f"Missing field '{field}' in response JSON"
        assert isinstance(data[field], expected_type), f"Field '{field}' should be of type {expected_type.__name__} but got {type(data[field]).__name__}"

# Run the test
test_blockchain_status_endpoint_returns_node_and_blockchain_info()