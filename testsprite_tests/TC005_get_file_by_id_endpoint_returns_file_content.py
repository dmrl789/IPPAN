import requests

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_get_file_by_id_returns_file_content():
    # First, create a file to ensure there is a valid file_id to test
    create_url = f"{BASE_URL}/storage/files"
    test_file_name = "test_get_file_by_id.txt"
    test_file_content = b"IPPAN test content for TC005"
    files = {"file": (test_file_name, test_file_content)}

    file_id = None
    try:
        # Upload file
        create_response = requests.post(create_url, files=files, timeout=TIMEOUT)
        assert create_response.status_code == 200, f"File creation failed: {create_response.status_code}, {create_response.text}"

        json_resp = create_response.json()
        assert "file_id" in json_resp, "Response missing 'file_id'"
        file_id = json_resp["file_id"]
        assert isinstance(file_id, str) and len(file_id) > 0, "'file_id' should be a non-empty string"

        # GET the file by id
        get_url = f"{BASE_URL}/storage/files/{file_id}"
        get_response = requests.get(get_url, timeout=TIMEOUT)
        assert get_response.status_code == 200, f"Failed to get file: {get_response.status_code}, {get_response.text}"

        # Validate content is binary and matches uploaded content
        assert get_response.content == test_file_content, "Downloaded file content does not match uploaded content"

    finally:
        # Clean up file after test if created
        if file_id:
            delete_url = f"{BASE_URL}/storage/files/{file_id}"
            try:
                delete_response = requests.delete(delete_url, timeout=TIMEOUT)
                assert delete_response.status_code == 200, f"File deletion failed: {delete_response.status_code}, {delete_response.text}"
            except Exception:
                pass  # Ignore cleanup errors


test_get_file_by_id_returns_file_content()