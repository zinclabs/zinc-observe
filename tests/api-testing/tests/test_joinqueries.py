import pytest
from datetime import datetime, timezone, timedelta

# Define test data with different queries and expected response details
test_data = [
    (
        "LIKE query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a INNER JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name WHERE a.kubernetes_pod_name LIKE '%ziox%' LIMIT 1",
        1,
    ),
    (
        "Basic join query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a INNER JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name LIMIT 10",
        10,
    ),
    (
        "IN query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a INNER JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name WHERE a.kubernetes_pod_name IN ('prometheus-k8s-1') LIMIT 1",
        1,
    ),
    (
        "Left join query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a LEFT JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name",
        100,
    ),
    (
        "Right join query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a RIGHT JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name",
        100,
    ),
    (
        "Full join query",
        "SELECT a.kubernetes_namespace_name, b.kubernetes_pod_name FROM stream_pytest_data AS a FULL JOIN stream_pytest_data AS b ON a.kubernetes_pod_name = b.kubernetes_pod_name",
        100,
    ),
]

@pytest.mark.parametrize("test_name, sql_query, expected_size", test_data)
def test_e2e_join_queries(create_session, base_url, test_name, sql_query, expected_size):
    """Parameterized E2E test for various join SQL queries."""
    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    
    # Define time range for the query
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    
    # Query data
    json_data = {
        "query": {
            "sql": sql_query,
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": expected_size,
            "quick_mode": False,
            "sql_mode": "full"
        }
    }
    
    # Send POST request with the session
    response = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    
    # Assert that the response status is 200
    assert response.status_code == 200, f"{test_name} failed: Expected status 200, got {response.status_code} - {response.content}"
