import requests
import io

BASE_URL = "http://localhost:3000"
TIMEOUT = 30  # seconds


def test_store_file_endpoint_accepts_and_stores_file_successfully():
    url = f"{BASE_URL}/storage/files"

    # Simulate a small test file content
    file_content = b"Test content for IPPAN encrypted sharded storage."
    file_name = "testfile.txt"

    files = {
        "file": (file_name, io.BytesIO(file_content), "text/plain")
    }

    try:
        response = requests.post(url, files=files, timeout=TIMEOUT)
    except requests.RequestException as e:
        assert False, f"Request to POST /storage/files failed: {e}"

    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"

    try:
        json_resp = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    # Validate presence and type of expected fields: file_id, hash, size
    assert "file_id" in json_resp, "Missing 'file_id' in response"
    assert isinstance(json_resp["file_id"], str), "'file_id' should be a string"

    assert "hash" in json_resp, "Missing 'hash' in response"
    assert isinstance(json_resp["hash"], str), "'hash' should be a string"

    assert "size" in json_resp, "Missing 'size' in response"
    assert isinstance(json_resp["size"], int), "'size' should be an integer"
    assert json_resp["size"] == len(file_content), f"'size' should be {len(file_content)}"

    # No resource cleanup needed as storage system presumably manages files independently


test_store_file_endpoint_accepts_and_stores_file_successfully()