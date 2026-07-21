use bevy::log::debug;
use reqwest::{
    StatusCode, Url,
    blocking::{Client, Response},
    header::{HeaderMap, HeaderValue},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::IpAddr;
use std::time::Duration;

const STUDIO_LOCAL_KEY: &str = "studio_play_local_key";
const MAX_TOKEN_LENGTH: usize = 4096;
const MAX_API_KEY_LENGTH: usize = 4096;
const MAX_USERNAME_LENGTH: usize = 128;
const MAX_RESPONSE_SIZE: usize = 64 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidateResponse {
    pub uid: i32,
    pub username: String,
}

pub(crate) fn is_studio_local_key(ukey: &str) -> bool {
    ukey == STUDIO_LOCAL_KEY
}

pub fn validate_user_ukey_offline(ukey: &str) -> Result<ValidateResponse, String> {
    if is_studio_local_key(ukey) {
        return Ok(ValidateResponse {
            uid: 1,
            username: "LocalPlayer".to_string(),
        });
    }
    Err("Invalid offline key".to_string())
}

pub fn validate_user_ukey(ukey: &str, is_local: bool) -> Result<ValidateResponse, String> {
    if is_local && is_studio_local_key(ukey) {
        return validate_user_ukey_offline(ukey);
    }

    validate_token_length(ukey)?;

    let configured_domain =
        std::env::var("VERTIGO_API_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string());
    let mut url = backend_url(&configured_domain)?;
    url.set_path("/api/v1/auth/validate");
    url.set_query(None);
    url.query_pairs_mut().append_pair("ukey", ukey);

    let api_key = std::env::var("GAMESERVER_API_KEY")
        .map_err(|_| "GAMESERVER_API_KEY environment variable is not configured".to_string())?;
    let api_key = api_key.trim().trim_matches('"');
    if api_key.is_empty() || api_key.len() > MAX_API_KEY_LENGTH {
        return Err("GAMESERVER_API_KEY has an invalid length".to_string());
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Gameserver-Key",
        HeaderValue::from_str(api_key)
            .map_err(|_| "GAMESERVER_API_KEY contains invalid characters".to_string())?,
    );
    let client = Client::builder()
        .default_headers(headers)
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| "Failed to initialize authentication client".to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|_| "Authentication backend request failed".to_string())?;
    let res_data = read_validate_response(response)?;
    debug!(
        "Authentication backend validated uid={}, username={}",
        res_data.uid, res_data.username
    );
    Ok(res_data)
}

fn validate_token_length(ukey: &str) -> Result<(), String> {
    if ukey.is_empty() || ukey.len() > MAX_TOKEN_LENGTH {
        return Err("Authentication token has an invalid length".to_string());
    }
    Ok(())
}

fn backend_url(configured: &str) -> Result<Url, String> {
    let configured = configured.trim();
    let candidate = if configured.contains("://") {
        configured.to_string()
    } else if configured.eq_ignore_ascii_case("localhost")
        || configured
            .split_once(':')
            .is_some_and(|(host, _)| host.eq_ignore_ascii_case("localhost"))
        || configured
            .split_once(':')
            .and_then(|(host, _)| host.parse::<IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback())
        || configured
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        format!("http://{configured}")
    } else {
        format!("https://{configured}")
    };

    let url =
        Url::parse(&candidate).map_err(|_| "VERTIGO_API_DOMAIN is not a valid URL".to_string())?;
    if !url.username().is_empty() || url.password().is_some() || url.host().is_none() {
        return Err("VERTIGO_API_DOMAIN must not contain credentials".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("VERTIGO_API_DOMAIN must not contain a query or fragment".to_string());
    }

    match url.scheme() {
        "https" => Ok(url),
        "http" if is_loopback_host(&url) => Ok(url),
        "http" => Err("Plain HTTP authentication is allowed only for loopback URLs".to_string()),
        _ => Err("VERTIGO_API_DOMAIN must use HTTPS".to_string()),
    }
}

fn is_loopback_host(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    })
}

fn read_validate_response(response: Response) -> Result<ValidateResponse, String> {
    let status = response.status();
    if status != StatusCode::OK {
        return Err("Authentication backend rejected request".to_string());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_SIZE as u64)
    {
        return Err("Authentication backend response is too large".to_string());
    }

    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_SIZE as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|_| "Failed to read authentication backend response".to_string())?;
    validate_response(status, &body)
}

fn validate_response(status: StatusCode, body: &[u8]) -> Result<ValidateResponse, String> {
    if status != StatusCode::OK {
        return Err("Authentication backend rejected request".to_string());
    }
    if body.len() > MAX_RESPONSE_SIZE {
        return Err("Authentication backend response is too large".to_string());
    }

    let response: ValidateResponse = serde_json::from_slice(body)
        .map_err(|_| "Authentication backend returned an invalid response".to_string())?;
    if response.username.is_empty() || response.username.len() > MAX_USERNAME_LENGTH {
        return Err("Authentication backend returned an invalid username".to_string());
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_url_requires_https_except_for_loopback() {
        assert_eq!(backend_url("api.example.com").unwrap().scheme(), "https");
        assert_eq!(backend_url("localhost:3000").unwrap().scheme(), "http");
        assert!(backend_url("http://127.0.0.1:3000").is_ok());
        assert!(backend_url("http://[::1]:3000").is_ok());
        assert!(backend_url("http://api.example.com").is_err());
        assert!(backend_url("ftp://api.example.com").is_err());
    }

    #[test]
    fn response_requires_exact_success_and_bounded_username() {
        let body = br#"{"uid":7,"username":"player"}"#;
        assert!(validate_response(StatusCode::OK, body).is_ok());
        assert!(validate_response(StatusCode::CREATED, body).is_err());

        let oversized = serde_json::to_vec(&ValidateResponse {
            uid: 7,
            username: "x".repeat(MAX_USERNAME_LENGTH + 1),
        })
        .unwrap();
        assert!(validate_response(StatusCode::OK, &oversized).is_err());
        assert!(validate_response(StatusCode::OK, &vec![b'x'; MAX_RESPONSE_SIZE + 1]).is_err());
    }

    #[test]
    fn token_length_is_bounded() {
        assert!(validate_token_length("").is_err());
        assert!(validate_token_length("token").is_ok());
        assert!(validate_token_length(&"x".repeat(MAX_TOKEN_LENGTH + 1)).is_err());
    }

    #[test]
    fn query_pairs_encode_token_values() {
        let mut url = backend_url("https://api.example.com").unwrap();
        url.query_pairs_mut().append_pair("ukey", "a&b c/ü");
        assert_eq!(url.query(), Some("ukey=a%26b+c%2F%C3%BC"));
    }
}
