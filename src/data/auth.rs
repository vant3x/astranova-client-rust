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

#[derive(Debug, Clone, Default)]
pub enum Auth {
    #[default]
    None,
    BearerToken(String),
    Basic {
        user: String,
        pass: String,
    },
}
