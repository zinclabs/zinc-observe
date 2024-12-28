from datetime import datetime, timezone, timedelta
import time

def get_dynamic_timestamps():
    """Generate dynamic timestamps for the query."""
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    return one_min_ago, end_time


def test_e2e_job(create_session, base_url):
    """Running an E2E test for post the job."""

    session = create_session
    url = base_url
    org_id = "default"

    one_min_ago, end_time = get_dynamic_timestamps()
    headers = {"Content-Type": "application/json"}

    json_data = {
                "query": {
                        "sql": "SELECT * from default",
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150
                },
            }
    resp_post_job = session.post(f"{url}api/{org_id}/search_jobs", headers=headers, json=json_data )

    print(resp_post_job.content)
    assert (
        resp_post_job.status_code == 200
    ), f"Post the job list 200, but got {resp_post_job.status_code} {resp_post_job.content}"

def test_get_job(create_session, base_url):
    """Running an E2E test for Get the job."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_job = session.get(f"{url}api/{org_id}/search_jobs")

    print(resp_get_job.content)
    assert (
        resp_get_job.status_code == 200
    ), f"Get the job list 200, but got {resp_get_job.status_code} {resp_get_job.content}"

def test_get_job_status(create_session, base_url):
    """Running an E2E test for Get the job status."""

    session = create_session
    url = base_url
    org_id = "default"
    resp_get_job = session.get(f"{url}api/{org_id}/search_jobs")
    # Ensure the response is successful and contains data
    assert resp_get_job.status_code == 200, f"Request failed: {resp_get_job.text}"
    response_json = resp_get_job.json()
    
    # Access the first job in the list
    assert isinstance(response_json, list), "Response is not a list"
    assert len(response_json) > 0, "No jobs returned in response"
    
    job_id = response_json[0]["id"]  # Access the first job's 'id'
    assert job_id, "Job ID not found"
    
    print(f"Job ID: {job_id}")

    resp_get_job_status = session.get(f"{url}api/{org_id}/search_jobs/{job_id}/status")

    print(resp_get_job_status.content)
    assert (
        resp_get_job_status.status_code == 200
    ), f"Get the job status 200, but got {resp_get_job_status.status_code} {resp_get_job_status.content}"  

def test_job_cancel(create_session, base_url):
    """Running an E2E test for post the job cancel."""

    session = create_session
    url = base_url
    org_id = "default"

    one_min_ago, end_time = get_dynamic_timestamps()
    headers = {"Content-Type": "application/json"}

    json_data = {
                "query": {
                        "sql": "SELECT * from default",
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150
                },
            }
    resp_post_job = session.post(f"{url}api/{org_id}/search_jobs", headers=headers, json=json_data )

    print(resp_post_job.content)
    assert (
        resp_post_job.status_code == 200
    ), f"Post the job list 200, but got {resp_post_job.status_code} {resp_post_job.content}"

    # Ensure the response is successful and contains the expected data
    assert resp_post_job.status_code == 200, f"Request failed: {resp_post_job.text}"
    response_json = resp_post_job.json()

    # Validate the response structure
    assert "code" in response_json and response_json["code"] == 200, "Unexpected response code"
    assert "message" in response_json, "Response 'message' not found"

    # Extract the job ID from the response message
    message = response_json["message"]
    job_id_prefix = "job_id: "
    assert job_id_prefix in message, "Job ID not found in the message"
    job_id = message.split(job_id_prefix)[-1].strip()
    assert job_id, "Extracted Job ID is empty"

    print(f"Job ID: {job_id}")

    resp_post_job_cancel = session.post(f"{url}api/{org_id}/search_jobs/{job_id}/cancel", headers=headers )

    print(resp_post_job_cancel.content)
    assert (
        resp_post_job_cancel.status_code == 200
    ), f"Post the job cancel 200, but got {resp_post_job_cancel.status_code} {resp_post_job_cancel.content}"  

    # Verify job status after cancellation
    time.sleep(1)  # Brief wait for status update
    resp_status = session.get(f"{url}api/{org_id}/search_jobs/{job_id}/status")
    status_json = resp_status.json()
    assert status_json["status"] == 3, f"Expected job status to be 'canceled' (3), got {status_json['status']}"



def test_job_retry(create_session, base_url):
    """Running an E2E test for post the job retry."""

    session = create_session
    url = base_url
    org_id = "default"
    resp_get_job = session.get(f"{url}api/{org_id}/search_jobs")

   # Ensure the response is successful and contains data
    assert resp_get_job.status_code == 200, f"Request failed: {resp_get_job.text}"
    response_json = resp_get_job.json()
    
     # Access the first job in the list
    assert isinstance(response_json, list), "Response is not a list"
    assert len(response_json) > 0, "No jobs returned in response"
    
    job_id = response_json[0]["id"]  # Access the first job's 'id'
    job_status = response_json[0]["status"]  # Check the job's status
    print(f"Job ID: {job_id}, Status: {job_status}")
    
    # Ensure the job is in a retryable state
    retryable_statuses = [2, 3]  # Finished or canceled
    if job_status not in retryable_statuses:
        raise AssertionError(f"Job is not in a retryable state. Current status: {job_status}")
    
    headers = {"Content-Type": "application/json"}
    resp_post_job_retry = session.post(f"{url}api/{org_id}/search_jobs/{job_id}/retry", headers=headers)
    
    print(resp_post_job_retry.content)
    assert (
        resp_post_job_retry.status_code == 200
    ), f"Post the job retry 200, but got {resp_post_job_retry.status_code} {resp_post_job_retry.content}"
    

def test_get_job_id(create_session, base_url):
    """Running an E2E test for Get the job id."""

    session = create_session
    url = base_url
    org_id = "default"
    resp_get_job = session.get(f"{url}api/{org_id}/search_jobs")

    # Ensure the response is successful and contains data
    assert resp_get_job.status_code == 200, f"Request failed: {resp_get_job.text}"
    response_json = resp_get_job.json()
    
    # Access the first job in the list
    assert isinstance(response_json, list), "Response is not a list"
    assert len(response_json) > 0, "No jobs returned in response"
    
    job_id = response_json[0]["id"]  # Access the first job's 'id'
    assert job_id, "Job ID not found"
    
    print(f"Job ID: {job_id}")

    # Retry mechanism to wait for the job result to be available
    max_retries = 10  # Number of retries
    retry_interval = 5  # Seconds between retries

    for attempt in range(max_retries):
        resp_get_job_id = session.get(f"{url}api/{org_id}/search_jobs/{job_id}/result")
        
        if resp_get_job_id.status_code == 200:
            print("Job result retrieved successfully.")
            print(resp_get_job_id.content)
            return
        
        print(f"Attempt {attempt + 1}: Job result not ready. Retrying in {retry_interval} seconds...")
        time.sleep(retry_interval)

    # If the loop completes without a successful response, raise an assertion error
    raise AssertionError(
        f"Job result not available after {max_retries * retry_interval} seconds. "
        f"Last response: {resp_get_job_id.status_code} {resp_get_job_id.content}"
    )

    resp_get_job_id = session.get(f"{url}api/{org_id}/search_jobs/{job_id}/result")

    print(resp_get_job_id.content)
    assert (
        resp_get_job_id.status_code == 200
    ), f"Get the job id 200, but got {resp_get_job_id.status_code} {resp_get_job_id.content}"  


def test_delete_job(create_session, base_url):
    """Running an E2E test for delete the job."""

    session = create_session
    url = base_url
    org_id = "default"
    
    one_min_ago, end_time = get_dynamic_timestamps()
    headers = {"Content-Type": "application/json"}

    json_data = {
                "query": {
                        "sql": "SELECT * from default",
                        "start_time": one_min_ago,
                        "end_time": end_time,
                        "from": 0,
                        "size": 150
                },
            }
    resp_post_job = session.post(f"{url}api/{org_id}/search_jobs", headers=headers, json=json_data )

    print(resp_post_job.content)
    assert (
        resp_post_job.status_code == 200
    ), f"Post the job list 200, but got {resp_post_job.status_code} {resp_post_job.content}"

    # Ensure the response is successful and contains the expected data
    assert resp_post_job.status_code == 200, f"Request failed: {resp_post_job.text}"
    response_json = resp_post_job.json()

    # Validate the response structure
    assert "code" in response_json and response_json["code"] == 200, "Unexpected response code"
    assert "message" in response_json, "Response 'message' not found"

    # Extract the job ID from the response message
    message = response_json["message"]
    job_id_prefix = "job_id: "
    assert job_id_prefix in message, "Job ID not found in the message"
    job_id = message.split(job_id_prefix)[-1].strip()
    assert job_id, "Extracted Job ID is empty"

    print(f"Job ID: {job_id}")

    resp_delete_job = session.delete(f"{url}api/{org_id}/search_jobs/{job_id}")

    print(resp_delete_job.content)
    assert (
        resp_delete_job.status_code == 200
    ), f"Delete the job 200, but got {resp_delete_job.status_code} {resp_delete_job.content}"

    # Verify job is deleted
    resp_verify = session.get(f"{url}api/{org_id}/search_jobs/{job_id}")
    assert resp_verify.status_code == 404, f"Expected 404 for deleted job, got {resp_verify.status_code}"


