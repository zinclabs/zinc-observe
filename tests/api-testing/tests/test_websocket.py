import os  # For environment variables
import websocket  # For WebSocket connection
import json # For JSON parsing
import base64 # For encoding user credentials
from datetime import datetime, timezone, timedelta # For timestamp conversion
import time # For sleep
from pathlib import Path # For file paths
import uuid # For generating UUIDs
import pytest # For fixtures
import random
import string
from requests.auth import HTTPBasicAuth

# Constants for WebSocket URL and user credentials
ZO_BASE_URL_SC = os.environ.get("ZO_BASE_URL_SC")  # Use environment variable
WS_ZO_BASE_URL = os.environ.get("WS_ZO_BASE_URL")
ZO_ROOT_USER_EMAIL = os.environ.get("ZO_ROOT_USER_EMAIL")  # Use environment variable
ZO_ROOT_USER_PASSWORD = os.environ.get("ZO_ROOT_USER_PASSWORD")  # Use environment variable
now = datetime.now(timezone.utc)
end_time = int(now.timestamp() * 1000000)
ten_min_ago = int((now - timedelta(minutes=10)).timestamp() * 1000000)

org_id = "default"

root_dir = Path(__file__).parent.parent.parent

def generate_random_string(length=5):
    # Define the characters to choose from lower case letters
    characters = string.ascii_lowercase  # This includes only lowercase letters
    random_string = ''.join(random.choice(characters) for _ in range(length))
    return random_string

# Generate a random string of length 5
random_string = generate_random_string()

stream_name = "tdef" + random_string

stream_join = "join" + random_string

print("Random Stream:", stream_name)

print("String Join:", stream_join)

@pytest.fixture
def base_url_sc():
    """Provide the base URL for the API of Super Cluster."""
    return ZO_BASE_URL_SC

def test_ingest_data(create_session, base_url_sc):
    """Ingest data into the openobserve running instance."""
    session = create_session
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)
    # Open the json data file and read it
    with open(root_dir / "test-data/logs_data.json") as f:
        data = f.read()
    url_Ing = f"{base_url_sc}api/{org_id}/{stream_name}/_json"
    resp_ing = session.post(url_Ing, data=data, headers={"Content-Type": "application/json"})
    print(f"Data ingested in {stream_name} of {base_url_sc} env, status code: ", resp_ing.status_code)
    assert ( resp_ing.status_code == 200), f"Data ingested successfully for websocket test, status code: {resp_ing.status_code}"

    # Join stream data
def test_ingest_join(create_session, base_url_sc):
    """Ingest join data into the openobserve running instance."""
    session = create_session
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)
    # Open the json data file and read it
    with open(root_dir / "test-data/logs_data.json") as f:
        data = f.read()
    url_join = f"{base_url_sc}api/{org_id}/{stream_join}/_json"
    resp_ing_join = session.post(url_join, data=data, headers={"Content-Type": "application/json"})
    print(f"Data ingested for join in {stream_join} of {base_url_sc} env, status code: ", resp_ing_join.status_code)
    assert ( resp_ing_join.status_code == 200), f"Data ingested for join, status code: {resp_ing_join.status_code}"

def test_disable_websocket(create_session, base_url):
    """Fixture to enable WebSocket and return cookies."""
    session = create_session
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)
    url = base_url
    payload_disable_webs = {
        "scrape_interval": 15,
        "span_id_field_name": "span_id",
        "trace_id_field_name": "trace_id",
        "toggle_ingestion_logs": True,
        "enable_websocket_search": False
    }

    print("Session headers:", session.headers)

    resp_disable_webs = session.post(f"{url}api/{org_id}/settings", json=payload_disable_webs)

    print("Disable Websocket", resp_disable_webs.content)

    assert (
        resp_disable_webs.status_code == 200
    ), f"Websocket disable 200, but got {resp_disable_webs.status_code} {resp_disable_webs.content}"


