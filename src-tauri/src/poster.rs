use serde::{Deserialize, Serialize};

use crate::config::{
    broadcasting_auth_url, device_login_url, device_logout_url, health_url, is_dev, match_url,
    matches_url, realtime_url,
};
use crate::match_payload::Match;

#[derive(Debug, Clone)]
pub struct PostResult {
    pub status: u16,
    #[allow(dead_code)]
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct DeviceSession {
    pub token: String,
    pub user_id: i64,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverbConfig {
    pub key: String,
    pub host: String,
    pub port: u16,
    pub scheme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    pub user_id: i64,
    pub channel: String,
    pub reverb: ReverbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchReport {
    pub id: i64,
    pub client_match_id: String,
    pub event_id: String,
    pub format: String,
    pub result: String,
    pub coaching_status: String,
    pub analysis: Option<String>,
    pub tips: Option<Vec<serde_json::Value>>,
    pub coaching_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PostError {
    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("two-factor authentication required")]
    TwoFactor,
    #[error("not signed in")]
    Unauthenticated,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error(transparent)]
    Encode(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    data: Option<LoginData>,
    two_factor: Option<bool>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginData {
    token: String,
    user: LoginUser,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    id: i64,
    email: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RealtimeResponse {
    data: RealtimeConfig,
}

#[derive(Debug, Deserialize)]
struct MatchResponse {
    data: MatchReport,
}

pub fn http_client() -> reqwest::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if is_dev() {
        builder = builder.danger_accept_invalid_certs(true);
    }
    builder.build()
}

pub fn should_retry(error: &PostError) -> bool {
    match error {
        PostError::Network(_) => true,
        PostError::Http { status, .. } => *status == 429 || *status >= 500,
        PostError::TwoFactor | PostError::Unauthenticated | PostError::Encode(_) => false,
    }
}

pub async fn post_match(
    client: &reqwest::Client,
    api_base: &str,
    token: Option<&str>,
    assembled: &Match,
) -> Result<PostResult, PostError> {
    if token.filter(|value| !value.is_empty()).is_none() {
        return Err(PostError::Unauthenticated);
    }
    let bytes = assembled.to_json_bytes()?;
    let url = matches_url(api_base);
    let mut request = client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(bytes);
    if let Some(token) = token.filter(|value| !value.is_empty()) {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        Ok(PostResult { status, body })
    } else {
        Err(PostError::Http { status, body })
    }
}

pub async fn post_match_with_retry(
    client: &reqwest::Client,
    api_base: &str,
    token: Option<&str>,
    assembled: &Match,
) -> Result<PostResult, PostError> {
    let mut attempt = 0u32;
    loop {
        match post_match(client, api_base, token, assembled).await {
            Ok(result) => return Ok(result),
            Err(error) if should_retry(&error) && attempt < 3 => {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_millis(500 * 2u64.pow(attempt))).await;
            }
            Err(error) => return Err(error),
        }
    }
}

pub async fn login_device(
    client: &reqwest::Client,
    api_base: &str,
    email: &str,
    password: &str,
    device_name: &str,
    code: Option<&str>,
    recovery_code: Option<&str>,
) -> Result<DeviceSession, PostError> {
    let mut payload = serde_json::json!({
        "email": email,
        "password": password,
        "device_name": device_name,
    });
    if let Some(code) = code.filter(|value| !value.is_empty()) {
        payload["code"] = serde_json::Value::String(code.to_string());
    }
    if let Some(recovery_code) = recovery_code.filter(|value| !value.is_empty()) {
        payload["recovery_code"] = serde_json::Value::String(recovery_code.to_string());
    }

    let response = client
        .post(device_login_url(api_base))
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    let parsed: LoginResponse = serde_json::from_str(&body).unwrap_or(LoginResponse {
        data: None,
        two_factor: None,
        message: None,
    });

    if parsed.two_factor == Some(true) {
        return Err(PostError::TwoFactor);
    }

    if (200..300).contains(&status) {
        if let Some(data) = parsed.data {
            return Ok(DeviceSession {
                token: data.token,
                user_id: data.user.id,
                email: data.user.email,
                name: data.user.name,
            });
        }
    }

    Err(PostError::Http {
        status,
        body: parsed.message.unwrap_or(body),
    })
}

pub async fn logout_device(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
) -> Result<(), PostError> {
    let response = client
        .delete(device_logout_url(api_base))
        .header("Accept", "application/json")
        .bearer_auth(token)
        .send()
        .await?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) || status == 401 {
        Ok(())
    } else {
        Err(PostError::Http {
            status,
            body: response.text().await.unwrap_or_default(),
        })
    }
}

pub async fn ping_host(client: &reqwest::Client, api_base: &str) -> Result<u16, PostError> {
    let response = client.get(health_url(api_base)).send().await?;
    Ok(response.status().as_u16())
}

pub async fn fetch_realtime(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
) -> Result<RealtimeConfig, PostError> {
    let response = client
        .get(realtime_url(api_base))
        .header("Accept", "application/json")
        .bearer_auth(token)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        let parsed: RealtimeResponse = serde_json::from_str(&body)?;
        return Ok(parsed.data);
    }
    Err(PostError::Http { status, body })
}

pub async fn fetch_match_report(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    client_match_id: &str,
) -> Result<MatchReport, PostError> {
    let response = client
        .get(match_url(api_base, client_match_id))
        .header("Accept", "application/json")
        .bearer_auth(token)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        let parsed: MatchResponse = serde_json::from_str(&body)?;
        return Ok(parsed.data);
    }
    Err(PostError::Http { status, body })
}

pub async fn authorize_broadcast(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    socket_id: &str,
    channel_name: &str,
) -> Result<serde_json::Value, PostError> {
    let response = client
        .post(broadcasting_auth_url(api_base))
        .header("Accept", "application/json")
        .bearer_auth(token)
        .json(&serde_json::json!({
            "socket_id": socket_id,
            "channel_name": channel_name,
        }))
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        return Ok(serde_json::from_str(&body)?);
    }
    Err(PostError::Http { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_server_and_network_class_errors() {
        assert!(should_retry(&PostError::Http {
            status: 503,
            body: "busy".into(),
        }));
        assert!(should_retry(&PostError::Http {
            status: 429,
            body: "slow down".into(),
        }));
        assert!(!should_retry(&PostError::Http {
            status: 422,
            body: "bad match".into(),
        }));
        assert!(!should_retry(&PostError::Unauthenticated));
        assert!(!should_retry(&PostError::TwoFactor));
    }
}
