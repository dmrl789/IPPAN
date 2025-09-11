import requests

def test_health_check_endpoint_returns_node_status():
    url = "http://localhost:3000/health"
    headers = {
        "Accept": "application/json"
    }
    try:
        response = requests.get(url, headers=headers, timeout=30)
        response.raise_for_status()
    except requests.RequestException as e:
        assert False, f"Request to /health endpoint failed: {e}"

    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"

    try:
        json_data = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    # Expected fields: node status, current timestamp, version string indicating the node is healthy.
    # Based on common naming for these fields (flexible because PRD does not specify exact keys):
    # Let's assert fields presence and types, and possible values.

    required_keys = ["status", "timestamp", "version"]

    for key in required_keys:
        assert key in json_data, f"Key '{key}' not found in response JSON"

    # Validate 'status' field indicates healthy node (for example "healthy" or "ok")
    status_value = json_data["status"]
    assert isinstance(status_value, str), "'status' should be a string"
    assert status_value.lower() in ["healthy", "ok", "running", "up"], f"Unexpected status value: {status_value}"

    # Validate 'timestamp' field is present and is int or str numeric timestamp
    timestamp = json_data["timestamp"]
    assert isinstance(timestamp, (int, str)), "'timestamp' should be int or str"
    if isinstance(timestamp, str):
        assert timestamp.isdigit(), "'timestamp' string should be numeric"

    # Validate 'version' field is a non-empty string
    version = json_data["version"]
    assert isinstance(version, str) and len(version) > 0, "'version' should be a non-empty string"

test_health_check_endpoint_returns_node_status()