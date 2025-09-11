import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_get_validators_list():
    url = f"{BASE_URL}/consensus/validators"
    headers = {
        "Accept": "application/json"
    }
    try:
        response = requests.get(url, headers=headers, timeout=TIMEOUT)
        response.raise_for_status()
    except requests.RequestException as e:
        assert False, f"Request to GET /consensus/validators failed: {e}"

    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"
    try:
        validators = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    assert isinstance(validators, list), f"Expected a list of validators, got {type(validators)}"

    for validator in validators:
        assert isinstance(validator, dict), "Validator item should be a dictionary"
        assert "address" in validator, "Validator missing 'address' field"
        assert "stake" in validator, "Validator missing 'stake' field"
        assert "status" in validator, "Validator missing 'status' field"

        # Validate address type and format
        address = validator["address"]
        assert isinstance(address, str) and address.strip(), "Validator 'address' should be a non-empty string"

        # Validate stake type and value
        stake = validator["stake"]
        assert (isinstance(stake, int) or isinstance(stake, float)) and stake >= 0, "Validator 'stake' should be a non-negative number"

        # Validate status field type and value
        status = validator["status"]
        assert isinstance(status, str) and status.strip(), "Validator 'status' should be a non-empty string"

test_get_validators_list()