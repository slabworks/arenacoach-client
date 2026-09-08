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

pub fn default_api_base() -> String {
    if let Ok(from_env) = std::env::var("ARENACOACH_API_BASE") {
        if !from_env.is_empty() {
            return from_env;
        }
    }
    if is_dev() {
        DEV_API_BASE.to_string()
    } else {
        PROD_API_BASE.to_string()
    }
}

pub fn matches_url(api_base: &str) -> String {
    format!("{}/api/matches", api_base.trim_end_matches('/'))
}

pub fn health_url(api_base: &str) -> String {
    format!("{}/up", api_base.trim_end_matches('/'))
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
    }

    #[test]
    fn debug_builds_default_to_herd_site() {
        if is_dev() {
            let base = default_api_base();
            if std::env::var("ARENACOACH_API_BASE").is_err() {
                assert_eq!(base, "https://arenacoach-web.test");
            }
        }
    }
}
