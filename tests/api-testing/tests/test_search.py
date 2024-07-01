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
            "sql": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from stream_pytest_data WHERE match_all_raw(\'provide_credentials\') GROUP BY zo_sql_key ORDER BY zo_sql_key",
            "start_time": one_min_ago,
            "end_time": end_time,
            "size": 0,
            "sql_mode":"full"
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
            "sql": 'select * from "stream_pytest_data" WHERE match_all_raw_ignore_case(\'provide_credentials\')',
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
            "sql": 'select * from "stream_pytest_data" WHERE match_all_raw_ignore_case()',
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
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_raw(\'provide_credentials\')',
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
    ), f"histogram mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"
    response_data = resp_get_allsearch.json()
        

    log_messages = [hit['log'] for hit in response_data.get('hits', [])]
        
        # Asserting that at least one log message contains 'provide_credentials'
    assert any('provide_credentials' in log for log in log_messages), "No log message contains 'provide_credentials'"
   
  
    assert (
        resp_get_allsearch.status_code == 200
    ), f"Sql mode added 200, but got {resp_get_allsearch.status_code} {resp_get_allsearch.content}"


def test_e2e_matchallcount(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data1 = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_raw_ignore_case(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "size": -1,
                        "track_total_hits": True,
                },
            }
    json_data2 = {
                "query": {
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all(\'provide_credentials\')',
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "size": -1,
                        "track_total_hits": True,
                },
            }

    resp_get_allsearch1 = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data1)
    resp_get_allsearch2 = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data2)
    assert (
        resp_get_allsearch1.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch1.status_code} {resp_get_allsearch1.content}"
    assert (
        resp_get_allsearch2.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_allsearch2.status_code} {resp_get_allsearch2.content}"
    response_data1 = resp_get_allsearch1.json()
    response_data2 = resp_get_allsearch2.json()

    # Asserting the counts match
    assert response_data1.get('total') == response_data2.get('total')


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
                        "sql": 'SELECT * FROM "stream_pytest_data" WHERE match_all_raw_ignore_case(\'provide_credentials\')',
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



def test_e2e_sqlcountaggregationquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": "SELECT COUNT(_timestamp) AS xyz, _timestamp FROM \"stream_pytest_data\" GROUP BY _timestamp ORDER BY _timestamp DESC",
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 250,
            "quick_mode": False,
            "sql_mode": "full"
        }
        }


    resp_get_countquery = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_countquery.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_countquery.status_code} {resp_get_countquery.content}"
    response_data = resp_get_countquery.json()


def test_e2e_sqlmatchquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": "SELECT _timestamp, array_extract(regexp_match(log, '^[^\\]\n]*\\]\\s+(?P<httpMethod>\\w+)(?:[^/\n]*/){4}(?P<catalogApi>\\w+)(?:[^\n]* ){2}(?P<httpStatusCode>[^ ]+)\\s+(?P<apiPayloadSize>[^ ]+)\\s+(?P<responseTime>\\d+)'), 3) AS status FROM \"stream_pytest_data\" ORDER BY _timestamp DESC",
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 250,
            "quick_mode": False,
            "sql_mode": "full"
        }
        }

    resp_get_matchquery = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_matchquery.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_matchquery.status_code} {resp_get_matchquery.content}"
    response_data = resp_get_matchquery.json()


def test_e2e_sqlmaxquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
  "query": {
    "sql": "SELECT MAX(_timestamp) as latest_timestamp FROM \"stream_pytest_data\" WHERE kubernetes_container_name = 'ziox'",
    "start_time": one_min_ago,
    "end_time": end_time,
    "from": 0,
    "size": 250,
    "quick_mode": False,
    "sql_mode": "full"
  }
}
    resp_get_maxquery = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_maxquery.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_maxquery.status_code} {resp_get_maxquery.content}"
    response_data = resp_get_maxquery.json()
        


def test_e2e_inquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
  "query": {
    "sql": "SELECT * FROM \"e2e_automate\" WHERE kubernetes_container_name IN ('controller', 'ziox') ORDER BY _timestamp DESC",
    "start_time": one_min_ago,
    "end_time": end_time,
    "from": 0,
    "size": 250,
    "quick_mode": False,
    "sql_mode": "full"
  }
}
    resp_get_inquery = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_inquery.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_inquery.status_code} {resp_get_inquery.content}"
    response_data = resp_get_inquery.json()


def test_e2e_distinctquery(create_session, base_url):
    """Running an E2E test for valid sql query."""

    session = create_session
    url = base_url
    org_id = "org_pytest_data"
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    json_data = {
        "query": {
            "sql": "SELECT DISTINCT kubernetes_container_name, _timestamp FROM \"e2e_automate\" ORDER BY _timestamp",
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": 250,
            "quick_mode": False,
            "sql_mode": "full"
        }
}
    resp_get_distinctquery = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    assert (
        resp_get_distinctquery.status_code == 200
    ), f"histogram mode added 200, but got {resp_get_distinctquery.status_code} {resp_get_distinctquery.content}"
    response_data = resp_get_distinctquery.json()


# def test_e2e_cachedscenario(create_session, base_url):
#     """Running an E2E test for valid SQL query."""

#     session = create_session
#     url = base_url
#     org_id = "org_pytest_data"
#     now = datetime.now(timezone.utc)
#     end_time = int(now.timestamp() * 1000000)
#     one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
#     thirty_min_ago = int((now - timedelta(minutes=30)).timestamp() * 1000000)

#     # First query data
#     json_data1 = {
#         "query": {
#             "sql": "select * from \"stream_pytest_data\" ",
#             "start_time": one_min_ago,
#             "end_time": end_time,
#             "size": 0,
#             "sql_mode": "full",
#             "track_total_hits": True
#         }
# }

#     # Second query data
#     json_data2 = {
#         "query": {
#             "sql": "select * from \"stream_pytest_data\" ",
#             "start_time": thirty_min_ago,
#             "end_time": end_time,
#             "size": 0,
#             "sql_mode": "full",
#             "track_total_hits": True
#         }
# }

#     resp_get_allsearch1 = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data1)
    
#     # Wait for 5 seconds
#     time.sleep(5)  
    
#     response_data1 = resp_get_allsearch1.json()
#     cached_ratio1 = response_data1.get('cached_ratio', 0)
#     assert cached_ratio1 == 0, f"Expected cached_ratio to be 0, but got {cached_ratio1}"

#     # Wait for 5 minutes
#     time.sleep(40)  # Sleep for 300 seconds (5 minutes)
    
#     resp_get_allsearch2 = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data2)
    
#     assert resp_get_allsearch1.status_code == 200, f"histogram mode added 200, but got {resp_get_allsearch1.status_code} {resp_get_allsearch1.content}"
#     assert resp_get_allsearch2.status_code == 200, f"histogram mode added 200, but got {resp_get_allsearch2.status_code} {resp_get_allsearch2.content}"

#     response_data2 = resp_get_allsearch2.json()
#     cached_ratio2 = response_data2.get('cached_ratio', 0)
#     assert cached_ratio2 > 0, f"Expected cached_ratio to be greater than 0, but got {cached_ratio2}"