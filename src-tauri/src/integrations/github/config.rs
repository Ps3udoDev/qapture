pub const API_BASE: &str = "https://api.github.com";
pub const WEB_BASE: &str = "https://github.com";
pub const CLIENT_ID: &str = "Iv23liai2kBF7qDefVd8";
pub const APP_SLUG: &str = "qapture-qa";
pub const API_VERSION: &str = "2022-11-28";
pub const INSTALL_URL: &str = "https://github.com/apps/qapture-qa/installations/new";
pub const REVOKE_URL: &str = "https://github.com/settings/apps/authorizations";

pub fn user_agent() -> String {
    format!("Qapture/{}", env!("CARGO_PKG_VERSION"))
}
