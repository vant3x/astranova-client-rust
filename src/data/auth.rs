#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AuthType {
    NoAuth,
    BearerToken,
    BasicAuth,
    ApiKey,
    Digest,
    OAuth2,
}

impl AuthType {
    pub const ALL: [AuthType; 6] = [
        AuthType::NoAuth,
        AuthType::BearerToken,
        AuthType::BasicAuth,
        AuthType::ApiKey,
        AuthType::Digest,
        AuthType::OAuth2,
    ];
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AuthType::NoAuth => "No Auth",
                AuthType::BearerToken => "Bearer Token",
                AuthType::BasicAuth => "Basic Auth",
                AuthType::ApiKey => "API Key",
                AuthType::Digest => "Digest Auth",
                AuthType::OAuth2 => "OAuth 2.0",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
}

impl ApiKeyLocation {
    pub const ALL: [ApiKeyLocation; 2] = [ApiKeyLocation::Header, ApiKeyLocation::Query];
}

impl std::fmt::Display for ApiKeyLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeyLocation::Header => write!(f, "Header"),
            ApiKeyLocation::Query => write!(f, "Query Parameter"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Auth {
    #[default]
    None,
    BearerToken(String),
    Basic {
        user: String,
        pass: String,
    },
    ApiKey {
        key: String,
        value: String,
        location: ApiKeyLocation,
    },
    Digest {
        user: String,
        pass: String,
    },
    OAuth2(OAuth2Config),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OAuth2Config {
    pub grant_type: OAuth2GrantType,
    pub auth_url: String,
    pub token_url: String,
    pub device_auth_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub redirect_uri: String,
    pub pkce_enabled: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: Option<String>,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub device_code_expires_in: Option<u64>,
    pub device_code_interval: Option<u64>,
    pub status: OAuth2Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum OAuth2Status {
    #[default]
    Idle,
    Loading,
    Success(String),
    Error(String),
    AwaitingAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OAuth2GrantType {
    #[default]
    AuthorizationCode,
    ClientCredentials,
    Implicit,
    DeviceCode,
}

impl OAuth2GrantType {
    pub const ALL: [OAuth2GrantType; 4] = [
        OAuth2GrantType::AuthorizationCode,
        OAuth2GrantType::ClientCredentials,
        OAuth2GrantType::Implicit,
        OAuth2GrantType::DeviceCode,
    ];
}

impl std::fmt::Display for OAuth2GrantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2GrantType::AuthorizationCode => write!(f, "Authorization Code"),
            OAuth2GrantType::ClientCredentials => write!(f, "Client Credentials"),
            OAuth2GrantType::Implicit => write!(f, "Implicit"),
            OAuth2GrantType::DeviceCode => write!(f, "Device Code"),
        }
    }
}

impl std::fmt::Display for OAuth2Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2Status::Idle => write!(f, ""),
            OAuth2Status::Loading => write!(f, "Loading..."),
            OAuth2Status::Success(msg) => write!(f, "{}", msg),
            OAuth2Status::Error(msg) => write!(f, "Error: {}", msg),
            OAuth2Status::AwaitingAuthorization => write!(f, "Awaiting authorization..."),
        }
    }
}

impl Auth {
    pub fn auth_type(&self) -> AuthType {
        match self {
            Auth::None => AuthType::NoAuth,
            Auth::BearerToken(_) => AuthType::BearerToken,
            Auth::Basic { .. } => AuthType::BasicAuth,
            Auth::ApiKey { .. } => AuthType::ApiKey,
            Auth::Digest { .. } => AuthType::Digest,
            Auth::OAuth2(_) => AuthType::OAuth2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_type_display_no_auth() {
        assert_eq!(AuthType::NoAuth.to_string(), "No Auth");
    }

    #[test]
    fn auth_type_display_bearer() {
        assert_eq!(AuthType::BearerToken.to_string(), "Bearer Token");
    }

    #[test]
    fn auth_type_display_basic() {
        assert_eq!(AuthType::BasicAuth.to_string(), "Basic Auth");
    }

    #[test]
    fn auth_type_display_api_key() {
        assert_eq!(AuthType::ApiKey.to_string(), "API Key");
    }

    #[test]
    fn auth_type_display_digest() {
        assert_eq!(AuthType::Digest.to_string(), "Digest Auth");
    }

    #[test]
    fn auth_type_display_oauth2() {
        assert_eq!(AuthType::OAuth2.to_string(), "OAuth 2.0");
    }

    #[test]
    fn auth_type_all_has_6_variants() {
        assert_eq!(AuthType::ALL.len(), 6);
    }

    #[test]
    fn auth_default_is_none() {
        assert!(matches!(Auth::default(), Auth::None));
    }

    #[test]
    fn auth_bearer_stores_token() {
        let auth = Auth::BearerToken("my-token".to_string());
        match auth {
            Auth::BearerToken(t) => assert_eq!(t, "my-token"),
            _ => panic!("Expected BearerToken"),
        }
    }

    #[test]
    fn auth_basic_stores_credentials() {
        let auth = Auth::Basic {
            user: "admin".to_string(),
            pass: "secret".to_string(),
        };
        match auth {
            Auth::Basic { user, pass } => {
                assert_eq!(user, "admin");
                assert_eq!(pass, "secret");
            }
            _ => panic!("Expected Basic"),
        }
    }

    #[test]
    fn auth_api_key_stores_key_value_and_location() {
        let auth = Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "abc123".to_string(),
            location: ApiKeyLocation::Header,
        };
        match &auth {
            Auth::ApiKey {
                key,
                value,
                location,
            } => {
                assert_eq!(key, "X-API-Key");
                assert_eq!(value, "abc123");
                assert_eq!(*location, ApiKeyLocation::Header);
            }
            _ => panic!("Expected ApiKey"),
        }
        assert_eq!(auth.auth_type(), AuthType::ApiKey);
    }

    #[test]
    fn auth_api_key_query_location() {
        let auth = Auth::ApiKey {
            key: "api_key".to_string(),
            value: "secret123".to_string(),
            location: ApiKeyLocation::Query,
        };
        match &auth {
            Auth::ApiKey { location, .. } => {
                assert_eq!(*location, ApiKeyLocation::Query);
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn auth_digest_stores_credentials() {
        let auth = Auth::Digest {
            user: "admin".to_string(),
            pass: "secret".to_string(),
        };
        match &auth {
            Auth::Digest { user, pass } => {
                assert_eq!(user, "admin");
                assert_eq!(pass, "secret");
            }
            _ => panic!("Expected Digest"),
        }
        assert_eq!(auth.auth_type(), AuthType::Digest);
    }

    #[test]
    fn auth_type_returns_correct_variant() {
        assert_eq!(Auth::None.auth_type(), AuthType::NoAuth);
        assert_eq!(
            Auth::BearerToken("t".into()).auth_type(),
            AuthType::BearerToken
        );
        assert_eq!(
            Auth::Basic {
                user: "u".into(),
                pass: "p".into()
            }
            .auth_type(),
            AuthType::BasicAuth
        );
        assert_eq!(
            Auth::ApiKey {
                key: "k".into(),
                value: "v".into(),
                location: ApiKeyLocation::Header
            }
            .auth_type(),
            AuthType::ApiKey
        );
        assert_eq!(
            Auth::Digest {
                user: "u".into(),
                pass: "p".into()
            }
            .auth_type(),
            AuthType::Digest
        );
        assert_eq!(
            Auth::OAuth2(OAuth2Config::default()).auth_type(),
            AuthType::OAuth2
        );
    }
}
