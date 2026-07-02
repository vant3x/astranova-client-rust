use super::auth::{ApiKeyLocation, OAuth2GrantType};

#[derive(Debug, Clone)]
pub enum AuthInput {
    BearerToken(String),
    BasicUser(String),
    BasicPass(String),
    ApiKeyKey(String),
    ApiKeyValue(String),
    ApiKeyLocation(ApiKeyLocation),
    DigestUser(String),
    DigestPass(String),
    OAuth2GrantType(OAuth2GrantType),
    OAuth2AuthUrl(String),
    OAuth2TokenUrl(String),
    OAuth2DeviceAuthUrl(String),
    OAuth2ClientId(String),
    OAuth2ClientSecret(String),
    OAuth2Scopes(String),
    OAuth2RedirectUri(String),
    OAuth2PkceEnabled(bool),
    OAuth2AccessToken(String),
    OAuth2RefreshToken(String),
}

use super::auth::Auth;

impl Auth {
    pub fn apply_input(&mut self, input: AuthInput) {
        match (self, input) {
            (Auth::BearerToken(token), AuthInput::BearerToken(new_token)) => {
                *token = new_token;
            }
            (Auth::Basic { user, .. }, AuthInput::BasicUser(new_user)) => {
                *user = new_user;
            }
            (Auth::Basic { pass, .. }, AuthInput::BasicPass(new_pass)) => {
                *pass = new_pass;
            }
            (Auth::ApiKey { key, .. }, AuthInput::ApiKeyKey(new_key)) => {
                *key = new_key;
            }
            (Auth::ApiKey { value, .. }, AuthInput::ApiKeyValue(new_value)) => {
                *value = new_value;
            }
            (Auth::ApiKey { location, .. }, AuthInput::ApiKeyLocation(new_location)) => {
                *location = new_location;
            }
            (Auth::Digest { user, .. }, AuthInput::DigestUser(new_user)) => {
                *user = new_user;
            }
            (Auth::Digest { pass, .. }, AuthInput::DigestPass(new_pass)) => {
                *pass = new_pass;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2GrantType(grant_type)) => {
                config.grant_type = grant_type;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2AuthUrl(url)) => {
                config.auth_url = url;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2TokenUrl(url)) => {
                config.token_url = url;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2ClientId(id)) => {
                config.client_id = id;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2ClientSecret(secret)) => {
                config.client_secret = secret;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2Scopes(scopes)) => {
                config.scopes = scopes;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2RedirectUri(uri)) => {
                config.redirect_uri = uri;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2PkceEnabled(enabled)) => {
                config.pkce_enabled = enabled;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2AccessToken(token)) => {
                config.access_token = token;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2RefreshToken(token)) => {
                config.refresh_token = token;
            }
            (Auth::OAuth2(config), AuthInput::OAuth2DeviceAuthUrl(url)) => {
                config.device_auth_url = url;
            }
            _ => {}
        }
    }
}
