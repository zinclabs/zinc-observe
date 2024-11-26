def delete_folder(session, base_url, org_id, folder_id):
    resp_delete_folder = session.delete(
        f"{base_url}api/{org_id}/folders/{folder_id}")
    assert (
        resp_delete_folder.status_code == 200
    ), f"Deleting this folder: Expected 200, but got {resp_delete_folder.status_code} {resp_delete_folder.content}"


def test_e2e_alldashboards(create_session, base_url):
    """Running an E2E test for all dashboards."""

    session = create_session
    org_id = "default"

    resp_get_alldashboards = session.get(f"{base_url}api/{org_id}/dashboards")

    print(resp_get_alldashboards.content)
    assert (
        resp_get_alldashboards.status_code == 200
    ), f"Get all functions list 200, but got {resp_get_alldashboards.status_code} {resp_get_alldashboards.content}"


def test_e2e_allfolders(create_session, base_url):
    """Running an E2E test for all folders."""

    session = create_session
    org_id = "default"

    resp_get_allfolders = session.get(f"{base_url}api/{org_id}/folders")

    print(resp_get_allfolders.content)
    assert (
        resp_get_allfolders.status_code == 200
    ), f"Get all functions list 200, but got {resp_get_allfolders.status_code} {resp_get_allfolders.content}"


def test_e2e_createdeletefolder(create_session, base_url):
    """Running an E2E test for create and delete folder."""

    session = create_session
    # Create a folder
    org_id = "default"
    url = base_url
    payload = {"description": "newfoldernvp", "name": "pytest-automationtests"}
    resp_create_folder = session.post(
        f"{url}api/{org_id}/folders", json=payload)

    print(resp_create_folder.content)

    folder_id = resp_create_folder.json()["folderId"]

    assert (
        resp_create_folder.status_code == 200
    ), f"Expected 200, but got {resp_create_folder.status_code} {resp_create_folder.content}"
    payload = {
        "description": "folderupdated",
        "folderId": folder_id,
        "name": "pytest-automation",
    }
    folder_id = resp_create_folder.json()["folderId"]
    # Update folder
    resp_update_folder = session.put(
        f"{base_url}api/{org_id}/folders/{folder_id}", json=payload
    )
    assert (
        resp_update_folder.status_code == 200
    ), f"updating this folder, but got {resp_update_folder.status_code} {resp_update_folder.content}"
    print(resp_update_folder.content)
    # Delete Folder
    resp_delete_folder = session.delete(
        f"{base_url}api/{org_id}/folders/{folder_id}")
    assert (
        resp_delete_folder.status_code == 200
    ), f"Deleting this folder, but got {resp_delete_folder.status_code} {resp_delete_folder.content}"


# TODO - Change this testcase when the bug is fixed 1687
# def test_e2e_createfolderwithblankspace(create_session):
#     """Running an E2E test for create and delete folder."""

#     session = create_session
#     # Create a folder
#     org_id = "e2e"
#     payload = {
#         "description": "newfoldernvp",
#         "name": " "
#     }
#     resp_create_folder = session.post(f"https://alpha1.gke.zinclabs.dev/api/{org_id}/folders", json=payload)

#     print(resp_create_folder.content)

#     folder_id = resp_create_folder.json()["folderId"]

#     assert resp_create_folder.status_code == 200, f"Expected 200, but got {resp_create_folder.status_code} {resp_create_folder.content}"
# resp_delete_folder = session.delete(f"https://alpha1.gke.zinclabs.dev/api/{org_id}/folders/{folder_id}")
# assert resp_delete_folder.status_code == 200, f"Deleting this folder, but got {resp_delete_folder.status_code} {resp_delete_folder.content}"


def test_e2e_createdeletedashboard(create_session, base_url):
    """Running an E2E test for create and delete dashboard."""

    session = create_session
    # Create a dashboard
    org_id = "default"
    payload = {"description": "pydashboards", "title": "pytestdashboard"}

    resp_create_dashboard = session.post(
        f"{base_url}api/{org_id}/dashboards", json=payload
    )

    print(resp_create_dashboard.content)

    # dashboard_id = resp_create_dashboard.json()["dashboardId"]

    assert (
        resp_create_dashboard.status_code == 200
    ), f"Expected 200, but got {resp_create_dashboard.status_code} {resp_create_dashboard.content}"
    payload = {
        "title": "testpy",
        "dashboardId": "",
        "description": "updatedashboard",
        "role": "",
        "owner": "root@alpha1.com",
        "created": "2023-10-19T14:08:48.090Z",
        "panels": [],
    }
    dashboard_id = resp_create_dashboard.json()["v1"]["dashboardId"]
    dashboard_hash = resp_create_dashboard.json()["hash"]
    resp_update_dashboard = session.put(
        f"{base_url}api/{org_id}/dashboards/{dashboard_id}?hash={dashboard_hash}", json=payload
    )
    assert (
        resp_update_dashboard.status_code == 200
    ), f"updating this dashboard, but got {resp_update_dashboard.status_code} {resp_update_dashboard.content}"
    resp_delete_dashboard = session.delete(
        f"{base_url}api/{org_id}/dashboards/{dashboard_id}"
    )
    assert (
        resp_delete_dashboard.status_code == 200
    ), f"Deleting this dashboard, but got {resp_delete_dashboard.status_code} {resp_delete_dashboard.content}"


