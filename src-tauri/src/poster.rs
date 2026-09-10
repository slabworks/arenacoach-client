use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::config::{
    broadcasting_auth_url, card_image_url, device_login_url, device_logout_url, health_url, is_dev,
    match_url, matches_page_url, matches_url, realtime_url, user_url,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub email: String,
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
    #[serde(default)]
    pub player_seat: Option<u32>,
    #[serde(default)]
    pub deck_grp_ids: Option<Vec<u32>>,
    #[serde(default)]
    pub timeline: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub cards: HashMap<String, CardRef>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRef {
    pub name: String,
    #[serde(default)]
    pub type_line: Option<String>,
    #[serde(default)]
    pub oracle_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPage {
    pub data: Vec<MatchReport>,
    pub meta: MatchPageMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPageMeta {
    pub current_page: u32,
    pub last_page: u32,
    pub total: u32,
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

#[derive(Debug, Deserialize)]
struct CardImageResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    data: UserData,
}

#[derive(Debug, Deserialize)]
struct UserData {
    id: i64,
    name: String,
    email: String,
}

fn json_headers(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .header("Accept", "application/json")
        .bearer_auth(token)
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

pub async fn create_user(
    client: &reqwest::Client,
    api_base: &str,
    name: &str,
    email: &str,
    password: &str,
    password_confirmation: &str,
) -> Result<Account, PostError> {
    let response = client
        .post(user_url(api_base))
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "name": name,
            "email": email,
            "password": password,
            "password_confirmation": password_confirmation,
        }))
        .send()
        .await?;
    parse_account_response(response).await
}

pub async fn fetch_user(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
) -> Result<Account, PostError> {
    let response = json_headers(client.get(user_url(api_base)), token)
        .send()
        .await?;
    parse_account_response(response).await
}

pub async fn update_user(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    name: &str,
    email: &str,
    current_password: Option<&str>,
    password: Option<&str>,
    password_confirmation: Option<&str>,
) -> Result<Account, PostError> {
    let mut payload = serde_json::json!({
        "name": name,
        "email": email,
    });
    if let Some(current_password) = current_password.filter(|value| !value.is_empty()) {
        payload["current_password"] = serde_json::Value::String(current_password.to_string());
    }
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        payload["password"] = serde_json::Value::String(password.to_string());
    }
    if let Some(password_confirmation) = password_confirmation.filter(|value| !value.is_empty()) {
        payload["password_confirmation"] =
            serde_json::Value::String(password_confirmation.to_string());
    }

    let response = json_headers(client.put(user_url(api_base)), token)
        .json(&payload)
        .send()
        .await?;
    parse_account_response(response).await
}

pub async fn delete_user(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    password: &str,
) -> Result<(), PostError> {
    let response = json_headers(client.delete(user_url(api_base)), token)
        .json(&serde_json::json!({ "password": password }))
        .send()
        .await?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(());
    }
    Err(PostError::Http {
        status,
        body: response.text().await.unwrap_or_default(),
    })
}

async fn parse_account_response(response: reqwest::Response) -> Result<Account, PostError> {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        return parse_account_body(&body);
    }
    Err(PostError::Http { status, body })
}

fn parse_account_body(body: &str) -> Result<Account, PostError> {
    let parsed: UserResponse = serde_json::from_str(body)?;
    Ok(Account {
        id: parsed.data.id,
        name: parsed.data.name,
        email: parsed.data.email,
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
    let response = json_headers(client.get(match_url(api_base, client_match_id)), token)
        .send()
        .await?;
    parse_match_response(response).await
}

pub async fn fetch_matches(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    page: u32,
) -> Result<MatchPage, PostError> {
    let response = json_headers(client.get(matches_page_url(api_base, page)), token)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        return Ok(serde_json::from_str(&body)?);
    }
    Err(PostError::Http { status, body })
}

pub async fn delete_match(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    client_match_id: &str,
) -> Result<(), PostError> {
    let response = json_headers(client.delete(match_url(api_base, client_match_id)), token)
        .send()
        .await?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(());
    }
    Err(PostError::Http {
        status,
        body: response.text().await.unwrap_or_default(),
    })
}

async fn parse_match_response(response: reqwest::Response) -> Result<MatchReport, PostError> {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        let parsed: MatchResponse = serde_json::from_str(&body)?;
        return Ok(parsed.data);
    }
    Err(PostError::Http { status, body })
}

pub async fn fetch_card_image(
    client: &reqwest::Client,
    api_base: &str,
    token: &str,
    grp_id: u32,
) -> Result<Option<String>, PostError> {
    if grp_id < 1 {
        return Ok(None);
    }

    let response = json_headers(client.get(card_image_url(api_base, grp_id)), token)
        .send()
        .await?;
    let status = response.status().as_u16();
    if status == 404 {
        return Ok(None);
    }

    let body = response.text().await.unwrap_or_default();
    if (200..300).contains(&status) {
        let parsed: CardImageResponse = serde_json::from_str(&body)?;
        if parsed.url.is_empty() {
            return Ok(None);
        }
        return Ok(Some(parsed.url));
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

    #[test]
    fn reads_account_resource_from_the_api() {
        let account = parse_account_body(
            r#"{"data":{"id":7,"name":"Arena Pilot","email":"pilot@example.com"}}"#,
        )
        .expect("account json");
        assert_eq!(
            account,
            Account {
                id: 7,
                name: "Arena Pilot".into(),
                email: "pilot@example.com".into(),
            }
        );
    }
}
