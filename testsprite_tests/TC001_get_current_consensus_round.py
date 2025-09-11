import requests

def test_register_model():
    url = "http://127.0.0.1:8000/models"
    payload = {
        "owner": "test_owner",
        "arch_id": 1,
        "version": 1,
        "weights_hash": "abc123def456",
        "size_bytes": 1024,
        "license_id": 1
    }
    headers = {"Content-Type": "application/json"}

    resp = requests.post(url, json=payload, headers=headers, timeout=10)

    assert resp.status_code == 200, f"Expected status code 200, got {resp.status_code}. Response body: {resp.text}"

    print('Test passed: /models POST returned 200')


if __name__ == "__main__":
    test_register_model()