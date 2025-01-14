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



