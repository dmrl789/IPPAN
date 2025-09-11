import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_get_current_consensus_round_returns_round_info():
    url = f"{BASE_URL}/consensus/round"
    headers = {
        "Accept": "application/json"
    }
    try:
        response = requests.get(url, headers=headers, timeout=TIMEOUT)
    except requests.RequestException as e:
        assert False, f"Request to {url} failed with exception: {e}"

    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"

    try:
        data = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    # Validate expected top-level fields in response
    # current consensus round number, list of validators, and round status
    assert "round_number" in data or "roundId" in data or "round_id" in data, "Missing consensus round number field"
    assert "validators" in data and isinstance(data["validators"], list), "Missing or invalid validators list"
    assert "round_status" in data or "status" in data, "Missing round status field"

    # Additional checks on the fields
    round_number = data.get("round_number") or data.get("roundId") or data.get("round_id")
    validators = data.get("validators")
    round_status = data.get("round_status") or data.get("status")

    assert isinstance(round_number, int), f"Round number should be int, got {type(round_number)}"
    assert isinstance(validators, list), f"Validators should be a list, got {type(validators)}"
    assert isinstance(round_status, str), f"Round status should be str, got {type(round_status)}"

    # Validate each validator has expected fields: address, stake, status (best effort)
    for v in validators:
        assert isinstance(v, dict), "Each validator entry should be a dict"
        assert "address" in v, "Validator missing 'address' field"
        assert "stake" in v, "Validator missing 'stake' field"
        assert "status" in v, "Validator missing 'status' field"

test_get_current_consensus_round_returns_round_info()