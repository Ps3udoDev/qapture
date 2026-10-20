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

/// `true` solo para una URL con el mismo esquema, host y puerto que `WEB_BASE`
/// (`https://github.com`) y sin credenciales: lo único que Qapture abre o enlaza como issue.
pub fn is_web_url(url: &str) -> bool {
    let (Ok(base), Ok(url)) = (reqwest::Url::parse(WEB_BASE), reqwest::Url::parse(url)) else {
        return false;
    };
    url.scheme() == base.scheme()
        && url.host_str() == base.host_str()
        && url.port_or_known_default() == base.port_or_known_default()
        && url.username().is_empty()
        && url.password().is_none()
}

/// `true` si `url` es exactamente `<WEB_BASE>/<owner>/<repo>/issues/<number>`; dueño y repo
/// sin distinguir mayúsculas, como en GitHub.
pub fn is_issue_url(url: &str, owner: &str, repo: &str, number: u64) -> bool {
    url.eq_ignore_ascii_case(&format!("{WEB_BASE}/{owner}/{repo}/issues/{number}"))
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

    #[test]
    fn only_https_github_urls_are_web_urls() {
        assert!(is_web_url("https://github.com/o/r/issues/1"));
        assert!(is_web_url("https://GitHub.com/o/r/issues/1"));
        for bad in [
            "http://github.com/o/r/issues/1",
            "https://github.com.evil.example/o/r",
            "https://evil.example/o/r",
            "https://u:p@github.com/o/r",
            "https://github.com:8443/o/r",
            "file:///C:/x.exe",
            "javascript:alert(1)",
            "no es una url",
            "",
        ] {
            assert!(!is_web_url(bad), "{bad}");
        }
    }

    #[test]
    fn issue_url_matches_case_insensitively() {
        assert!(is_issue_url(
            "https://github.com/O/R/issues/45",
            "o",
            "r",
            45
        ));
        assert!(!is_issue_url(
            "https://github.com/o/r/issues/46",
            "o",
            "r",
            45
        ));
        assert!(!is_issue_url(
            "https://github.com/o/x/issues/45",
            "o",
            "r",
            45
        ));
        assert!(!is_issue_url(
            "https://github.com/o/r/issues/45/../../x",
            "o",
            "r",
            45
        ));
    }
}
