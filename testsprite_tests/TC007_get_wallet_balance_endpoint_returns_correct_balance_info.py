import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_get_wallet_balance_endpoint_returns_correct_balance_info():
    url = f"{BASE_URL}/wallet/balance"
    headers = {
        "Accept": "application/json"
    }
    try:
        response = requests.get(url, headers=headers, timeout=TIMEOUT)
        # Verify the status code
        assert response.status_code == 200, f"Expected status code 200 but got {response.status_code}"
        json_data = response.json()
        # Check required fields presence and types
        assert isinstance(json_data, dict), "Response is not a JSON object"

        required_fields = ["address", "balance", "staked", "rewards", "pending_transactions"]
        for field in required_fields:
            assert field in json_data, f"Missing field '{field}' in wallet balance response"

        # Validate types of the fields
        assert isinstance(json_data["address"], str) and len(json_data["address"]) > 0, "Invalid or empty wallet address"
        # balance, staked, rewards should be numeric types (int or float)
        for numeric_field in ["balance", "staked", "rewards"]:
            assert isinstance(json_data[numeric_field], (int, float)), f"Field '{numeric_field}' should be a number"

        # pending_transactions should be a list (can be empty)
        assert isinstance(json_data["pending_transactions"], list), "Field 'pending_transactions' should be a list"
    except requests.RequestException as e:
        assert False, f"Request to /wallet/balance endpoint failed: {e}"

test_get_wallet_balance_endpoint_returns_correct_balance_info()