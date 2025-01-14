import tink
import base64
import random
import json
import os
from tink import daead
from tink import secret_key_access
from pathlib import Path

# root directory of the project
root_dir = Path(__file__).parent.parent.parent


# number of total log entries to be ingested
LOG_COUNT = 1000
# keyset json. For simple type key, you still need to give the tink json from which you generated the key
KS = r"""{"primaryKeyId":2939429116,"key":[{"keyData":{"typeUrl":"type.googleapis.com/google.crypto.tink.AesSivKey","value":"EkDqH9D86ii0QPF8EBhcZI1PkBKKdDGMPDS2wFITqqfjQ77RQbDROhhAXI8m5qUcYNbflns8Xo//BORbgtX0msbf","keyMaterialType":"SYMMETRIC"},"status":"ENABLED","keyId":2939429116,"outputPrefixType":"TINK"}]}"""

# openobserve base url
BASE_URL = os.environ["ZO_BASE_URL"]


# values can be tink or simple, depending on the key you are using in OO
KEY_TYPE = "tink"

# fake = Faker()
daead.register()


# normal aead encrypted value
def encrypt_simple(s):
  keyset_handle = tink.json_proto_keyset_format.parse(
  	KS, secret_key_access.TOKEN
  )
  primitive = keyset_handle.primitive(daead.DeterministicAead)
  ciphertext = primitive.encrypt_deterministically(bytes(s,'utf-8'), b'')
  return base64.b64encode(ciphertext[5:]).decode('utf-8')

# uses tink lib to encrypt
def encrypt_tink(s):
  keyset_handle = tink.json_proto_keyset_format.parse(
  	KS, secret_key_access.TOKEN
  )
  primitive = keyset_handle.primitive(daead.DeterministicAead)
  ciphertext = primitive.encrypt_deterministically(bytes(s,'utf-8'), b'')
  return base64.b64encode(ciphertext).decode('utf-8')

def test_cipher_data(create_session, base_url):
    """Ingest data into the openobserve running instance."""
    efn = None
    
    # depending on type of key, change the encryption fn
    if KEY_TYPE == "tink":
        efn = encrypt_tink
    else:
        efn = encrypt_simple

    session = create_session
    # Open the json data file and read it
    with open(root_dir / "test-data/logs_data.json") as f:
        data = f.read()
    
    temp = json.loads(data)
    for t in temp:
        if t["log"] is not None:
            t["log"] = efn(t["log"])

    stream_name = "default"
    org = "default"
    url = f"{BASE_URL}api/{org}/{stream_name}/_json"
    resp = session.post(url, json=temp, headers={"Content-Type": "application/json"})
    print("Data ingested successfully, status code: ", resp.status_code)
    return resp.status_code == 200

def test_get_cipher_keys(create_session, base_url):
    """Running an E2E test for all cipher_keys."""

    session = create_session
    org_id = "default"

    resp_get_cipher_keys = session.get(f"{base_url}api/{org_id}/cipher_keys")

    print(resp_get_cipher_keys.content)
    assert (
        resp_get_cipher_keys.status_code == 200
    ), f"Get all cipher_keys list 200, but got {resp_get_cipher_keys.status_code} {resp_get_cipher_keys.content}"

def test_cipher_key(create_session, base_url):
    """Running an E2E test for create cipher_key."""
    cipher_name = f"cipher_{random.randint(1000, 9999)}"  # Make the name unique
    session = create_session
    org_id = "default"
    payload = {
            "name":cipher_name,
            "key":{"store":{"type":"local","akeyless":{"base_url":"","access_id":"","auth":{"type":"access_key","access_key":"","ldap":{"username":"","password":""}},"store":{"type":"static_secret","static_secret":"","dfc":{"name":"","iv":"","encrypted_data":""}}},"local":"{\"primaryKeyId\":2939429116,\"key\":[{\"keyData\":{\"typeUrl\":\"type.googleapis.com/google.crypto.tink.AesSivKey\",\"value\":\"EkDqH9D86ii0QPF8EBhcZI1PkBKKdDGMPDS2wFITqqfjQ77RQbDROhhAXI8m5qUcYNbflns8Xo//BORbgtX0msbf\",\"keyMaterialType\":\"SYMMETRIC\"},\"status\":\"ENABLED\",\"keyId\":2939429116,\"outputPrefixType\":\"TINK\"}]}"},"mechanism":{"type":"tink_keyset","simple_algorithm":"aes-256-siv"}}}
    
    resp_create_cipher_key = session.post(
            f"{base_url}api/{org_id}/cipher_keys", json=payload
        )
    
    print(resp_create_cipher_key.content)
    
    assert (
            resp_create_cipher_key.status_code == 200
        ), f"Expected 200, but got {resp_create_cipher_key.status_code} {resp_create_cipher_key.content}"

    resp_get_cipher_key = session.get(
            f"{base_url}api/{org_id}/cipher_keys/{cipher_name}"
        )
    
    print(resp_get_cipher_key.content)
    
    assert (
            resp_get_cipher_key.status_code == 200
        ), f"Expected 200, but got {resp_get_cipher_key.status_code} {resp_get_cipher_key.content}"

    payload = {
        "name":cipher_name,
        "key":{"mechanism":{"type":"tink_keyset"},"store":{"type":"local","local":"{\"primaryKeyId\":2939429116,\"key\":[{\"keyData\":{\"typeUrl\":\"type.googleapis.com/google.crypto.tink.AesSivKey\",\"value\":\"EkDqH9D86ii0QPF8EBhcZI1PkBKKdDGMPDS2wFITqqfjQ77RQbDROhhAXI8m5qUcYNbflns8Xo//BORbgtX0msbf\",\"keyMaterialType\":\"SYMMETRIC\"},\"status\":\"ENABLED\",\"keyId\":2939429116,\"outputPrefixType\":\"TINK\"}]}"}}}
    
    resp_update_cipher_key = session.put(
            f"{base_url}api/{org_id}/cipher_keys", json=payload
        )
    
    print(resp_update_cipher_key.content)
    
    assert (
            resp_update_cipher_key.status_code == 200
        ), f"Expected 200, but got {resp_update_cipher_key.status_code} {resp_update_cipher_key.content}"

    resp_delete_cipher_key = session.delete(
            f"{base_url}api/{org_id}/cipher_keys/{cipher_name}"
        )
    
    print(resp_delete_cipher_key.content)
    
    assert (
            resp_delete_cipher_key.status_code == 200
        ), f"Expected 200, but got {resp_delete_cipher_key.status_code} {resp_delete_cipher_key.content}"

   