# Define test data with different queries and expected response details for histogram
test_data_histog = [
    (
        "Stream",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        3848,
    ),

    (
        "AND",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE kubernetes_container_name = 'ziox' AND kubernetes_labels_app = 'ziox' GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2002,
    ),

    (
        "OR",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE kubernetes_container_name = 'ziox' OR kubernetes_labels_app = 'ziox' GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2002,
    ),
    (
        "Match_all",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE match_all('ziox') GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        18,
    ),  
    (
        "str_match",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE str_match(kubernetes_container_name, 'ziox') GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2002,
    ), 

    (
        "Like",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE kubernetes_container_name LIKE '%ziox%' GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2002,
    ), 
    
     (
        "IN",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE kubernetes_container_name IN ('controller', 'ziox') GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2816,
    ),  
    
    
    (
        "str_match_ignore_case",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE str_match_ignore_case(kubernetes_container_name, 'ziox') GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        2002,
    ),  

    (
        "Count Having",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        3848,
    ),

    (
        "Not Null",
        f"SELECT histogram(_timestamp, '10 second') AS \"zo_sql_key\", COUNT(*) AS \"zo_sql_num\" FROM \"{stream_name}\" WHERE kubernetes_container_image IS NOT NULL GROUP BY zo_sql_key ORDER BY zo_sql_key ASC",
        1,
        3846,
    ),
  
  
 

 



 
    
  
   

]

@pytest.mark.parametrize("test_name, hist_query, expected_total_hits_results_histg, expected_zo_sql_num_histg", test_data_histog)
def test_histogram(create_session, base_url, test_name, hist_query, expected_total_hits_results_histg, expected_zo_sql_num_histg):
    """Running an E2E test for histogram queries with Parameterized data when websocket is disabled."""

    session = create_session
    url = base_url
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)

    time.sleep(15)  # Increase this time if necessary

    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    ten_min_ago = int((now - timedelta(minutes=10)).timestamp() * 1000000)
    json_data_hist = {
        "query": {
                "sql": hist_query,
                "start_time": ten_min_ago,
                "end_time": end_time,
                "size": -1,
                "sql_mode": "full"
        }
    }

    res_histog = session.post(f"{url}api/{org_id}/_search?type=logs&search_type=UI&use_cache=false", json=json_data_hist)
   
    assert (
        res_histog.status_code == 200
    ), f"histogram mode {test_name} added 200, but got {res_histog.status_code} {res_histog.content}"

    print(f"API {test_name} Response {url} Histog status code:", res_histog.status_code) 

    # Parse the JSON response
    
    res_data_histog = res_histog.json()

    # Validate the total in the response
    total_hits_histog = res_data_histog["total"]

    # Adjust the assertion based on our expectations
    expected_hits_histog = expected_total_hits_results_histg  # we're expecting
    assert total_hits_histog == expected_hits_histog, f"Expected total {test_name} to be {expected_hits_histog}, but got {total_hits_histog}"

    # Validate zo_sql_num hits histogram in the first hit
    if total_hits_histog > 0:
        actual_zo_sql_num_hits_histog = res_data_histog["hits"][0]["zo_sql_num"]
        assert actual_zo_sql_num_hits_histog == expected_zo_sql_num_histg, f"Expected zo_sql_num histogram to be {expected_zo_sql_num_histg}, but got {actual_zo_sql_num_hits_histog}"
    else:
        pytest.fail("No hits found in the response.")

    # Generate request for histogram cache enabled
    res_histog_cache = session.post(f"{url}api/{org_id}/_search?type=logs&search_type=UI&use_cache=true", json=json_data_hist)
   
    assert (
        res_histog_cache.status_code == 200
    ), f"histogram cache {test_name} mode added 200, but got {res_histog_cache.status_code} {res_histog_cache.content}"

    print(f"Response {url} Cache Histog status code:", res_histog_cache.status_code) 

    # Parse the JSON response
    
    res_data_histog_cache = res_histog_cache.json()

    # Validate the total in the response
    total_hits_histog_cache = res_data_histog_cache["total"]

    # Adjust the assertion based on our expectations
    expected_hits_histog_cache = expected_total_hits_results_histg  # what we're expecting
    assert total_hits_histog_cache == expected_hits_histog_cache, f"Expected {test_name} total to be {expected_hits_histog_cache}, but got {total_hits_histog_cache}"

    # Validate zo_sql_num hits histogram in the first hit
    if total_hits_histog_cache > 0:
        actual_zo_sql_num_hits_histog_cache = res_data_histog_cache["hits"][0]["zo_sql_num"]
        assert actual_zo_sql_num_hits_histog_cache == expected_zo_sql_num_histg, f"Expected zo_sql_num histogram to be {expected_zo_sql_num_histg}, but got {actual_zo_sql_num_hits_histog_cache}"
    else:
        pytest.fail("No hits found in the response.")

