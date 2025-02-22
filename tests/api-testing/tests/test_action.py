import base64
import os
from pathlib import Path
import random
import requests
import pytest

# Constants for WebSocket URL and user credentials
ZO_BASE_URL_SC = os.environ.get("ZO_BASE_URL_SC")  # Use environment variable
ZO_BASE_URL = os.environ.get("ZO_BASE_URL")  # Use environment variable
ZO_ROOT_USER_EMAIL = os.environ.get("ZO_ROOT_USER_EMAIL")  # Use environment variable
ZO_ROOT_USER_PASSWORD = os.environ.get("ZO_ROOT_USER_PASSWORD")  # Use environment variable
org_id = "default"

# Define the email_id for the service account
email_id = f"em_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique
action_name = f"ac_{random.randint(1000, 9999)}"  # Make the name unique

root_dir = Path(__file__).parent.parent.parent

def base_url():
    """Provide the base URL for the API of Super Cluster."""
    return ZO_BASE_URL

# def test_get_actions(create_session, base_url):
#     """Running an E2E test for getting all the actions list."""
#     session = create_session
#     resp_get_allactions = session.get(f"{base_url}api/{org_id}/actions")

#     print("Actions all: ", resp_get_allactions.content)
#     assert (
#         resp_get_allactions.status_code == 200
#     ), f"Expected 200, but got {resp_get_allactions.status_code} {resp_get_allactions.content}"

def test_create_serviceaccount(create_session: requests.Session, base_url: str):
    """Running an E2E test for creating and deleting a service account."""
    session = create_session
    # Create a service account
    payload_serviceaccounts = {
        "email": email_id,
        "organization": org_id,
        "first_name": "Action",
        "last_name": "Services",
    }

    resp_create_serviceaccount = session.post(
        f"{ZO_BASE_URL}api/{org_id}/service_accounts", json=payload_serviceaccounts
    )

    print("Service Account Created", resp_create_serviceaccount.content)

    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"


# def test_create_action(create_session, base_url):
BOUNDARY = "----WebKitFormBoundaryaQgmYHuE6dQrlLss"

def create_multipart_data(fields, boundary):
    """Create multipart form data with a specified boundary."""
    boundary_str = f"--{boundary}"
    lines = []

    # Add the fields to the multipart data
    for key, value in fields.items():
        if isinstance(value, tuple):
            # This is the file field
            filename, file_obj, content_type = value
            lines.append(f"{boundary_str}")
            lines.append(f'Content-Disposition: form-data; name="{key}"; filename="{filename}"')
            lines.append(f'Content-Type: {content_type}')
            lines.append("")
            # Read the file content as binary
            lines.append(file_obj.read())  # Read as binary data
        else:
            # Regular fields
            lines.append(f"{boundary_str}")
            lines.append(f'Content-Disposition: form-data; name="{key}"')
            lines.append("")
            lines.append(value)

    # End the multipart data
    lines.append(f"{boundary_str}--")
    
    return b"\r\n".join(line.encode('utf-8') if isinstance(line, str) else line for line in lines)



 
    
    
    


@pytest.fixture
def upload_file(create_session: requests.Session):
    """Fixture to handle the file upload."""
    session = create_session
    URL_ACTION = f"{ZO_BASE_URL}api/{org_id}/actions/upload"
    print(f"Constructed URL: {URL_ACTION}")

    # Open the ZIP file and prepare the multipart/form-data request
    zip_file_path = root_dir / "test-data/test1234_py.zip"
    print(f"Looking for file at: {zip_file_path}")

    # Ensure the file exists
    with open(zip_file_path, 'rb') as f:
        fields = {
            'name': action_name,
            'description': 'Action description',
            'execution_details': 'Once',
            'service_account': email_id,
            'owner': ZO_ROOT_USER_EMAIL,
            'environment_variables': '{}',
            'file': ('test1234_py.zip', f, 'application/zip'),  # Use tuple for file details
            'filename': 'test1234_py.zip'
        }

        # Create the multipart form data
        multipart_data = create_multipart_data(fields, BOUNDARY)

    # Define the headers for the request
    HEADERS = {
        'accept': 'application/json, text/plain, */*',
        'accept-language': 'en-US,en;q=0.9',
        'cache-control': 'no-cache',
        'Content-Type': f'multipart/form-data; boundary={BOUNDARY}',
        'cookie': f"auth_tokens={{\"access_token\":\"Basic {base64.b64encode((ZO_ROOT_USER_EMAIL + ':' + ZO_ROOT_USER_PASSWORD).encode()).decode()}\",\"refresh_token\":\"\"}}",
        'origin': ZO_BASE_URL,
        'pragma': 'no-cache',
        'priority': 'u=1, i',
        'referer': f"{ZO_BASE_URL}/web/action-scripts?action=add&org_identifier={org_id}",
        'user-agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36'
    }

    # Perform the API request
    response = session.post(URL_ACTION, headers=HEADERS, data=multipart_data)

    print(f"Upload response status: {response.status_code}, content: {response.content}")
    return response


def test_upload_file(upload_file: requests.Response):
    """Test the file upload API."""
    response = upload_file
    
    print("Upload File Response:", response.text)  # Print the response body for debugging
    print(f"Response Status Code: {response.status_code}")
    print("Response Headers:", response.headers)

    # Check if the response is successful
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"

    # Optionally, check the response content
    response_json = response.json()
    assert 'uuid' in response_json, "Response does not contain 'uuid' key"

    # Extract the UUID
    extracted_uuid = response_json['uuid']
    print("Extracted UUID:", extracted_uuid)  # Print the extracted UUID for debugging

   

    
   

if __name__ == "__main__":
    pytest.main()


# def test_delete_serviceaccount(create_session, base_url):
#     """Running an E2E test for deleting a service account."""
#     session = create_session
    
#     # Delete the service account
#     resp_delete_serviceaccount = session.delete(
#         f"{base_url}api/{org_id}/service_accounts/{email_id}"
#     )

#     print("Service Account Deleted", resp_delete_serviceaccount.content)

#     assert (
#         resp_delete_serviceaccount.status_code == 200
#     ), f"Expected 200, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"
