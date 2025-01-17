def test_e2e_folder(create_session, base_url):
    """Running an E2E test for all xfunctions."""

    session = create_session
    org_id = "default"

    resp_get_allfunctions = session.get(f"{base_url}api/{org_id}/functions")

    print(resp_get_allfunctions.content)
    assert (
        resp_get_allfunctions.status_code == 200
    ), f"Get all functions list 200, but got {resp_get_allfunctions.status_code} {resp_get_allfunctions.content}"


def test_e2e_createdeletefunction(create_session, base_url):
    """Running an E2E test for create and delete functions."""

    session = create_session
    # Create a function
    org_id = "default"
    payload = {
        "function": ".a=190025552",
        "name": "pytestfunction",
        "params": "row",
        "transType": 0,
    }

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 200
    ), f"Expected 200, but got {resp_create_function.status_code} {resp_create_function.content}"
    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunction"
    )
    assert (
        resp_delete_function.status_code == 200
    ), f"Deleting this function, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_invalidfunction(create_session, base_url):
    """Running an E2E test for invalid function creation"""

    session = create_session
    # Create a function
    org_id = "default"
    payload = {"name": "...", "function": "...",
               "params": "row", "transType": 0}

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 400
    ), f"Invalid function creation, but got {resp_create_function.status_code} {resp_create_function.content}"


def test_e2e_updatingfunction(create_session, base_url):
    """Running an E2E test for update and delete an existing function."""

    session = create_session
    # Create a function
    org_id = "default"
    payload = {
        "name": "pytestfunctions",
        "function": ".a=190022",
        "params": "row",
        "transType": 0,
    }

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 200
    ), f"Expected 200, but got {resp_create_function.status_code} {resp_create_function.content}"
    session = create_session
    # Create a function
    org_id = "default"
    payload = {
        "name": "pytestfunctions",
        "function": ".a=1900",
        "params": "row",
        "transType": 0,
    }
    resp_update_function = session.put(
        f"{base_url}api/{org_id}/functions/pytestfunctions",
        json=payload,
    )
    print(resp_create_function.content)
    assert (
        resp_update_function.status_code == 200
    ), f"Updating this function, but got {resp_update_function.status_code} {resp_update_function.content}"
    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunctions"
    )
    assert (
        resp_delete_function.status_code == 200
    ), f"Deleting this function, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_duplicatefunction(create_session, base_url):
    """Running an E2E test for function already exists."""

    session = create_session
    # Create a function
    org_id = "default"
    payload = {
        "name": "pytestfunctions",
        "function": ".a=190022",
        "params": "row",
        "transType": 0,
    }

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 200
    ), f"Expected 200, but got {resp_create_function.status_code} {resp_create_function.content}"
    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunction"
    )
    session = create_session
    # Create a function
    org_id = "default"
    payload = {
        "name": "pytestfunctions",
        "function": ".a=190022",
        "params": "row",
        "transType": 0,
    }

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 400
    ), f"Expected 400 function already exists, but got {resp_create_function.status_code} {resp_create_function.content}"
    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunctions"
    )
    assert (
        resp_delete_function.status_code == 200
    ), f"Deleting this function, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_functionnotfound(create_session, base_url):
    """Running an E2E test for function not found."""

    session = create_session
    # Create a function
    org_id = "default"

    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunctionsss"
    )
    assert (
        resp_delete_function.status_code == 404
    ), f"Function not found, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_addDeleteStreamFunction(create_session, base_url):
    """Running an E2E test for add stream to a function and delete."""

    session = create_session
    org_id = "default"
    stream_name = "test"

    # Create a function
    org_id = "default"
    payload = {
        "function": ".a=190025552",
        "name": "pytestfunction",
        "params": "row",
        "transType": 0,
    }

    resp_create_function = session.post(
        f"{base_url}api/{org_id}/functions", json=payload
    )

    print(resp_create_function.content)
    assert (
        resp_create_function.status_code == 200
    ), f"Expected 200, but got {resp_create_function.status_code} {resp_create_function.content}"

    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/functions/pytestfunction"
    )
    assert (
        resp_delete_function.status_code == 200
    ), f"Deleting this function, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_testinvalidfunction(create_session, base_url):
    """Running an E2E test for invalid test function"""

    session = create_session
    org_id = "default"
    payload = {
        "function": ".test=2jehwkhhe\n.",
        "events": [
            {
                "_timestamp": 1735128523652186,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            },
            {
                "_timestamp": 1735128522644223,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            }
        ]
    }

    resp_test_function = session.post(
        f"{base_url}api/{org_id}/functions/test", json=payload
    )

    print(resp_test_function.content)
    assert (
        resp_test_function.status_code == 400
    ), f"Invalid function creation, but got {resp_test_function.status_code} {resp_test_function.content}"


def test_e2e_onlytextfunction(create_session, base_url):
    """Running an E2E test for invalid test function"""

    session = create_session
    org_id = "default"
    payload = {
        "function": "test",
        "events": [
            {
            "_timestamp": 1735128523652186,
            "job": "test",
            "level": "info",
            "log": "test message for openobserve"
            },
            {
            "_timestamp": 1735128522644223,
            "job": "test",
            "level": "info",
            "log": "test message for openobserve"
            }
        ]
        }

    resp_test_function = session.post(
        f"{base_url}api/{org_id}/functions/test", json=payload
    )

    print(resp_test_function.content)
    assert (
        resp_test_function.status_code == 400
    ), f"Invalid function creation, but got {resp_test_function.status_code} {resp_test_function.content}"



def test_e2e_testonlyspecialcharfunction(create_session, base_url):
    """Running an E2E test for invalid test function"""

    session = create_session
    org_id = "default"
    payload = {
        "function": "=====",
        "events": [
            {
                "_timestamp": 1735128523652186,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            },
            {
                "_timestamp": 1735128522644223,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            }
        ]
    }

    resp_test_function = session.post(
        f"{base_url}api/{org_id}/functions/test", json=payload
    )

    print(resp_test_function.content)
    assert (
        resp_test_function.status_code == 400
    ), f"Invalid function creation, but got {resp_test_function.status_code} {resp_test_function.content}"

def test_e2e_testfunction(create_session, base_url):
    """Running an E2E test for the test function"""

    session = create_session
    # Define organization ID and payload
    org_id = "default"
    payload = {
        "function": ".test=2\n.",
        "events": [
            {
                "_timestamp": 1735128523652186,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            },
            {
                "_timestamp": 1735128522644223,
                "job": "test",
                "level": "info",
                "log": "test message for openobserve"
            }
        ]
    }

    # Send a POST request to the API
    resp_test_function = session.post(
        f"{base_url}api/{org_id}/functions/test", json=payload
    )

    # Print the response content for debugging
    print(resp_test_function.content)

    # Assert that the response status code is 200
    assert resp_test_function.status_code == 200, (
        f"Expected status code 200, but got {resp_test_function.status_code}. "
        f"Response content: {resp_test_function.content}"
    )

    # Parse the response JSON
    response_json = resp_test_function.json()

    # Assert that `test=2` is present in each event in the response
    for result in response_json.get("results", []):
        assert result["event"].get("test") == 2, (
            f"Expected 'test=2' in the response event, but got {result}."
        )
