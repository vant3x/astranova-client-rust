#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AuthType {
    NoAuth,
    BearerToken,
    BasicAuth,
    // ApiKey
}

impl AuthType {
    pub const ALL: [AuthType; 3] = [AuthType::NoAuth, AuthType::BearerToken, AuthType::BasicAuth];
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
            }
        )
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
    fn auth_type_all_has_3_variants() {
        assert_eq!(AuthType::ALL.len(), 3);
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
}
