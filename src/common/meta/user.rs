// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserRequest {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(skip_serializing)]
    pub role: UserRole,
    /// Is the user created via ldap flow.
    #[serde(default)]
    pub is_external: bool,
}

impl UserRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn to_new_dbuser(
        &self,
        password: String,
        salt: String,
        org: String,
        token: String,
        rum_token: String,
        is_external: bool,
        password_ext: String,
    ) -> DBUser {
        DBUser {
            email: self.email.clone(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            password,
            salt,
            organizations: vec![UserOrg {
                name: org,
                token,
                rum_token: Some(rum_token),
                role: self.role.clone(),
            }],
            is_external,
            password_ext: Some(password_ext),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DBUser {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(default)]
    pub salt: String,
    pub organizations: Vec<UserOrg>,
    #[serde(default)]
    pub is_external: bool,
    pub password_ext: Option<String>,
}

impl DBUser {
    pub fn get_user(&self, org_id: String) -> Option<User> {
        if self.organizations.is_empty() {
            return None;
        }

        let mut local = self.clone();
        local.organizations.retain(|org| org.name.eq(&org_id));
        if local.organizations.is_empty() {
            return None;
        }

        let org = local.organizations.first().unwrap();
        Some(User {
            email: local.email,
            first_name: local.first_name,
            last_name: local.last_name,
            password: local.password,
            role: org.role.clone(),
            org: org.name.clone(),
            token: org.token.clone(),
            rum_token: org.rum_token.clone(),
            salt: local.salt,
            is_external: self.is_external,
            password_ext: self.password_ext.clone(),
        })
    }

    pub fn get_all_users(&self) -> Vec<User> {
        let mut ret_val = vec![];
        if self.organizations.is_empty() {
            ret_val
        } else {
            for org in self.organizations.clone() {
                ret_val.push(User {
                    email: self.email.clone(),
                    first_name: self.first_name.clone(),
                    last_name: self.last_name.clone(),
                    password: self.password.clone(),
                    role: org.role,
                    org: org.name,
                    token: org.token,
                    rum_token: org.rum_token,
                    salt: self.salt.clone(),
                    is_external: self.is_external,
                    password_ext: self.password_ext.clone(),
                })
            }
            ret_val
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(default)]
    pub salt: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub rum_token: Option<String>,
    pub role: UserRole,
    pub org: String,
    /// Is the user authenticated and created via LDAP
    pub is_external: bool,
    pub password_ext: Option<String>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserOrg {
    pub name: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub rum_token: Option<String>,
    #[serde(default)]
    pub role: UserRole,
}

impl PartialEq for UserOrg {
    fn eq(&self, other: &Self) -> bool {
        !self.name.eq(&other.name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserOrgRole {
    pub role: UserRole,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Eq, PartialEq, Default)]
pub struct UpdateUser {
    #[serde(default)]
    pub change_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<UserRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema, EnumIter)]
pub enum UserRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "member")] // admin in OpenSource
    Member,
    #[serde(rename = "root")]
    Root,
    #[cfg(feature = "enterprise")]
    #[serde(rename = "viewer")] // read only user
    Viewer,
    #[cfg(feature = "enterprise")]
    #[serde(rename = "user")] // No access only login user
    User,
    #[cfg(feature = "enterprise")]
    #[serde(rename = "editor")]
    Editor,
    #[cfg(feature = "enterprise")]
    #[serde(rename = "service_account")]
    ServiceAccount,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Member => write!(f, "member"),
            UserRole::Root => write!(f, "root"),
            #[cfg(feature = "enterprise")]
            UserRole::Viewer => write!(f, "viewer"),
            #[cfg(feature = "enterprise")]
            UserRole::Editor => write!(f, "editor"),
            #[cfg(feature = "enterprise")]
            UserRole::User => write!(f, "user"),
            #[cfg(feature = "enterprise")]
            UserRole::ServiceAccount => write!(f, "service_account"),
        }
    }
}

impl UserRole {
    pub fn get_label(&self) -> String {
        match self {
            UserRole::Admin => "Admin".to_string(),
            UserRole::Member => "Member".to_string(),
            UserRole::Root => "Root".to_string(),
            #[cfg(feature = "enterprise")]
            UserRole::Viewer => "Viewer".to_string(),
            #[cfg(feature = "enterprise")]
            UserRole::Editor => "Editor".to_string(),
            #[cfg(feature = "enterprise")]
            UserRole::User => "User".to_string(),
            #[cfg(feature = "enterprise")]
            UserRole::ServiceAccount => "Service Account".to_string(),
        }
    }
}

