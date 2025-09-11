import requests
import uuid
import io

BASE_URL = "http://localhost:3000"
TIMEOUT = 30

def test_delete_file_by_id_endpoint_deletes_file_successfully():
    # Step 1: Create a new file to get a valid file_id for deletion test
    file_content = b"Test file content for deletion."
    file_name = f"testfile_{uuid.uuid4().hex}.txt"
    files = {
        "file": (file_name, io.BytesIO(file_content), "text/plain")
    }

    file_id = None
    try:
        # Upload file
        response_post = requests.post(
            f"{BASE_URL}/storage/files",
            files=files,
            timeout=TIMEOUT
        )
        assert response_post.status_code == 200, f"File upload failed: {response_post.text}"
        data_post = response_post.json()
        assert "file_id" in data_post, "Response missing file_id"
        file_id = data_post["file_id"]
        assert isinstance(file_id, str) and len(file_id) > 0, "Invalid file_id received"

        # Step 2: Delete the uploaded file by file_id
        response_delete = requests.delete(
            f"{BASE_URL}/storage/files/{file_id}",
            timeout=TIMEOUT
        )
        assert response_delete.status_code == 200, f"File deletion failed: {response_delete.text}"

        # Step 3: Verify file is deleted by attempting to GET it (expecting 404 or similar)
        response_get = requests.get(
            f"{BASE_URL}/storage/files/{file_id}",
            timeout=TIMEOUT
        )
        # Expecting not found or error since file was deleted
        assert response_get.status_code != 200, "Deleted file still accessible"

    finally:
        # Cleanup: If file still exists, try to delete forcibly (just cleanup)
        if file_id:
            try:
                requests.delete(f"{BASE_URL}/storage/files/{file_id}", timeout=TIMEOUT)
            except Exception:
                pass

test_delete_file_by_id_endpoint_deletes_file_successfully()