#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AuthType {
    NoAuth,
    BearerToken,
    BasicAuth,
    ApiKey,
    Digest,
}

impl AuthType {
    pub const ALL: [AuthType; 5] = [
        AuthType::NoAuth,
        AuthType::BearerToken,
        AuthType::BasicAuth,
        AuthType::ApiKey,
        AuthType::Digest,
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
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyLocation {
    Header,
    Query,
}

impl ApiKeyLocation {
    pub const ALL: [ApiKeyLocation; 2] = [ApiKeyLocation::Header, ApiKeyLocation::Query];
}

impl Default for ApiKeyLocation {
    fn default() -> Self {
        ApiKeyLocation::Header
    }
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
}

impl Auth {
    pub fn auth_type(&self) -> AuthType {
        match self {
            Auth::None => AuthType::NoAuth,
            Auth::BearerToken(_) => AuthType::BearerToken,
            Auth::Basic { .. } => AuthType::BasicAuth,
            Auth::ApiKey { .. } => AuthType::ApiKey,
            Auth::Digest { .. } => AuthType::Digest,
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
    fn auth_type_all_has_5_variants() {
        assert_eq!(AuthType::ALL.len(), 5);
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
    }
}
