import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_list_stored_files_endpoint_returns_file_list():
    url = f"{BASE_URL}/storage/files"
    headers = {
        "Accept": "application/json"
    }
    try:
        response = requests.get(url, headers=headers, timeout=TIMEOUT)
        response.raise_for_status()
    except requests.RequestException as e:
        assert False, f"Request to {url} failed: {e}"

    # Assert status code is 200
    assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"

    # Assert response is JSON array
    try:
        files = response.json()
    except ValueError:
        assert False, "Response is not valid JSON"

    assert isinstance(files, list), f"Expected JSON array (list), got {type(files)}"

    # Each file item contains required fields with proper types
    for file in files:
        assert isinstance(file, dict), f"Each file item should be a dict, got {type(file)}"
        # file_id: could be string or int depending on implementation, test presence only
        assert "file_id" in file, "file_id missing in a file entry"
        assert "name" in file, "name missing in a file entry"
        assert "size" in file, "size missing in a file entry"
        assert "hash" in file, "hash missing in a file entry"

        # Optionally check types
        # file_id: string or int
        assert isinstance(file["file_id"], (int, str)), f"file_id should be int or string, got {type(file['file_id'])}"
        assert isinstance(file["name"], str), f"name should be string, got {type(file['name'])}"
        assert isinstance(file["size"], int), f"size should be int, got {type(file['size'])}"
        assert isinstance(file["hash"], str), f"hash should be string, got {type(file['hash'])}"

test_list_stored_files_endpoint_returns_file_list()