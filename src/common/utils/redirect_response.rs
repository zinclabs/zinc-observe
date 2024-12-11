// Copyright 2024 OpenObserve Inc.
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

use std::{collections::HashMap, fmt};

use actix_web::{http::header::ContentType, HttpResponse, ResponseError};

const DEFAULT_REDIRECT_RELATIVE_URI: &str = "/web/";

#[derive(Debug)]
pub struct RedirectResponse {
    redirect_relative_uri: String,
    query_params: HashMap<String, String>,
}

impl RedirectResponse {
    pub fn redirect_http(&self) -> HttpResponse {
        self.build_redirect_response()
    }

    fn build_full_redirect_uri(&self) -> String {
        let mut redirect_uri = self.redirect_relative_uri.clone();

        // If there are query parameters, append them to the URI.
        if !self.query_params.is_empty() {
            let query_string: String = self
                .query_params
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>()
                .join("&");

            redirect_uri = format!("{}?{}", redirect_uri, query_string);
        }

        redirect_uri
    }

    fn build_redirect_response(&self) -> HttpResponse {
        let redirect_uri = self.build_full_redirect_uri();
        if redirect_uri.len() < 1024 {
            HttpResponse::Found()
                .append_header(("Location", redirect_uri))
                .finish()
        } else {
            let html = format!(
                r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="utf-8">
                    <meta http-equiv="refresh" content="0;url={redirect_uri}">
                    <title>OpenObserve Redirecting...</title>
                </head>
                <body>
                    Redirecting to <a href="{redirect_uri}">click here</a>
                </body>
                </html>"#
            );
            HttpResponse::Found()
                .content_type(ContentType::html())
                .body(html)
        }
    }
}

impl fmt::Display for RedirectResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redirect_uri = self.build_full_redirect_uri();
        write!(f, "Redirecting to {}", redirect_uri)
    }
}

impl ResponseError for RedirectResponse {
    /// Generate an HTTP response that performs a redirection to the stored URL.
    fn error_response(&self) -> HttpResponse {
        self.build_redirect_response()
    }
}

pub struct RedirectResponseBuilder {
    redirect_relative_uri: String,
    query_params: HashMap<String, String>,
}

impl RedirectResponseBuilder {
    /// Create a new `RedirectResponseBuilder` with a given redirection URL.
    ///
    /// If no URL is provided, a default URL (`/web`) will be used.
    pub fn new(redirect_relative_uri: &str) -> Self {
        RedirectResponseBuilder {
            redirect_relative_uri: redirect_relative_uri.to_string(),
            query_params: HashMap::new(),
        }
    }

    pub fn with_query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> RedirectResponse {
        RedirectResponse {
            redirect_relative_uri: self.redirect_relative_uri,
            query_params: self.query_params,
        }
    }
}

impl Default for RedirectResponseBuilder {
    fn default() -> Self {
        RedirectResponseBuilder {
            redirect_relative_uri: DEFAULT_REDIRECT_RELATIVE_URI.to_string(),
            query_params: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::LOCATION, HttpResponse};

    use super::*;

    #[test]
    fn test_redirect_with_short_url() {
        // Build a RedirectResponse with a short_url query parameter
        let redirect_response = RedirectResponseBuilder::new("/web")
            .with_query_param(
                "redirect_url",
                "http://localhost:5080/api/default/short/1234",
            )
            .build();

        // Check if the constructed URI is correct
        let expected_uri = "/web?redirect_url=http://localhost:5080/api/default/short/1234";
        assert_eq!(redirect_response.build_full_redirect_uri(), expected_uri);

        // Check if the HTTP response contains the correct "Location" header
        let http_response: HttpResponse = redirect_response.redirect_http();
        assert_eq!(http_response.status(), actix_web::http::StatusCode::FOUND);
        assert_eq!(http_response.headers().get(LOCATION).unwrap(), expected_uri);
    }

    #[test]
    fn test_redirect_without_query_params() {
        // Build a RedirectResponse without any query parameters
        let redirect_response = RedirectResponseBuilder::new("/web").build();

        // Check if the constructed URI is correct (no query params)
        let expected_uri = "/web";
        assert_eq!(redirect_response.build_full_redirect_uri(), expected_uri);

        // Check if the HTTP response contains the correct "Location" header
        let http_response: HttpResponse = redirect_response.redirect_http();
        assert_eq!(http_response.status(), actix_web::http::StatusCode::FOUND);
        assert_eq!(http_response.headers().get(LOCATION).unwrap(), expected_uri);
    }

    #[test]
    fn test_redirect_with_multiple_query_params() {
        // Build a RedirectResponse with multiple query parameters
        let redirect_response = RedirectResponseBuilder::new("/web")
            .with_query_param(
                "redirect_url",
                "http://localhost:5080/api/default/short/1234",
            )
            .with_query_param("user_id", "42")
            .build();

        // Check if the constructed URI is correct with multiple query parameters
        let expected_uri_1 =
            "/web?redirect_url=http://localhost:5080/api/default/short/1234&user_id=42";
        let expected_uri_2 =
            "/web?user_id=42&redirect_url=http://localhost:5080/api/default/short/1234";
        assert!(
            redirect_response.build_full_redirect_uri() == expected_uri_1
                || redirect_response.build_full_redirect_uri() == expected_uri_2
        );

        // Check if the HTTP response contains the correct "Location" header
        let http_response: HttpResponse = redirect_response.redirect_http();
        assert_eq!(http_response.status(), actix_web::http::StatusCode::FOUND);
        // assert_eq!(http_response.headers().get(LOCATION).unwrap(), expected_uri);
        let location = http_response.headers().get(LOCATION).unwrap();
        assert!(location == expected_uri_1 || location == expected_uri_2);
    }
}
