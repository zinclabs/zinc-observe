import pytest
import json
from datetime import datetime, timezone, timedelta

# Load queries from the queries.json file
with open('../test-data/joinqueries.json', 'r') as f:
    queries = json.load(f)

# Define test data for users and orders
users_payload = [
    {"user_id": 1, "name": "Alice", "status": "active", "age": 30},
    {"user_id": 2, "name": "Bob", "status": "inactive", "age": 25},
    {"user_id": 3, "name": "Charlie", "status": "pending", "age": 35},
    {"user_id": 4, "name": "Diana", "status": "active", "age": 28},
    {"user_id": 5, "name": "Eve", "status": "inactive", "age": 22}
]

orders_payload = [
    {"order_id": 101, "user_id": 1, "product": "Laptop", "amount": 1200},
    {"order_id": 102, "user_id": 2, "product": "Smartphone", "amount": 800},
    {"order_id": 103, "user_id": 3, "product": "Tablet", "amount": 500},
    {"order_id": 104, "user_id": 4, "product": "Headphones", "amount": 150}
]

@pytest.mark.parametrize("test_name, sql_query", [(query["test_name"], query["sql_query"]) for query in queries])
def test_e2e_join_queries(create_session, base_url, test_name, sql_query):
    """Parameterized E2E test for various join SQL queries."""
    
    session = create_session
    url = base_url
    org_id = "default"
    stream_name_users = "u1"
    stream_name_orders = "orders"

    # Ingest users data into u1
    resp_create_users_logstream = session.post(
        f"{url}api/{org_id}/{stream_name_users}/_json", json=users_payload
    )
    assert resp_create_users_logstream.status_code == 200, f"Failed to ingest users data: {resp_create_users_logstream.content}"

    # Ingest orders data into orders
    resp_create_orders_logstream = session.post(
        f"{url}api/{org_id}/{stream_name_orders}/_json", json=orders_payload
    )
    assert resp_create_orders_logstream.status_code == 200, f"Failed to ingest orders data: {resp_create_orders_logstream.content}"

    # Define time range for the query
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)

    # Query data for the test
    json_data = {
        "query": {
            "sql": sql_query,
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 100,  # Adjust the size according to your needs
            "quick_mode": False,
            "sql_mode": "full"
        }
    }

    # Send POST request with the session for the SQL query
    response = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)

    # Assert that the response status is 200
    assert response.status_code == 200, f"{test_name} failed: Expected status 200, got {response.status_code} - {response.content}"

    # Assert that the response contains the 'hits' field
    response_json = response.json()
    assert "hits" in response_json, f"{test_name} failed: 'hits' field not found in the response"
    
    # Optionally, assert that 'hits' is not empty
    assert len(response_json["hits"]) > 0, f"{test_name} failed: 'hits' is empty in the response"
