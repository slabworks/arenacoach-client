const DEV_API_BASE: &str = "https://arenacoach-web.test";
const PROD_API_BASE: &str = "https://arenacoach.com";

pub fn is_dev() -> bool {
    cfg!(debug_assertions)
}

pub fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "other"
    }
}

pub fn api_base_for_mode(developer_mode: bool) -> String {
    if developer_mode {
        DEV_API_BASE
    } else {
        PROD_API_BASE
    }
    .to_string()
}

pub fn matches_url(api_base: &str) -> String {
    format!("{}/api/matches", api_base.trim_end_matches('/'))
}

pub fn matches_page_url(api_base: &str, page: u32) -> String {
    format!("{}?page={}&summary=1", matches_url(api_base), page.max(1))
}

pub fn health_url(api_base: &str) -> String {
    format!("{}/up", api_base.trim_end_matches('/'))
}

pub fn device_login_url(api_base: &str) -> String {
    format!("{}/api/device/login", api_base.trim_end_matches('/'))
}

pub fn device_logout_url(api_base: &str) -> String {
    format!("{}/api/device/logout", api_base.trim_end_matches('/'))
}

pub fn realtime_url(api_base: &str) -> String {
    format!("{}/api/realtime", api_base.trim_end_matches('/'))
}

pub fn broadcasting_auth_url(api_base: &str) -> String {
    format!("{}/api/broadcasting/auth", api_base.trim_end_matches('/'))
}

pub fn match_url(api_base: &str, client_match_id: &str) -> String {
    format!(
        "{}/api/matches/{}",
        api_base.trim_end_matches('/'),
        client_match_id
    )
}

pub fn card_image_url(api_base: &str, grp_id: u32) -> String {
    format!(
        "{}/api/cards/{}/image",
        api_base.trim_end_matches('/'),
        grp_id
    )
}

pub fn user_url(api_base: &str) -> String {
    format!("{}/api/user", api_base.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_url_strips_trailing_slash() {
        assert_eq!(
            matches_url("https://arenacoach-web.test/"),
            "https://arenacoach-web.test/api/matches"
        );
        assert_eq!(
            device_login_url("https://arenacoach-web.test/"),
            "https://arenacoach-web.test/api/device/login"
        );
        assert_eq!(
            realtime_url("https://arenacoach-web.test/"),
            "https://arenacoach-web.test/api/realtime"
        );
        assert_eq!(
            match_url("https://arenacoach-web.test/", "match-1"),
            "https://arenacoach-web.test/api/matches/match-1"
        );
        assert_eq!(
            card_image_url("https://arenacoach-web.test/", 73951),
            "https://arenacoach-web.test/api/cards/73951/image"
        );
        assert_eq!(
            matches_page_url("https://arenacoach-web.test/", 2),
            "https://arenacoach-web.test/api/matches?page=2&summary=1"
        );
        assert_eq!(
            user_url("https://arenacoach-web.test/"),
            "https://arenacoach-web.test/api/user"
        );
    }

    #[test]
    fn mode_selects_endpoint_independently_of_build() {
        assert_eq!(api_base_for_mode(false), "https://arenacoach.com");
        assert_eq!(api_base_for_mode(true), "https://arenacoach-web.test");
    }
}
