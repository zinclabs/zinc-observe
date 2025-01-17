import json
import time


def valid_trace():
    start_time = int(time.time()*1000000000)
    end_time = start_time+6*100000
    json_str =  """{"resourceSpans": [
    {"resource": {
        "attributes": [{
            "key": "service.name",
            "value": {"stringValue": "eternaltwin"}
          },
          {"key": "telemetry.sdk.language",
            "value": {"stringValue": "rust"}
          }]
      },
      "scopeSpans": [
        {"scope": {
            "name": "eternaltwin_rest",
            "version": "",
            "attributes": [],
            "droppedAttributesCount": 0
          },
          "spans": [{
              "traceId": "99fbb827d84246cfb5113f2fbd8bcede",
              "spanId": "51ce5088d24241d8",
              "traceState": "",
              "parentSpanId": "",
              "flags": 1,
              "name": "POST /oauth/token",
              "kind": 2,
              "startTimeUnixNano": <start_time>,
              "endTimeUnixNano": <end_time>,
              "attributes": [{
                  "key": "http.route",
                  "value": {"stringValue": "/oauth/token"}
                },
                {"key": "http.request.method",
                  "value": {"stringValue": "POST"}
                },
                {"key": "url.path",
                  "value": {"stringValue": "/oauth/token"}
                },
                {"key": "http.response.status_code",
                  "value": {"intValue": 200}
                }
              ],
              "droppedAttributesCount": 0,
              "events": [],
              "droppedEventsCount": 0,
              "links": [],
              "droppedLinksCount": 0,
              "status": {"message": "","code": 1}
            }]},
        {
          "scope": {
            "name": "eternaltwin_services::auth",
            "version": "",
            "attributes": [],
            "droppedAttributesCount": 0
          },
          "spans": [{
              "traceId": "99fbb827d84246cfb5113f2fbd8bcede",
              "spanId": "436436e4d9334747",
              "traceState": "",
              "parentSpanId": "68fa70dc0532497d",
              "flags": 1,
              "name": "AuthService::grant_oauth_authorization::claims",
              "kind": 1,
              "startTimeUnixNano": <start_time>,
              "endTimeUnixNano": <end_time>,
              "attributes": [],
              "droppedAttributesCount": 0,
              "events": [],
              "droppedEventsCount": 0,
              "links": [],
              "droppedLinksCount": 0,
              "status": {
                "message": "",
                "code": 0
              }}]}
    ]}]}"""

    json_str = json_str.replace("<start_time>","\"{:d}\"".format(start_time))
    json_str = json_str.replace("<end_time>","\"{:d}\"".format(end_time))
    return json.loads(json_str)


def test_e2e_valid_trace_ingestion(create_session, base_url):
    """Valid trace should get ingested."""

    session = create_session
    url = base_url
    org_id = "default"

    trace = valid_trace()
    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json=trace)

    assert (
        resp_post_trace.status_code == 200
    ), f"Post trace expected 200, but got {resp_post_trace.status_code} {resp_post_trace.content}"

def test_e2e_old_trace_ingestion(create_session, base_url):
    """Traces outside the time range should get be skipped."""

    session = create_session
    url = base_url
    org_id = "default"

    trace = valid_trace()
    trace["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["startTimeUnixNano"] = "1724898237575000000"
    trace["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["endTimeUnixNano"] = "1724898237576000000"
    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json=trace)

    assert (
        resp_post_trace.status_code == 206
    ), f"Invalid trace span time expected 206, but got {resp_post_trace.status_code}"

    content = resp_post_trace.json()
    assert (
        content["partialSuccess"]["rejectedSpans"] == 1
    ), f"Invalid trace span time expected 1 rejected span, but got {content}"

# This is specifically for https://github.com/openobserve/openobserve/issues/4371
def test_e2e_invalid_trace_ingestion(create_session, base_url):
    """Invalid structure JSON trace should return proper error"""
    session = create_session
    url = base_url
    org_id = "default"

    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json={})
    assert (
        resp_post_trace.status_code == 400
    ), f"Invalid trace json expected 400, but got {resp_post_trace.status_code}"

    content = resp_post_trace.json()
    assert (
        "missing field `resourceSpans`" in content["message"]
    ), f"Invalid trace json response expected to contain missing field, but got {content}"

    trace = valid_trace()
    trace["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["startTimeUnixNano"] = 1724898237575000000
    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json=trace)

    assert (
        resp_post_trace.status_code == 400
    ), f"Invalid trace json expected 400, but got {resp_post_trace.status_code}"

    content = resp_post_trace.json()
    assert (
        "Invalid json: invalid type: integer `1724898237575000000`, expected a string" in content["message"]
    ), f"Invalid trace json response expected to contain incorrect field, but got {content}"


def test_e2e_trace_invalid_ids(create_session, base_url):
    """Traces links with invalid ids should get be skipped."""

    session = create_session
    url = base_url
    org_id = "default"
    
    start_time = int(time.time()*1000000)
    start_time = start_time - 10000000
    end_time = start_time + 100000000
    start_time_str = "{:d}".format(start_time)
    end_time_str = "{:d}".format(end_time)
    get_query_url = f"{url}api/{org_id}/default/traces/latest?filter=duration >= 0 AND duration <= 100000000&start_time={start_time_str}&end_time={end_time_str}&from=0&size=25"

    resp_get_trace = session.get(get_query_url)
    original_count = len(resp_get_trace.json()["hits"])

    trace = valid_trace()
    trace["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["links"] = [
        {
        "traceId": [157,48,54,48,139,160,71,67,152,135,40,155], # correct trace id is 16 bytes
        "spanId": [233,128,29,199,119,206,74,251],
        "traceState": "",
        "attributes": [],
        "droppedAttributesCount": 0,
        "flags": 1
    }]
    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json=trace)

    assert (
        resp_post_trace.status_code == 400
    ), f"Invalid span id expected 400, but got {resp_post_trace.status_code}"

    trace = valid_trace()
    trace["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["links"] = [
        {
        "traceId": [157,48,54,48,139,160,71,67,152,135,40,155,0,0,0,0],
        "spanId": [233,128,29,199,119,206], # correct span id is 8 bytes
        "traceState": "",
        "attributes": [],
        "droppedAttributesCount": 0,
        "flags": 1
    }]
    resp_post_trace = session.post(f"{url}api/{org_id}/v1/traces",json=trace)

    assert (
        resp_post_trace.status_code == 400
    ), f"Invalid trace id expected 400, but got {resp_post_trace.status_code}"

    resp_get_trace = session.get(get_query_url)
    new_count = len(resp_get_trace.json()["hits"])

    assert (original_count == new_count), "Expected count of traces to increase by 1."