# Define test data with different queries and expected response details for SQL when websocket is disabled
test_data_sql = [
    (
        "Stream",
        f"SELECT * FROM \"{stream_name}\"",
        100,
        100,
    ),

    (
        "AND",
        f"SELECT * FROM \"{stream_name}\" where kubernetes_container_name = 'ziox' AND kubernetes_labels_app = 'ziox'",
        100,
        100,
    ),

    (

        "OR",
        f"SELECT * FROM \"{stream_name}\" where kubernetes_container_name = 'ziox' OR kubernetes_labels_app = 'ziox'",
        100,
        100,
    ),
    (
        "Match_all",
        f"SELECT * FROM \"{stream_name}\" WHERE match_all('ziox')",
        100,
        18,
    ),

    (
        "Str_match",
        f"SELECT * FROM \"{stream_name}\" where str_match(kubernetes_container_name, 'ziox')",
        100,
        100,
    ),

    (
        "Like",
        f"SELECT * FROM \"{stream_name}\" WHERE kubernetes_container_name LIKE '%ziox%'",
        100,
        100,
    ),

    (
        "AS",
        f"SELECT kubernetes_container_name as \"breakdown_1\" FROM \"{stream_name}\"",
        100,
        100,
    ),

    (
        "IN",
        f"SELECT * FROM \"{stream_name}\" WHERE kubernetes_container_name IN ('controller', 'ziox')",
        100,
        100,
    ),

    (
        "str_match_ignore_case",
        f"SELECT * FROM \"{stream_name}\" where str_match_ignore_case(kubernetes_container_name, 'ziox')",
        100,
        100,
    ),
  
    
    (
        "Limit",
        f"SELECT * FROM \"{stream_name}\" LIMIT 10",
        10,
        10,
    ),
    
    (
        "DISTINCT",
        f"SELECT DISTINCT code FROM \"{stream_name}\"",
        100,
        3,
    ),

    (
        "UNION",
        f"SELECT * FROM \"{stream_name}\" UNION SELECT * FROM \"{stream_join}\"",
        50,
        50,
    ),

    (
        "UNION ALL",
        f"SELECT * FROM \"{stream_name}\" UNION ALL SELECT * FROM \"{stream_join}\"",
        50,
        50,
    ),

    (
        "Join",
        f"SELECT a.kubernetes_namespace_name , b.kubernetes_namespace_name  FROM \"{stream_name}\" as a join \"{stream_join}\" as b on a.kubernetes_namespace_name  = b.kubernetes_namespace_name",
        50,
        50,
    ),

    (
        "LEFT Join",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a LEFT JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id",
        50,
        50,
    ),

    (
        "RIGHT Join",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a RIGHT JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id",
        50,
        50,
    ),

    (
        "FULL Join",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a FULL JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id",
        50,
        50,
    ),

    (
        "Join Where",
        f"SELECT a.kubernetes_namespace_name , b.kubernetes_namespace_name  FROM \"{stream_name}\" as a join \"{stream_join}\" as b on a.kubernetes_docker_id  = b.kubernetes_docker_id WHERE a.kubernetes_container_name = 'ziox' AND b.kubernetes_container_name = 'ziox'",
        50,
        50,
    ),

    (
        "LEFT Join Where",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a LEFT JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id WHERE a.kubernetes_container_name = 'ziox' AND b.kubernetes_container_name = 'ziox'",
        50,
        50,
    ),

    (
        "RIGHT Join Where",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a RIGHT JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id WHERE a.kubernetes_container_name = 'ziox' AND b.kubernetes_container_name = 'ziox'",
        50,
        50,
    ),

    (
        "FULL Join",
        f"SELECT a.kubernetes_docker_id , b.kubernetes_docker_id FROM \"{stream_name}\" as a FULL JOIN \"{stream_join}\" as b ON a.kubernetes_docker_id  = b.kubernetes_docker_id WHERE a.kubernetes_container_name = 'ziox' AND b.kubernetes_container_name = 'ziox'",
        50,
        50,
    ),

    (
        "INNER Join Like",
        f"SELECT \"a\".kubernetes_docker_id, \"b\".kubernetes_docker_id FROM \"{stream_name}\" AS \"a\" INNER JOIN \"{stream_join}\" AS \"b\" ON \"a\".kubernetes_docker_id = \"b\".kubernetes_docker_id WHERE \"a\".kubernetes_container_name LIKE '%ziox%'",
        50,
        50,
    ),

    (
        "INNER Join Like Limit",
        f"SELECT \"a\".kubernetes_docker_id, \"b\".kubernetes_docker_id FROM \"{stream_name}\" AS \"a\" INNER JOIN \"{stream_join}\" AS \"b\" ON \"a\".kubernetes_docker_id = \"b\".kubernetes_docker_id WHERE \"a\".kubernetes_container_name LIKE '%ziox%' LIMIT 10",
        50,
        10,
    ),

    (
        "INNER Join IN",
        f"SELECT \"a\".kubernetes_docker_id, \"b\".kubernetes_docker_id FROM \"{stream_name}\" AS \"a\" INNER JOIN \"{stream_join}\" AS \"b\" ON \"a\".kubernetes_docker_id = \"b\".kubernetes_docker_id WHERE \"a\".kubernetes_container_name IN ('ziox')",
        50,
        50,
    ),

    (
        "INNER Join IN Limit",
        f"SELECT \"a\".kubernetes_docker_id, \"b\".kubernetes_docker_id FROM \"{stream_name}\" AS \"a\" INNER JOIN \"{stream_join}\" AS \"b\" ON \"a\".kubernetes_docker_id = \"b\".kubernetes_docker_id WHERE \"a\".kubernetes_container_name IN ('ziox') LIMIT 10",
        50,
        10,
    ),

    (
        "Count Having",
        f"SELECT COUNT(_timestamp) as totallogcount FROM \"{stream_name}\" Having totallogcount > 1000",
        -1,
        1,
    ),

    (
        "regexp_match",
        f"SELECT _timestamp, array_extract(regexp_match(log, '^[^\\\\]\\n]*\\\\]\\\\s+(?P<httpMethod>\\\\w+)(?:[^/\\n]*/){4}(?P<catalogApi>\\\\w+)(?:[^\\n]* ){2}(?P<httpStatusCode>[^ ]+)\\\\s+(?P<apiPayloadSize>[^ ]+)\\\\s+(?P<responseTime>\\\\d+)'), 3) AS status FROM \"{stream_name}\"",
        100,
        100,
    ),

    (
        "Count Distinct",
        f"SELECT count(distinct(kubernetes_container_name)) FROM \"{stream_name}\"",
        -1,
        1,
    ),

    (
        "MAX",
        f"SELECT MAX(_timestamp), count(_timestamp) FROM \"{stream_name}\"",
        -1,
        1,
    ),

    (
        "Count Aggregate",
        f"SELECT count(*) FROM \"{stream_name}\"",
        -1,
        1,
    ),

    (
        "Not Null",
        f"SELECT * FROM \"{stream_name}\" WHERE kubernetes_container_image IS NOT NULL",
        -1,
        1000,
    ),

    (
        "Avg",
        f"SELECT avg(code) FROM \"{stream_name}\" WHERE code > 200",
        -1,
        1,
    ),






 



 
    
  
   

]