// Implementing FromStr for UserRole
impl FromStr for UserRole {
    type Err = ();

    fn from_str(input: &str) -> Result<UserRole, Self::Err> {
        match input {
            "admin" => Ok(UserRole::Admin),
            "member" => Ok(UserRole::Member),
            "root" => Ok(UserRole::Root),
            #[cfg(feature = "enterprise")]
            "viewer" => Ok(UserRole::Viewer),
            #[cfg(feature = "enterprise")]
            "editor" => Ok(UserRole::Editor),
            #[cfg(feature = "enterprise")]
            "user" => Ok(UserRole::User),
            #[cfg(feature = "enterprise")]
            "service_account" => Ok(UserRole::ServiceAccount),
            #[cfg(feature = "enterprise")]
            _ => Ok(UserRole::User),
            #[cfg(not(feature = "enterprise"))]
            _ => Ok(UserRole::Admin),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub role: UserRole,
    #[serde(default)]
    pub is_external: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserList {
    pub data: Vec<UserResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SignInUser {
    pub name: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct SignInResponse {
    pub status: bool,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TokenValidationResponse {
    pub is_valid: bool,
    pub user_email: String,
    pub user_name: String,
    pub family_name: String,
    pub given_name: String,
    pub is_internal_user: bool,
    pub user_role: Option<UserRole>,
}
pub struct TokenValidationResponseBuilder {
    pub response: TokenValidationResponse,
}

/// Builder for creating a `TokenValidationResponse` from a `DBUser`.
impl TokenValidationResponseBuilder {
    /// Creates a new `TokenValidationResponseBuilder` from a `DBUser`.
    ///
    /// # Arguments
    ///
    /// * `user` - The `DBUser` object used to build the `TokenValidationResponse`.
    ///
    /// # Returns
    ///
    /// A `TokenValidationResponseBuilder` object.
    pub fn from_db_user(user: &DBUser) -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse {
                is_valid: true,
                user_email: user.email.clone(),
                is_internal_user: !user.is_external,
                user_role: None,
                user_name: user.first_name.clone(),
                given_name: user.first_name.clone(),
                family_name: user.last_name.clone(),
            },
        }
    }

    /// Creates a new `TokenValidationResponseBuilder` from a `User`.
    ///
    /// Arguments
    ///
    /// * `user` - The `User` object used to build the `TokenValidationResponse`.
    pub fn from_user(user: &User) -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse {
                is_valid: true,
                user_email: user.email.clone(),
                is_internal_user: !user.is_external,
                user_role: Some(user.role.clone()),
                user_name: user.first_name.clone(),
                given_name: user.first_name.clone(),
                family_name: user.last_name.clone(),
            },
        }
    }

    pub fn new() -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse::default(),
        }
    }

    pub fn is_valid(mut self, is_valid: bool) -> Self {
        self.response.is_valid = is_valid;
        self
    }

    pub fn user_email(mut self, user_email: String) -> Self {
        self.response.user_email = user_email;
        self
    }

    pub fn user_name(mut self, user_name: String) -> Self {
        self.response.user_name = user_name;
        self
    }

    pub fn family_name(mut self, family_name: String) -> Self {
        self.response.family_name = family_name;
        self
    }

    pub fn given_name(mut self, given_name: String) -> Self {
        self.response.given_name = given_name;
        self
    }

    pub fn is_internal_user(mut self, is_internal_user: bool) -> Self {
        self.response.is_internal_user = is_internal_user;
        self
    }

    pub fn user_role(mut self, user_role: Option<UserRole>) -> Self {
        self.response.user_role = user_role;
        self
    }

    pub fn build(self) -> TokenValidationResponse {
        TokenValidationResponse {
            is_valid: self.response.is_valid,
            user_email: self.response.user_email,
            user_name: self.response.user_name,
            family_name: self.response.family_name,
            given_name: self.response.given_name,
            is_internal_user: self.response.is_internal_user,
            user_role: self.response.user_role,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RoleOrg {
    pub role: UserRole,
    pub org: String,
    pub custom_role: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UserGroup {
    pub name: String,
    pub users: Option<std::collections::HashSet<String>>,
    pub roles: Option<std::collections::HashSet<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UserGroupRequest {
    pub add_users: Option<std::collections::HashSet<String>>,
    pub remove_users: Option<std::collections::HashSet<String>>,
    pub add_roles: Option<std::collections::HashSet<String>>,
    pub remove_roles: Option<std::collections::HashSet<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UserRoleRequest {
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RolesResponse {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthTokensExt {
    pub auth_ext: String,
    pub refresh_token: String,
    pub request_time: i64,
    pub expires_in: i64,
}
