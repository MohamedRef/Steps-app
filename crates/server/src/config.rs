//! Worker configuration: `[vars]` from wrangler.toml plus Worker secrets.
//! Built from a lookup function so tests don't need the Cloudflare runtime.
// adapted from kabar-app main: crates/server/src/config.rs (sign-in only)

use crate::tokens::TokenKey;

#[derive(Clone, Debug)]
pub struct Config {
    /// Steps's own OAuth client ids whose Google ID tokens we accept (`aud`).
    pub google_client_ids: Vec<String>,
    /// Lower-cased emails allowed to sign in (Mohamed's).
    pub allowed_emails: Vec<String>,
    pub token_key: Option<TokenKey>,
    /// Origins of the web app (docs/WEB.md) whose pages may call the API.
    pub web_origins: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is not set")]
    Missing(&'static str),
    #[error("{0} is invalid: {1}")]
    Invalid(&'static str, String),
}

fn list(s: &str) -> Vec<String> {
    s.split([',', '\n', ' ']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

impl Config {
    /// `get` returns a var or secret by name; empty strings count as unset.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| get(name).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let token_key = match get("TOKEN_SIGNING_KEY") {
            None => None,
            Some(v) => {
                Some(TokenKey::from_base64(&v).map_err(|e| ConfigError::Invalid("TOKEN_SIGNING_KEY", e))?)
            }
        };
        Ok(Self {
            google_client_ids: get("GOOGLE_CLIENT_IDS").map(|v| list(&v)).unwrap_or_default(),
            allowed_emails: get("ALLOWED_EMAILS")
                .map(|v| list(&v).into_iter().map(|e| e.to_lowercase()).collect())
                .unwrap_or_default(),
            token_key,
            web_origins: get("WEB_ORIGINS")
                .map(|v| list(&v).into_iter().map(|o| o.trim_end_matches('/').to_owned()).collect())
                .unwrap_or_default(),
        })
    }

    /// The web app's pages get CORS answers; nobody else does.
    pub fn web_origin_allowed(&self, origin: &str) -> bool {
        self.web_origins.iter().any(|o| o == origin)
    }

    /// Sign-in needs all three.
    pub fn auth_ready(&self) -> Result<&TokenKey, ConfigError> {
        if self.google_client_ids.is_empty() {
            return Err(ConfigError::Missing("GOOGLE_CLIENT_IDS"));
        }
        if self.allowed_emails.is_empty() {
            return Err(ConfigError::Missing("ALLOWED_EMAILS"));
        }
        self.token_key.as_ref().ok_or(ConfigError::Missing("TOKEN_SIGNING_KEY"))
    }

    pub fn email_allowed(&self, email: &str) -> bool {
        let email = email.trim().to_lowercase();
        self.allowed_emails.contains(&email)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn only_listed_web_origins_are_allowed() {
        let c = cfg(&[("WEB_ORIGINS", "https://steps-web.x.workers.dev/, http://localhost:8080")]).unwrap();
        assert!(c.web_origin_allowed("https://steps-web.x.workers.dev"));
        assert!(c.web_origin_allowed("http://localhost:8080"));
        assert!(!c.web_origin_allowed("https://evil.example"));
        assert!(!cfg(&[]).unwrap().web_origin_allowed("https://steps-web.x.workers.dev"));
    }

    fn cfg(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn sign_in_reports_what_is_missing() {
        let c = cfg(&[]).unwrap();
        assert_eq!(c.auth_ready().unwrap_err(), ConfigError::Missing("GOOGLE_CLIENT_IDS"));
        let c = cfg(&[("GOOGLE_CLIENT_IDS", "a.apps.googleusercontent.com")]).unwrap();
        assert_eq!(c.auth_ready().unwrap_err(), ConfigError::Missing("ALLOWED_EMAILS"));
    }

    #[test]
    fn allowlist_is_case_insensitive_and_comma_separated() {
        let c = cfg(&[("ALLOWED_EMAILS", " Learner@Example.com, other@example.com ")]).unwrap();
        assert!(c.email_allowed("learner@example.com"));
        assert!(c.email_allowed("OTHER@example.com"));
        assert!(!c.email_allowed("someone@example.com"));
    }

    #[test]
    fn a_bad_signing_key_is_reported_and_never_printed() {
        assert!(matches!(
            cfg(&[("TOKEN_SIGNING_KEY", "short")]).unwrap_err(),
            ConfigError::Invalid("TOKEN_SIGNING_KEY", _)
        ));
        let seed = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
        let c = cfg(&[("TOKEN_SIGNING_KEY", seed)]).unwrap();
        assert!(!format!("{c:?}").contains(seed));
    }
}
