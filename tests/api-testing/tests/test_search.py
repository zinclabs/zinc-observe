import json
import requests

from datetime import datetime, timezone, timedelta
import time


def test_e2e_getsearch(create_session, base_url):
    """Running an E2E test no payload."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs")

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 400
    ), f"No payload added 400, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"

def test_e2e_query(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE code=200',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_invalidsqlquery(create_session, base_url):
    """Running an E2E test for invalid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE code',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 500
    ), f"Sql mode added 500, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_limitadded(create_session, base_url):
    """Running an E2E test add limit to sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE code=200 limit 5',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added  with limit 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_validhistogram(create_session, base_url):
    """Running an E2E test for valid histogram."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
             "sql": 'select * from "stream_pytest_data" WHERE code=200',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '10 second') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
    }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_histogramwithlimit(create_session, base_url):
    """Running an E2E test for invalid query -history with limit  list."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE code=200 limit 5',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '10 second') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
    }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 500
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallhistogram(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all(\'provide_credentials\')',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallindexhistogram(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all_indexed(\'provide_credentials\')',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallignorecasehistogram(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all_ignore_case(\'provide_credentials\')',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)


def test_e2e_matchallindexedignorecasehistogram(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all_indexed_ignore_case(\'provide_credentials\')',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"

    response_data = resp_get_allsearch.json()

    # In histogram type queries, there should be no hits.
    assert(len(response_data.get("hits")) == 0 )

    # In histogram type queries, there should be agg.histogram type keys.
    assert len(response_data.get("aggs", {}).get("histogram")) != 0
 

def test_e2e_matchallindexedignorecasewithoutsearchfeild(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all_ignore_case()',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 500
    ), f"histogram mode added 500, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallindexedignorecaseinvalidsearchfeild(create_session, base_url):
    """Running an E2E test for valid match all histogram query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": 'select * from "stream_pytest_data" WHERE match_all_indexed_ignore_case('')',
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 0,
            "quick_mode": True,
            "track_total_hits": True
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
} 

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
   

    # print(resp_get_allalerts.content)
    assert (
        resp_get_allsearch.status_code == 500
    ), f"histogram mode added 500, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"

def test_e2e_matchallsql(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
 
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallindexedsql(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_indexed(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        

    log_messages = [hit['log'] for hit in response_data.get('hits', [])]
        
        # Asserting that at least one log message contains 'provide_credentials'
    assert any('provide_credentials' in log for log in log_messages), "No log message contains 'provide_credentials'"
   
  
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallindexedignorecasesql(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_indexed_ignore_case(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        

    log_messages = [hit['log'] for hit in response_data.get('hits', [])]
   
        
        # Asserting that at least one log message contains 'provide_credentials'
    assert any('provide_credentials' in log for log in log_messages), "No log message contains 'provide_credentials'"
   
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallignorecasesql(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_ignore_case(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150,
                        "sql_mode": "full",
                },
            }

    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        

    log_messages = [hit['log'] for hit in response_data.get('hits', [])]
        
        # Asserting that at least one log message contains 'provide_credentials'
    assert any('provide_credentials' in log for log in log_messages), "No log message contains 'provide_credentials'"
    
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"




def test_e2e_sqlaggregationquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
            "query": {
                "sql": "SELECT service_name,_timestamp, COUNT(*) AS log_count FROM \"default\" GROUP BY service_name,_timestamp",
                "start_time": one_min_ago,
                "end_time": end_time,
                "from": 0,
                "size": 250,
                "quick_mode": True,
                "sql_mode": "full"
            },
}
    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
    # result = execute_sql_query(**query_params)

    # # Perform assertions on the result
    # assert isinstance(result, list)
    # assert len(result) > 0

    # for entry in result:
    #     assert "_timestamp" in entry
    #     assert "log_count" in entry
    #     # Optionally, assert that "service_name" is present if available
    #     if "service_name" in entry:
    #         assert isinstance(entry["service_name"], str)
        

def test_e2e_sqlgroupbytimestamp(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
            "query": {
                "sql": "SELECT count(*), _timestamp FROM \"default\" group by _timestamp",
                "start_time": one_min_ago,
                "end_time": end_time,
                "from": 0,
                "size": 250,
                "quick_mode": True,
                "sql_mode": "full"
            },
}
    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        
   
def test_e2e_sqlgroupbytimestampwithk8s(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
            "query": {
                "sql": "SELECT count(*), k8s_node_name,_timestamp FROM \"default\" group by k8s_node_name,_timestamp",
                "start_time": one_min_ago,
                "end_time": end_time,
                "from": 0,
                "size": 250,
                "quick_mode": True,
                "sql_mode": "full"
            },
}
    resp_get_allsearch = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_allsearch.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        
   