@pytest.mark.parametrize("test_name_sql, sql_query, sql_size, total_exp", test_data_sql)
def test_sql(create_session, base_url, test_name_sql, sql_query, sql_size, total_exp):
    """Running an E2E test for sql queries with Parameterized data when websocket is disabled."""

    session = create_session
    url = base_url
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    ten_min_ago = int((now - timedelta(minutes=10)).timestamp() * 1000000)
    json_data_sql = {
    "query": {
        "sql": sql_query,
        "start_time": ten_min_ago,
        "end_time": end_time,
        "from": 0,
        "size": sql_size,
        "quick_mode": False,
        "sql_mode": "full"
    },
    "regions": [],
    "clusters": []
    }
    
    res_sql = session.post(f"{url}api/{org_id}/_search?type=logs&search_type=UI&use_cache=false", json=json_data_sql)
   
    assert (
        res_sql.status_code == 200
    ), f"SQL mode {test_name_sql} added 200, but got {res_sql.status_code} {res_sql.content}"

    print(f"Response {url} SQL code:", res_sql.status_code) 

    # Parse the JSON response
    
    res_data_sql = res_sql.json()

    # Validate the total in the response
    total_hits_sql = res_data_sql["total"]

    # Adjust the assertion based on our expectations
    expected_hits_sql = total_exp  # what we're expecting
    assert total_hits_sql == expected_hits_sql, f"Expected total {test_name_sql} to be {expected_hits_sql}, but got {total_hits_sql}"

    # Generate request for cache
    res_sql_cache = session.post(f"{url}api/{org_id}/_search?type=logs&search_type=UI&use_cache=true", json=json_data_sql)
   
    assert (
        res_sql_cache.status_code == 200
    ), f"SQL cache {test_name_sql} mode added 200, but got {res_sql_cache.status_code} {res_sql_cache.content}"

    print(f"Response Cache SQL {url} status code :", res_sql_cache.status_code) 

    # Parse the JSON response
    
    res_data_sql_cache = res_sql_cache.json()

    # Validate the total in the response
    total_hits_sql_cache = res_data_sql_cache["total"]

    # Adjust the assertion based on our expectations
    expected_hits_sql_cache = total_exp  # what we're expecting
    assert total_hits_sql_cache == expected_hits_sql_cache, f"Expected {test_name_sql} total to be {expected_hits_sql_cache}, but got {total_hits_sql_cache}"



