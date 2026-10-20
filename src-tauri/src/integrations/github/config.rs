pub const API_BASE: &str = "https://api.github.com";
pub const WEB_BASE: &str = "https://github.com";
pub const CLIENT_ID: &str = "Iv23liai2kBF7qDefVd8";
// Solo la comprueba un test: `INSTALL_URL` es literal para poder ser `&'static str`.
#[allow(dead_code)]
pub const APP_SLUG: &str = "qapture-qa";
pub const API_VERSION: &str = "2022-11-28";
pub const INSTALL_URL: &str = "https://github.com/apps/qapture-qa/installations/new";
pub const REVOKE_URL: &str = "https://github.com/settings/apps/authorizations";

pub fn user_agent() -> String {
    format!("Qapture/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_install_url_points_to_the_app_slug() {
        assert_eq!(
            INSTALL_URL,
            format!("{WEB_BASE}/apps/{APP_SLUG}/installations/new")
        );
        assert!(REVOKE_URL.starts_with(WEB_BASE));
    }
}
