import random  # Add this line
from http import HTTPStatus  # Add this import
import pytest
import json
import requests
from typing import Dict, Any
def test_get_service_accounts(create_session, base_url):
    """Running an E2E test for get all the service_accounts list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_service_accounts = session.get(f"{url}api/{org_id}/service_accounts")

    print(resp_get_service_accounts.content)
    assert (
        resp_get_service_accounts.status_code == 200
    ), f"Get all service accounts list 200, but got {resp_get_service_accounts.status_code} {resp_get_service_accounts.content}"

def test_get_service_account(create_session, base_url):
    """Running an E2E test for get all the service_accounts list."""
    session = create_session
    url = base_url
    org_id = "default"

    response = session.get(f"{url}api/{org_id}/service_accounts")

    assert response.status_code == HTTPStatus.OK, f"Expected {HTTPStatus.OK}, but got {response.status_code}"
    response_data = response.json()
    
    # Extract the 'data' field which contains the list of service accounts
    assert "data" in response_data, "Response should contain a 'data' field"
    data = response_data["data"]
    
    assert isinstance(data, list), "Response should be a list of service accounts"



def test_e2e_createdeleteserviceaccounts(create_session, base_url):
    """Running an E2E test for create and delete service account."""

    email_id = f"email_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
        "email":email_id,
        "organization":"default",
        "first_name":"",
        "last_name":""
        }

    resp_create_serviceaccount = session.post(
        f"{base_url}api/{org_id}/service_accounts", json=payload
    )

    print(resp_create_serviceaccount.content)
    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"
    resp_delete_serviceaccount = session.delete(
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_delete_serviceaccount.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"

def test_e2e_creategetdeleteserviceaccounts(create_session, base_url):
    """Running an E2E test for create and delete service account."""

    email_id = f"email_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
        "email":email_id,
        "organization":"default",
        "first_name":"",
        "last_name":""
        }

    resp_create_serviceaccount = session.post(
        f"{base_url}api/{org_id}/service_accounts", json=payload
    )

    print(resp_create_serviceaccount.content)
    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"
    resp_get_serviceaccount = session.get(
    
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_get_serviceaccount.status_code == 200
    ), f"Getting this service account, but got {resp_get_serviceaccount.status_code} {resp_get_serviceaccount.content}"

    resp_delete_serviceaccount = session.delete(
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_delete_serviceaccount.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"

def test_e2e_creategetupdatedeleteserviceaccounts(create_session, base_url):
    """Running an E2E test for create, get, update and delete service account."""

    email_id = f"email_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
        "email":email_id,
        "organization":"default",
        "first_name":"",
        "last_name":""
        }

    resp_create_serviceaccount = session.post(
        f"{base_url}api/{org_id}/service_accounts", json=payload
    )

    print(resp_create_serviceaccount.content)
    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"
    resp_get_serviceaccount = session.get(
    
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_get_serviceaccount.status_code == 200
    ), f"Getting this service account, but got {resp_get_serviceaccount.status_code} {resp_get_serviceaccount.content}"

    payload = {
        "email":email_id,
        "organization":"default",
        "first_name":"updated",
        "last_name":"updated"
        }
    resp_update_serviceaccount = session.put(
        f"{base_url}api/{org_id}/service_accounts/{email_id}", json=payload
    )
    print(resp_update_serviceaccount.content)
    assert (
        resp_update_serviceaccount.status_code == 200
    ), f"Updating this service account, but got {resp_update_serviceaccount.status_code} {resp_update_serviceaccount.content}"
    

    resp_delete_serviceaccount = session.delete(
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_delete_serviceaccount.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"

def test_e2e_creategetrefreshdeleteserviceaccounts(create_session, base_url):
    """Running an E2E test for create, get, refresh and delete service account."""

    email_id = f"email_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique


    session = create_session
    # Create a service account
    org_id = "default"
    payload = {
        "email":email_id,
        "organization":"default",
        "first_name":"Get",
        "last_name":"Refreshed"
        }

    resp_create_serviceaccount = session.post(
        f"{base_url}api/{org_id}/service_accounts", json=payload
    )

    print(resp_create_serviceaccount.content)
    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"
    resp_get_serviceaccount = session.get(
    
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_get_serviceaccount.status_code == 200
    ), f"Getting this service account, but got {resp_get_serviceaccount.status_code} {resp_get_serviceaccount.content}"

    resp_refresh_serviceaccount = session.put(
        f"{base_url}api/{org_id}/service_accounts/{email_id}?rotateToken=true", json=payload
    )
    print(resp_refresh_serviceaccount.content)
    assert (
        resp_refresh_serviceaccount.status_code == 200
    ), f"Refreshing this service account, but got {resp_refresh_serviceaccount.status_code} {resp_refresh_serviceaccount.content}"
    

    resp_delete_serviceaccount = session.delete(
    f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    assert (
        resp_delete_serviceaccount.status_code == 200
    ), f"Deleting this service account, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"

@pytest.fixture
def service_account_payload():
    email = f"email_{random.randint(1000, 9999)}@gmail.com"
    return {
        "email": email,
        "organization": "default",
        "first_name": "",
        "last_name": ""
    }

def create_service_account(session, base_url, payload):
    org_id = "default"
    response = session.post(
        f"{base_url}api/{org_id}/service_accounts",
        json=payload
    )
    assert response.status_code == HTTPStatus.OK
    return response.json()

def delete_service_account(session, base_url, email):
    org_id = "default"
    response = session.delete(
        f"{base_url}api/{org_id}/service_accounts/{email}"
    )
    assert response.status_code == HTTPStatus.OK

@pytest.mark.parametrize("invalid_email", [
    "invalid_email",
    "",
    "email@",
    "@domain.com"
    ])

def test_create_service_account_invalid_email(create_session, base_url, invalid_email):
    payload = {
        "email": invalid_email,
        "organization": "default",
        "first_name": "",
        "last_name": ""
    }
    response = create_session.post(
        f"{base_url}api/default/service_accounts",
        json=payload
    )
    assert response.status_code == HTTPStatus.BAD_REQUEST