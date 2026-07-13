use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use bevy::log::{info, warn, debug, trace};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidateResponse {
    pub uid: i32,
    pub username: String,
}

pub fn validate_user_ukey(ukey: &str) -> Result<ValidateResponse, String> {
    if ukey == "studio_play_local_key" || ukey.starts_with("offline_") {
        return Ok(ValidateResponse {
            uid: 1,
            username: "LocalPlayer".to_string(),
        });
    }

    let domain = std::env::var("VERTIGO_API_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string());
    let mut api_key = std::env::var("GAMESERVER_API_KEY").map_err(|_| "GAMESERVER_API_KEY environment variable is not configured".to_string())?;

    api_key = api_key.trim().trim_matches('"').to_string();

    trace!("API_LOG: Starting validation with domain={}, api_key_length={}", domain, api_key.len());

    let (host, port) = if let Some(pos) = domain.find(':') {
        let (h, p) = domain.split_at(pos);
        (h.to_string(), p[1..].to_string())
    } else {
        (domain.clone(), "80".to_string())
    };

    let address = format!("{}:{}", host, port);
    let addr = match address.parse() {
        Ok(ip) => ip,
        Err(_) => {
            use std::net::ToSocketAddrs;
            match address.to_socket_addrs() {
                Ok(addrs) => {
                    let mut chosen_addr = None;
                    for a in addrs {
                        if a.is_ipv4() {
                            chosen_addr = Some(a);
                            break;
                        }
                    }
                    let addr = match chosen_addr {
                        Some(a) => a,
                        None => {
                            use std::net::ToSocketAddrs;
                            match address.to_socket_addrs().ok().and_then(|mut iter| iter.next()) {
                                Some(first) => first,
                                None => return Err("DNS resolution returned no addresses".to_string()),
                            }
                        }
                    };
                    addr
                }
                Err(e) => return Err(format!("DNS resolution failed: {}", e)),
            }
        }
    };
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| e.to_string())?;

    stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;

    let path = format!("/api/v1/auth/validate?ukey={}", ukey);
    let req_str = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         X-Gameserver-Key: {}\r\n\
         Connection: close\r\n\r\n",
        path, domain, api_key
    );

    stream.write_all(req_str.as_bytes()).map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|e| e.to_string())?;

    let response_str = String::from_utf8_lossy(&response);
    let mut parts = response_str.splitn(2, "\r\n\r\n");
    let headers = parts.next().ok_or("No headers in response")?;
    let body = parts.next().ok_or("No body in response")?;

    if !headers.contains("HTTP/1.1 200") && !headers.contains("HTTP/1.0 200") {
        warn!("API_LOG: Go backend returned !!non-200!! response headers: {}", headers);
        return Err(format!("Server returned error: {}", headers));
    }

    let res_data: ValidateResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    debug!("API_LOG: Successfully validated client ukey uid={}, username={}", res_data.uid, res_data.username);
    Ok(res_data)
}