# Define test data with different queries and expected response details for websocket enable
def test_enable_websocket(create_session, base_url):
    """Fixture to enable WebSocket"""
    session = create_session
    url = base_url
    session.auth = HTTPBasicAuth(ZO_ROOT_USER_EMAIL, ZO_ROOT_USER_PASSWORD)
    payload_websocket = {
        "scrape_interval": 15,
        "span_id_field_name": "span_id",
        "trace_id_field_name": "trace_id",
        "toggle_ingestion_logs": True,
        "enable_websocket_search": True
    }

    print("Session Websocket Enabled headers:", session.headers)

    resp_websocket = session.post(f"{url}api/{org_id}/settings", json=payload_websocket)

    print("Enable Websocket", resp_websocket.content)
    assert (
        resp_websocket.status_code == 200
    ), f"Websocket enable 200, but got {resp_websocket.status_code} {resp_websocket.content}"

@pytest.mark.parametrize("test_name, hist_query, expected_total_hits_results_histg, expected_zo_sql_num_histg", test_data_histog)
def test_websocket_histogram(test_name, hist_query, expected_total_hits_results_histg, expected_zo_sql_num_histg):
    """Test WebSocket connection and histogram endpoint."""
    
    cookie_header_histogram = f"auth_tokens={{\"access_token\":\"Basic {base64.b64encode((ZO_ROOT_USER_EMAIL + ':' + ZO_ROOT_USER_PASSWORD).encode()).decode()}\",\"refresh_token\":\"\"}}"

    # Generate a dynamic UUID
    uuid_histogram = str(uuid.uuid4())  # Generates a new UUID

    # Construct the WebSocket URL
    WS_URL_histogram = f"{WS_ZO_BASE_URL}api/{org_id}/ws/{uuid_histogram}"

    # Example of using the WS_URL
    print("WebSocket Histogram URL:", WS_URL_histogram)

    # Now we can use WS_URL in our WebSocket connection
    try:
        ws_histogram = websocket.create_connection(WS_URL_histogram, header={"Cookie": cookie_header_histogram})
    except websocket.WebSocketBadStatusException as e:
        print(f"Failed to connect: {e}")

        print(f"WebSocket histogram connection {WS_ZO_BASE_URL} established", ws_histogram)

    # Generate a dynamic trace_id
    trace_id_histogram = str(uuid.uuid4())

    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    # Prepare the message to send
    message_histogram = {
        "type": "search",
        "content": {
            "trace_id": trace_id_histogram,  # Use dynamic trace_id
            "payload": {
                "query": {
                    "sql": hist_query,
                    "start_time": ten_min_ago,
                    "end_time": end_time,
                    "size": -1,
                    "sql_mode": "full"
                },
                "regions": [],
                "clusters": []
            },
            "stream_type": "logs",
            "search_type": "ui",
            "use_cache": False
        }
    }

    # Send the message
    ws_histogram.send(json.dumps(message_histogram))

    # Receive the response
    response_histogram = ws_histogram.recv()

    # print("WebSocket response Histogram:", response_histogram) 

    # Parse the JSON response
    response_data_histogram = json.loads(response_histogram)

    # Validate the total in the response
    total_hits_histogram = response_data_histogram["content"]["results"]["total"]

    # Adjust the assertion based on our expectations
    expected_hits_histogram = expected_total_hits_results_histg  #That's what we're expecting
    assert total_hits_histogram == expected_hits_histogram, f"Expected {test_name} total to be {expected_hits_histogram}, but got {total_hits_histogram}"
    
    # Validate zo_sql_num hits histogram in the first hit
    if total_hits_histogram > 0:
        actual_zo_sql_num_hits_histogram = response_data_histogram["content"]["results"]["hits"][0]["zo_sql_num"]
        assert actual_zo_sql_num_hits_histogram == expected_zo_sql_num_histg, f"Expected zo_sql_num histogram to be {expected_zo_sql_num_histg}, but got {actual_zo_sql_num_hits_histogram}"
    else:
        pytest.fail("No hits found in the response.")

    ws_histogram.close()

    # Generate a dynamic UUID for cache when websocket is enabled

    uuid_histogram_cache = str(uuid.uuid4())  # Generates a new UUID

    # Construct the WebSocket URL
    WS_URL_histogram_cache = f"{WS_ZO_BASE_URL}api/{org_id}/ws/{uuid_histogram_cache}"

    # Example of using the WS_URL
    # print("WebSocket Histogram URL Cache:", WS_URL_histogram_cache)

    # Now we can use WS_URL in our WebSocket connection for cache

    ws_histogram_cache = websocket.create_connection(WS_URL_histogram_cache, header={"Cookie": cookie_header_histogram})

    print(f"WebSocket Cache histogram {WS_ZO_BASE_URL} connection established", ws_histogram_cache)

    # Generate a dynamic trace_id
    trace_id_histogram_cache = str(uuid.uuid4())

    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    # Prepare the message to send
    message_histogram_cashe = {
        "type": "search",
        "content": {
            "trace_id": trace_id_histogram_cache,  # Use dynamic trace_id
            "payload": {
                "query": {
                    "sql": hist_query,
                    "start_time": ten_min_ago,
                    "end_time": end_time,
                    "size": -1,
                    "sql_mode": "full"
                },
                "regions": [],
                "clusters": []
            },
            "stream_type": "logs",
            "search_type": "ui",
            "use_cache": True
        }
    }

    # Send the message
    ws_histogram_cache.send(json.dumps(message_histogram_cashe))

    # Receive the response
    response_histogram_cache = ws_histogram_cache.recv()

    # print("WebSocket cache response Histogram:", response_histogram_cache) 

    # Parse the JSON response
    response_data_histogram_cache = json.loads(response_histogram_cache)

    # Validate the total in the response
    total_hits_histogram_cache = response_data_histogram_cache["content"]["results"]["total"]

    # Adjust the assertion based on our expectations
    expected_hits_histogram_cache = expected_total_hits_results_histg  # That's what you're expecting
    assert total_hits_histogram_cache == expected_hits_histogram_cache, f"Expected {test_name} cache total to be {expected_hits_histogram_cache}, but got {total_hits_histogram_cache}"
    
    # Validate zo_sql_num hits histogram in the first hit
    if total_hits_histogram_cache > 0:
        actual_zo_sql_num_hits_histogram_cache = response_data_histogram_cache["content"]["results"]["hits"][0]["zo_sql_num"]
        assert actual_zo_sql_num_hits_histogram_cache == expected_zo_sql_num_histg, f"Expected zo_sql_num histogram to be {expected_zo_sql_num_histg}, but got {actual_zo_sql_num_hits_histogram_cache}"
    else:
        pytest.fail("No hits found in the response.")

    ws_histogram_cache.close()


