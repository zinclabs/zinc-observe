def retrieve_new_dashboard_folders(session, base_url, org_id, folder_id):
    resp_get_dashboard_folders = session.get(
        f"{base_url}api/v2/{org_id}/folders/dashboards/{folder_id}")
    print(resp_get_dashboard_folders.content)
    assert (
        resp_get_dashboard_folders.status_code == 200
    ), f"Retrieve these dashboards: Expected 200, but got {resp_get_dashboard_folders.status_code} {resp_get_dashboard_folders.content}"

def delete_new_dashboard_folders(session, base_url, org_id, folder_id):
    resp_delete_dashboard_folders = session.delete(
        f"{base_url}api/v2/{org_id}/folders/dashboards/{folder_id}")
    assert (
        resp_delete_dashboard_folders.status_code == 200
    ), f"Deleting these dashboards: Expected 200, but got {resp_delete_dashboard_folders.status_code} {resp_delete_dashboard_folders.content}"

def test_new_dashboards_folders(create_session, base_url):
    """Running an E2E test for all new dashboards."""

    session = create_session
    org_id = "default"

    resp_new_dashboards_folders = session.get(f"{base_url}api/v2/{org_id}/folders/dashboards")

    print(resp_new_dashboards_folders.content)
    assert (
        resp_new_dashboards_folders.status_code == 200
    ), f"Retrieve all functions list 200, but got {resp_new_dashboards_folders.status_code} {resp_new_dashboards_folders.content}"

def test_new_create_update_delete_dashboards_folders(create_session, base_url):
    """Running an E2E test for create and delete dashboards folder."""

    session = create_session
    # Create a dashboards folder
    org_id = "default"
    url = base_url
    payload = {"description": "newfoldernvp", "name": "pytest-automationtests"}
    resp_create_new_dashboards_folders = session.post(
        f"{url}api/v2/{org_id}/folders/dashboards", json=payload)

    print(resp_create_new_dashboards_folders.content)

    folder_id = resp_create_new_dashboards_folders.json()["folderId"]

    assert (
        resp_create_new_dashboards_folders.status_code == 200
    ), f"Expected 200 dashboards, but got {resp_create_new_dashboards_folders.status_code} {resp_create_new_dashboards_folders.content}"
    
    payload = {
        "description": "folderupdated",
        "folderId": folder_id,
        "name": "pytest-automation",
    }
    folder_id = resp_create_new_dashboards_folders.json()["folderId"]
    # Update folder
    resp_update_new_dashboards_folders = session.put(
        f"{base_url}api/v2/{org_id}/folders/dashboards/{folder_id}", json=payload
    )
    assert (
        resp_update_new_dashboards_folders.status_code == 200
    ), f"updating this dashboard folder, but got {resp_update_new_dashboards_folders.status_code} {resp_update_new_dashboards_folders.content}"
    print(resp_update_new_dashboards_folders.content)
    # Delete Dashboard Folder
    resp_delete_new_dashboards_folders = session.delete(
        f"{base_url}api/v2/{org_id}/folders/dashboards/{folder_id}")
    assert (
        resp_delete_new_dashboards_folders.status_code == 200
    ), f"Deleting this Dashboards folder, but got {resp_delete_new_dashboards_folders.status_code} {resp_delete_new_dashboards_folders.content}"

def retrieve_new_alerts_folder(session, base_url, org_id, folder_id):
    resp_get_alert_folder = session.get(
        f"{base_url}api/v2/{org_id}/folders/alerts/{folder_id}")
    print(resp_get_alert_folder.content)
    assert (
        resp_get_alert_folder.status_code == 200
    ), f"Retrieve this folder alert: Expected 200, but got {resp_get_alert_folder.status_code} {resp_get_alert_folder.content}"


def test_new_alerts_folders(create_session, base_url):
    """Running an E2E test for all new alerts."""

    session = create_session
    org_id = "default"

    resp_new_all_alerts_folders = session.get(f"{base_url}api/v2/{org_id}/folders/alerts")

    print(resp_new_all_alerts_folders.content)
    assert (
        resp_new_all_alerts_folders.status_code == 200
    ), f"Retrieve all functions list 200, but got {resp_new_all_alerts_folders.status_code} {resp_new_all_alerts_folders.content}"

def test_new_create_alerts_update_delete_folders(create_session, base_url):
    """Running an E2E test for create, update and delete alerts folder."""

    session = create_session
    # Create a alerts folder
    org_id = "default"
    url = base_url
    payload = {"description": "newfoldernvp", "name": "pytest-automationtests"}
    resp_create_new_alerts_folders = session.post(
        f"{url}api/v2/{org_id}/folders/alerts", json=payload)

    print(resp_create_new_alerts_folders.content)

    folder_id = resp_create_new_alerts_folders.json()["folderId"]

    assert (
        resp_create_new_alerts_folders.status_code == 200
    ), f"Expected 200 alerts, but got {resp_create_new_alerts_folders.status_code} {resp_create_new_alerts_folders.content}"
    
    payload = {
        "description": "folderupdated",
        "folderId": folder_id,
        "name": "pytest-automation",
    }
    folder_id = resp_create_new_alerts_folders.json()["folderId"]
    # Update Alerts folder
    resp_update_new_alerts_folders = session.put(
        f"{base_url}api/v2/{org_id}/folders/alerts/{folder_id}", json=payload
    )
    assert (
        resp_update_new_alerts_folders.status_code == 200
    ), f"updating this alerts folder, but got {resp_update_new_alerts_folders.status_code} {resp_update_new_alerts_folders.content}"
    print(resp_update_new_alerts_folders.content)
    # Delete Alerts Folder
    resp_delete_new_alerts_folders = session.delete(
        f"{base_url}api/v2/{org_id}/folders/alerts/{folder_id}")
    assert (
        resp_delete_new_alerts_folders.status_code == 200
    ), f"Deleting this Alerts folder, but got {resp_delete_new_alerts_folders.status_code} {resp_delete_new_alerts_folders.content}"

def delete_new_alerts_folders(session, base_url, org_id, folder_id):
    resp_delete_alerts_folders = session.delete(
        f"{base_url}api/v2/{org_id}/folders/alerts/{folder_id}")
    assert (
        resp_delete_alerts_folders.status_code == 200
    ), f"Deleting this folder alerts: Expected 200, but got {resp_delete_alerts_folders.status_code} {resp_delete_alerts_folders.content}"



