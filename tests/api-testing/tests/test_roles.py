import random  # Add this line
# Other imports...

from http import HTTPStatus  # Add this import
import pytest


# Other necessary imports...


def test_get_roles(create_session, base_url):
    """Running an E2E test for get all the roles list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_roles = session.get(f"{url}api/{org_id}/roles")

    print(resp_get_roles.content)
    assert (
        resp_get_roles.status_code == 200
    ), f"Get all service accounts list 200, but got {resp_get_roles.status_code} {resp_get_roles.content}"



def test_e2e_createdeleteroles(create_session, base_url):
    """Running an E2E test for create and delete service account."""

    role_name = f"role_{random.randint(1000, 9999)}"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
    "name": role_name
           }

    resp_create_role = session.post(
        f"{base_url}api/{org_id}/roles", json=payload
    )

    print(resp_create_role.content)
    assert (
        resp_create_role.status_code == 200
    ), f"Expected 200, but got {resp_create_role.status_code} {resp_create_role.content}"
    resp_delete_role = session.delete(
    f"{base_url}api/{org_id}/roles/{role_name}"
    )

    assert (
        resp_delete_role.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_role.status_code} {resp_delete_role.content}"

def test_e2e_creategetdeleteroles(create_session, base_url):
    """Running an E2E test for create and delete service account."""

    role_name = f"role_{random.randint(1000, 9999)}"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
    "name": role_name
           }

    resp_create_role = session.post(
        f"{base_url}api/{org_id}/roles", json=payload
    )

    print(resp_create_role.content)
    assert (
        resp_create_role.status_code == 200
    ), f"Expected 200, but got {resp_create_role.status_code} {resp_create_role.content}"
    resp_get_role = session.get(
    
    f"{base_url}api/{org_id}/roles/{role_name}/permissions/stream"
    )

    assert (
        resp_get_role.status_code == 200
    ), f"Getting this service account, but got {resp_get_role.status_code} {resp_get_role.content}"

    resp_delete_role = session.delete(
    f"{base_url}api/{org_id}/roles/{role_name}"
    )

    assert (
        resp_delete_role.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_role.status_code} {resp_delete_role.content}"

def test_e2e_creategetupdatedeleteroles(create_session, base_url):
    """Running an E2E test for create, get, update and delete service account."""

    role_name = f"role_{random.randint(1000, 9999)}"  # Make the name unique


    session = create_session
    # Create a role
    org_id = "default"

    payload = {
    "name": role_name
           }

    resp_create_role = session.post(
        f"{base_url}api/{org_id}/roles", json=payload
    )

    print(resp_create_role.content)
    assert (
        resp_create_role.status_code == 200
    ), f"Expected 200, but got {resp_create_role.status_code} {resp_create_role.content}"
    resp_get_role = session.get(
    
    f"{base_url}api/{org_id}/roles/{role_name}/permissions/logs"
    )

    assert (
        resp_get_role.status_code == 200
    ), f"Getting this service account, but got {resp_get_role.status_code} {resp_get_role.content}"

    payload = {
    "name": role_name
           }

    resp_update_role = session.put(
        f"{base_url}api/{org_id}/roles/{role_name}", json=payload
    )
    print(resp_update_role.content)
    assert (
        resp_update_role.status_code == 200
    ), f"Updating this service account, but got {resp_update_role.status_code} {resp_update_role.content}"
    

    resp_delete_role = session.delete(
    f"{base_url}api/{org_id}/roles/{role_name}"
    )

    assert (
        resp_delete_role.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_role.status_code} {resp_delete_role.content}"

def test_e2e_creategetrefreshdeleteroles(create_session, base_url):
    """Running an E2E test for create, get, refresh and delete service account."""

    role_name = f"role_{random.randint(1000, 9999)}"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
    "name": role_name
           }


    resp_create_role = session.post(
        f"{base_url}api/{org_id}/roles", json=payload
    )

    print(resp_create_role.content)
    assert (
        resp_create_role.status_code == 200
    ), f"Expected 200, but got {resp_create_role.status_code} {resp_create_role.content}"
    resp_get_role = session.get(
    
    f"{base_url}api/{org_id}/roles/{role_name}"
    )

    assert (
        resp_get_role.status_code == 200
    ), f"Getting this service account, but got {resp_get_role.status_code} {resp_get_role.content}"

    resp_refresh_role = session.put(
        f"{base_url}api/{org_id}/roles/{role_name}?rotateToken=true", json=payload
    )
    print(resp_refresh_role.content)
    assert (
        resp_refresh_role.status_code == 200
    ), f"Refreshing this service account, but got {resp_refresh_role.status_code} {resp_refresh_role.content}"
    

    resp_delete_role = session.delete(
    f"{base_url}api/{org_id}/roles/{role_name}"
    )

    assert (
        resp_delete_role.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_role.status_code} {resp_delete_role.content}"

@pytest.fixture
def role_payload():
    email = f"email_{random.randint(1000, 9999)}@gmail.com"
    return {
    "name": role_name
           }

def create_role(session, base_url, payload):
    org_id = "default"
    response = session.post(
        f"{base_url}api/{org_id}/roles",
        json=payload
    )
    assert response.status_code == HTTPStatus.OK
    return response.json()

def delete_role(session, base_url, email):
    org_id = "default"
    response = session.delete(
        f"{base_url}api/{org_id}/roles/{email}"
    )
    assert response.status_code == HTTPStatus.OK

@pytest.mark.parametrize("invalid_email", [
    "invalid_email",
    "",
    "email@",
    "@domain.com"
    ])

def test_create_role_invalid_email(create_session, base_url, invalid_email):
    payload = {
        "email": invalid_email,
        "organization": "default",
        "first_name": "",
        "last_name": ""
    }
    response = create_session.post(
        f"{base_url}api/default/roles",
        json=payload
    )
    assert response.status_code == HTTPStatus.BAD_REQUEST