# TODO - Change this testcase when the bug is fixed 1687
# def test_e2e_createdeletedashboard(create_session):
#     """Running an E2E test for create and delete dashboard."""

#     session = create_session
#     # Create a dashboard
#     org_id = "e2e"
#     payload = {
#         "description": "pydashboards",
#         "title": " "
#     }

#     resp_create_dashboard = session.post(f"https://alpha1.gke.zinclabs.dev/api/{org_id}/dashboards", json=payload)

#     print(resp_create_dashboard.content)


#     assert resp_create_dashboard.status_code == 200, f"Expected 200, but got {resp_create_dashboard.status_code} {resp_create_dashboard.content}"
#     dashboard_id = resp_create_dashboard.json()["v1"]["dashboardId"]
# resp_delete_dashboard = session.delete(f"https://alpha1.gke.zinclabs.dev/api/{org_id}/dashboards/{dashboard_id}")
# assert resp_delete_dashboard.status_code == 200, f"Deleting this dashboard, but got {resp_delete_dashboard.status_code} {resp_delete_dashboard.content}"


def test_e2e_movedashboard(create_session, base_url):
    """Running an E2E test for create 2 folders, move dashboards and delete dashboards and folders."""

    session = create_session
    # Create a folder
    org_id = "default"
    url = base_url
    # create folder a

    payload = {"description": "newfoldernvp", "name": "pytest-a"}
    resp_create_folder = session.post(
        f"{url}api/{org_id}/folders", json=payload)

    print(resp_create_folder.content)

    folder_id_a = resp_create_folder.json()["folderId"]
    assert (
        resp_create_folder.status_code == 200
    ), f"unable to create folder 1, but got {resp_create_folder.status_code} {resp_create_folder.content}"

    # create folder b
    payload = {"description": "newfoldernvp", "name": "pytest-b"}
    resp_create_folder = session.post(
        f"{url}api/{org_id}/folders", json=payload)

    print(resp_create_folder.content)

    folder_id_b = resp_create_folder.json()["folderId"]

    assert (
        resp_create_folder.status_code == 200
    ), f"unable to create folder b, but got {resp_create_folder.status_code} {resp_create_folder.content}"

    # create dashboard
    org_id = "default"
    payload = {
        "title": "testpy",
        "dashboardId": "",
        "description": "test",
        "role": "",
        "owner": "root@alpha1.com",
        "created": "2023-10-19T14:08:48.090Z",
        "panels": [],
    }
    resp_create_dashboard = session.post(
        f"{base_url}api/{org_id}/dashboards?folder={folder_id_a}", json=payload
    )
    print(resp_create_dashboard.content)
    try:
        dashboard_id = resp_create_dashboard.json()["v1"]["dashboardId"]
    except KeyError:
        dashboard_id = None
    assert (
        resp_create_dashboard.status_code == 200
    ), f"Created a dashboard 200, but got {resp_create_dashboard.status_code} {resp_create_dashboard.content}"
    try:
        dashboard_id = resp_create_dashboard.json()["v1"]["dashboardId"]
    except KeyError:
        dashboard_id = None
    payload = {"from": folder_id_a, "to": folder_id_b}
    # move dashboard from folder A to folder B
    resp_move_dashboard = session.put(
        f"{base_url}api/{org_id}/folders/dashboards/{dashboard_id}", json=payload
    )
    assert (
        resp_move_dashboard.status_code == 200
    ), f"Moved dashboard 200, but got {resp_move_dashboard.status_code} {resp_move_dashboard.content}"
    try:
        dashboard_id = resp_create_dashboard.json()["v1"]["dashboardId"]
    except KeyError:
        dashboard_id = None
    print(resp_move_dashboard.content)
    # Delete Dashboard from Folder B
    resp_delete_dashboard = session.delete(
        f"{base_url}api/{org_id}/dashboards/{dashboard_id}?folder={folder_id_b}"
    )
    assert (
        resp_delete_dashboard.status_code == 200
    ), f"Deleting this dashboard, but got {resp_delete_dashboard.status_code} {resp_delete_dashboard.content}"
    # Delete folders
    delete_folder(session, base_url, org_id, folder_id_a)
    delete_folder(session, base_url, org_id, folder_id_b)