@pytest.mark.parametrize("test_name_sql, sql_query, sql_size, total_exp", test_data_sql)
def test_websocket_sql(test_name_sql, sql_query, sql_size, total_exp):
    """Test WebSocket with sql when websocket is enabled ."""
    # Prepare headers with cookies

    cookie_header_sql = f"auth_tokens={{\"access_token\":\"Basic {base64.b64encode((ZO_ROOT_USER_EMAIL + ':' + ZO_ROOT_USER_PASSWORD).encode()).decode()}\",\"refresh_token\":\"\"}}"
    
    # Generate a dynamic UUID
    uuid_sql = str(uuid.uuid4())  # Generates a new UUID

    # Construct the WebSocket URL
    WS_URL_sql = f"{WS_ZO_BASE_URL}api/{org_id}/ws/{uuid_sql}"

    # Now we can use WS_URL in our WebSocket connection

    
    try:
        ws_sql = websocket.create_connection(WS_URL_sql, header={"Cookie": cookie_header_sql})
        # Proceed with your WebSocket logic here
    except websocket.WebSocketBadStatusException as e:
        print(f"WebSocket connection failed: {e}")
        # Use the exception's message directly
        status_code = e.args[0].split()[2] if e.args else "Unknown"
        assert False, f"WebSocket connection failed with status: {status_code} and message: {e.args[0]}"
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
        assert False, f"An unexpected error occurred: {e}"

    print(f"WebSocket SQL {WS_ZO_BASE_URL} connection established", ws_sql)

    # Generate a dynamic trace_id
    trace_id_sql = str(uuid.uuid4())

    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    # Prepare the specific message to send
    message_sql = {
        "type": "search",
        "content": {
            "trace_id": trace_id_sql,  # Use dynamic trace_id
            "payload": {
                "query": {
                    "sql": sql_query,
                    "start_time": ten_min_ago,
                    "end_time": end_time,
                    "from": 0,
                    "size": sql_size,
                    "quick_mode": False,
                    "sql_mode": "full"
                },
                "regions": [],
                "clusters": []
            },
            "stream_type": "logs",
            "search_type": "ui",
            "use_cache": False
        }
    }

    # Send the specific message
    ws_sql.send(json.dumps(message_sql))

    # Receive the response
    response_sql = ws_sql.recv()

    # print("WebSocket response for SQL:", response_sql)

    # Parse the JSON response
    response_data_sql = json.loads(response_sql)

    # Validate the total in the response
    total_hits_sql = response_data_sql["content"]["results"]["total"]

    # Adjust the assertion based on our expectations
    expected_hits_sql = total_exp  # That's what we're expecting
    assert total_hits_sql == expected_hits_sql, f"Expected {test_name_sql} total to be {expected_hits_sql}, but got {total_hits_sql}"
    
    ws_sql.close()

    # Generate a dynamic UUID for cache when websocket is enabled

    uuid_sql_cache = str(uuid.uuid4())  # Generates a new UUID

    # Construct the WebSocket URL
    WS_URL_sql_cache = f"{WS_ZO_BASE_URL}api/{org_id}/ws/{uuid_sql_cache}"

    # Example of using the WS_URL
    # print("WebSocket SQL URL Cache:", WS_URL_sql_cache)

    # Now we can use WS_URL in our WebSocket connection for cache

    ws_sql_cache = websocket.create_connection(WS_URL_sql_cache, header={"Cookie": cookie_header_sql})

    print(f"WebSocket Cache SQL {WS_ZO_BASE_URL} connection established", ws_sql_cache)

    # Generate a dynamic trace_id
    trace_id_sql_cache = str(uuid.uuid4())

    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    # Prepare the specific message to send
    message_sql_cache = {
        "type": "search",
        "content": {
            "trace_id": trace_id_sql_cache,  # Use dynamic trace_id
            "payload": {
                "query": {
                    "sql": sql_query,
                    "start_time": ten_min_ago,
                    "end_time": end_time,
                    "from": 0,
                    "size": sql_size,
                    "quick_mode": False,
                    "sql_mode": "full"
                },
                "regions": [],
                "clusters": []
            },
            "stream_type": "logs",
            "search_type": "ui",
            "use_cache": True
        }
    }

    # Send the specific message
    ws_sql_cache.send(json.dumps(message_sql_cache))


    # Receive the response
    response_sql_cache = ws_sql_cache.recv()

    # print("WebSocket cache response for SQL:", response_sql_cache)

    # Parse the JSON response

    response_data_sql_cache = json.loads(response_sql_cache)

    # Validate the total in the response
    total_hits_sql_cache = response_data_sql_cache["content"]["results"]["total"]

    # Adjust the assertion based on our expectations
    expected_hits_sql_cache = total_exp  # That's what we're expecting
    assert total_hits_sql_cache == expected_hits_sql_cache, f"Expected cache {test_name_sql} total to be {expected_hits_sql_cache}, but got {total_hits_sql_cache}"

    ws_sql_cache.close()

def test_delete_stream(create_session, base_url):
    """Running an E2E test for deleting the created stream."""
    session = create_session
    url = base_url
    # Proceed to delete the created Stream
    resp_delete_stream= session.delete(f"{url}api/{org_id}/streams/{stream_name}?type=logs")
    print(f"Deleted Stream Response: {resp_delete_stream.text}")
    assert resp_delete_stream.status_code == 200, f"Failed to delete stream {stream_name}"
    print(f"Successfully deleted stream {stream_name}")

def test_delete_stream_join(create_session, base_url):
    """Running an E2E test for deleting the created join stream."""
    session = create_session
    url = base_url
    # Proceed to delete the created Stream
    resp_delete_stream_join= session.delete(f"{url}api/{org_id}/streams/{stream_join}?type=logs")
    print(f"Deleted Stream Response: {resp_delete_stream_join.text}")
    assert resp_delete_stream_join.status_code == 200, f"Failed to delete stream {stream_join}"
    print(f"Successfully deleted stream {stream_join}")
