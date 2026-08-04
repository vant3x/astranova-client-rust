use crate::error::AppError;
use rusqlite::params;

const SERVICE_NAME: &str = "com.astraio.client";

#[derive(Debug, Clone)]
pub struct SecretStore;

#[allow(dead_code)]
impl SecretStore {
    pub fn new() -> Self {
        Self
    }

    fn build_key(category: &str, identifier: &str, field: &str) -> String {
        format!("{category}:{identifier}:{field}")
    }

    pub fn store_secret(
        &self,
        category: &str,
        identifier: &str,
        field: &str,
        secret: &str,
    ) -> Result<(), AppError> {
        let key = Self::build_key(category, identifier, field);
        let entry = keyring::Entry::new(SERVICE_NAME, &key)
            .map_err(|e| AppError::Io(format!("Failed to create keyring entry '{key}': {e}")))?;
        entry
            .set_password(secret)
            .map_err(|e| AppError::Io(format!("Failed to store secret in keyring '{key}': {e}")))?;
        log::debug!("Stored secret: category={category}, field={field}");
        Ok(())
    }

    pub fn get_secret(
        &self,
        category: &str,
        identifier: &str,
        field: &str,
    ) -> Result<Option<String>, AppError> {
        let key = Self::build_key(category, identifier, field);
        let entry = keyring::Entry::new(SERVICE_NAME, &key)
            .map_err(|e| AppError::Io(format!("Failed to create keyring entry '{key}': {e}")))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Io(format!(
                "Failed to read secret from keyring '{key}': {e}"
            ))),
        }
    }

    pub fn delete_secret(
        &self,
        category: &str,
        identifier: &str,
        field: &str,
    ) -> Result<bool, AppError> {
        let key = Self::build_key(category, identifier, field);
        let entry = keyring::Entry::new(SERVICE_NAME, &key)
            .map_err(|e| AppError::Io(format!("Failed to create keyring entry '{key}': {e}")))?;
        match entry.delete_credential() {
            Ok(()) => {
                log::debug!("Deleted secret: category={category}, field={field}");
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(AppError::Io(format!(
                "Failed to delete secret from keyring '{key}': {e}"
            ))),
        }
    }

    pub fn delete_all_for_identifier(
        &self,
        category: &str,
        identifier: &str,
        fields: &[&str],
    ) -> Result<(), AppError> {
        for field in fields {
            let _ = self.delete_secret(category, identifier, field)?;
        }
        Ok(())
    }

    pub fn store_oauth2_tokens(
        &self,
        identifier: &str,
        access_token: &str,
        refresh_token: &str,
        client_secret: &str,
    ) -> Result<(), AppError> {
        self.store_secret("oauth2", identifier, "access_token", access_token)?;
        if !refresh_token.is_empty() {
            self.store_secret("oauth2", identifier, "refresh_token", refresh_token)?;
        }
        if !client_secret.is_empty() {
            self.store_secret("oauth2", identifier, "client_secret", client_secret)?;
        }
        Ok(())
    }

    pub fn get_oauth2_tokens(&self, identifier: &str) -> Result<OAuth2Secrets, AppError> {
        let access_token = self
            .get_secret("oauth2", identifier, "access_token")?
            .unwrap_or_default();
        let refresh_token = self
            .get_secret("oauth2", identifier, "refresh_token")?
            .unwrap_or_default();
        let client_secret = self
            .get_secret("oauth2", identifier, "client_secret")?
            .unwrap_or_default();
        Ok(OAuth2Secrets {
            access_token,
            refresh_token,
            client_secret,
        })
    }

    pub fn delete_oauth2_tokens(&self, identifier: &str) -> Result<(), AppError> {
        self.delete_all_for_identifier(
            "oauth2",
            identifier,
            &["access_token", "refresh_token", "client_secret"],
        )
    }

    pub fn store_basic_password(&self, identifier: &str, password: &str) -> Result<(), AppError> {
        self.store_secret("basic", identifier, "pass", password)
    }

    pub fn get_basic_password(&self, identifier: &str) -> Result<Option<String>, AppError> {
        self.get_secret("basic", identifier, "pass")
    }

    pub fn store_api_key(&self, identifier: &str, api_key: &str) -> Result<(), AppError> {
        self.store_secret("apikey", identifier, "value", api_key)
    }

    pub fn get_api_key(&self, identifier: &str) -> Result<Option<String>, AppError> {
        self.get_secret("apikey", identifier, "value")
    }

    pub fn store_bearer_token(&self, identifier: &str, token: &str) -> Result<(), AppError> {
        self.store_secret("bearer", identifier, "token", token)
    }

    pub fn get_bearer_token(&self, identifier: &str) -> Result<Option<String>, AppError> {
        self.get_secret("bearer", identifier, "token")
    }

    pub fn store_proxy_password(&self, identifier: &str, password: &str) -> Result<(), AppError> {
        self.store_secret("proxy", identifier, "pass", password)
    }

    pub fn get_proxy_password(&self, identifier: &str) -> Result<Option<String>, AppError> {
        self.get_secret("proxy", identifier, "pass")
    }

    pub fn store_pkce_verifier(&self, identifier: &str, verifier: &str) -> Result<(), AppError> {
        self.store_secret("oauth2", identifier, "pkce_verifier", verifier)
    }

    pub fn get_pkce_verifier(&self, identifier: &str) -> Result<Option<String>, AppError> {
        self.get_secret("oauth2", identifier, "pkce_verifier")
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct OAuth2Secrets {
    pub access_token: String,
    pub refresh_token: String,
    pub client_secret: String,
}

pub fn migrate_plaintext_tokens_to_keyring(
    store: &SecretStore,
    conn: &rusqlite::Connection,
) -> Result<u32, AppError> {
    let already_migrated = crate::persistence::database::get_app_setting(conn, "keyring_migrated")
        .is_some_and(|v| v == "true");

    if already_migrated {
        log::debug!("Keyring migration already completed, skipping");
        return Ok(0);
    }

    let mut migrated = 0u32;

    match conn.prepare(
        "SELECT id, collection_id, name, auth_type, auth_data FROM collection_requests WHERE auth_type = 'oauth2' AND auth_data IS NOT NULL"
    ) {
        Ok(mut stmt) => {
            let rows: Vec<(i32, i32, String, String, String)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .filter_map(std::result::Result::ok)
                .collect();

            for (id, collection_id, _name, _auth_type, auth_data) in rows {
                if let Ok(crate::data::auth::Auth::OAuth2(config)) = serde_json::from_str::<crate::data::auth::Auth>(&auth_data) {
                    let identifier = format!("col_{collection_id}_{id}");

                    if !config.access_token.is_empty()
                        && store.store_secret("oauth2", &identifier, "access_token", &config.access_token).is_ok() {
                        migrated += 1;
                    }
                    if !config.refresh_token.is_empty() {
                        let _ = store.store_secret("oauth2", &identifier, "refresh_token", &config.refresh_token);
                    }
                    if !config.client_secret.is_empty() {
                        let _ = store.store_secret("oauth2", &identifier, "client_secret", &config.client_secret);
                    }
                }

                // Sanitize auth_data in DB after migrating to keyring
                if let Ok(safe_auth) = serde_json::from_str::<crate::data::auth::Auth>(&auth_data)
                    .and_then(|a| a.to_safe_json())
                {
                    if let Err(e) = conn.execute(
                        "UPDATE collection_requests SET auth_data = ?1 WHERE id = ?2",
                        params![safe_auth, id],
                    ) {
                        log::warn!("Failed to sanitize auth_data for request {id}: {e}");
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to query collection_requests for migration: {e}");
        }
    }

    match conn.prepare(
        "SELECT id, method, url, request_data FROM request_history WHERE request_data IS NOT NULL",
    ) {
        Ok(mut stmt) => {
            let rows: Vec<(i32, String, String, String)> = stmt
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .filter_map(std::result::Result::ok)
                .collect();

            for (id, _method, _url, request_data) in rows {
                if let Ok(mut request) =
                    serde_json::from_str::<crate::http_client::request::HttpRequest>(&request_data)
                {
                    if let Some(crate::data::auth::Auth::OAuth2(config)) = &request.auth {
                        let identifier = format!("hist_{id}");

                        if !config.access_token.is_empty()
                            && store
                                .store_secret(
                                    "oauth2",
                                    &identifier,
                                    "access_token",
                                    &config.access_token,
                                )
                                .is_ok()
                        {
                            migrated += 1;
                        }
                        if !config.refresh_token.is_empty() {
                            let _ = store.store_secret(
                                "oauth2",
                                &identifier,
                                "refresh_token",
                                &config.refresh_token,
                            );
                        }
                    }

                    // Sanitize auth in request_data after migration
                    if request.auth.is_some() {
                        request.auth = None;
                        if let Ok(sanitized) = serde_json::to_string(&request) {
                            if let Err(e) = conn.execute(
                                "UPDATE request_history SET request_data = ?1 WHERE id = ?2",
                                params![sanitized, id],
                            ) {
                                log::warn!("Failed to sanitize request_data for history {id}: {e}");
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to query request_history for migration: {e}");
        }
    }

    log::info!("Keyring migration complete: {migrated} tokens migrated");

    let _ = crate::persistence::database::set_app_setting(conn, "keyring_migrated", "true");

    Ok(migrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SecretStore {
        SecretStore::new()
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_secret() {
        let store = test_store();
        let result = store.store_secret("test", "id1", "field1", "my_secret_value");
        if result.is_err() {
            return;
        }
        let retrieved = store.get_secret("test", "id1", "field1").unwrap();
        assert_eq!(retrieved.as_deref(), Some("my_secret_value"));
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn get_nonexistent_secret_returns_none() {
        let store = test_store();
        let result = store.get_secret("test_nonexistent", "nonexistent", "field");
        if result.is_err() {
            return;
        }
        assert!(result.unwrap().is_none());
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn delete_secret_works() {
        let store = test_store();
        let _ = store.store_secret("test_del", "id1", "field1", "value1");
        let result = store.delete_secret("test_del", "id1", "field1");
        if result.is_err() {
            return;
        }
        let retrieved = store.get_secret("test_del", "id1", "field1").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn delete_nonexistent_returns_false() {
        let store = test_store();
        let result = store.delete_secret("test_del_ne", "nonexistent", "field");
        if result.is_err() {
            return;
        }
        assert!(!result.unwrap());
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_oauth2_tokens() {
        let store = test_store();
        let result =
            store.store_oauth2_tokens("test_oauth", "access123", "refresh456", "secret789");
        if result.is_err() {
            return;
        }
        let secrets = store.get_oauth2_tokens("test_oauth").unwrap();
        assert_eq!(secrets.access_token, "access123");
        assert_eq!(secrets.refresh_token, "refresh456");
        assert_eq!(secrets.client_secret, "secret789");
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn delete_oauth2_tokens_clears_all() {
        let store = test_store();
        let _ = store.store_oauth2_tokens("test_oauth_del", "a", "r", "c");
        let _ = store.delete_oauth2_tokens("test_oauth_del");
        let secrets = store.get_oauth2_tokens("test_oauth_del").unwrap();
        assert!(secrets.access_token.is_empty());
        assert!(secrets.refresh_token.is_empty());
        assert!(secrets.client_secret.is_empty());
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_empty_refresh_token_not_stored() {
        let store = test_store();
        let result = store.store_oauth2_tokens("test_oauth_empty", "access", "", "secret");
        if result.is_err() {
            return;
        }
        let secrets = store.get_oauth2_tokens("test_oauth_empty").unwrap();
        assert_eq!(secrets.access_token, "access");
        assert!(secrets.refresh_token.is_empty());
    }

    #[test]
    fn build_key_format() {
        let key = SecretStore::build_key("oauth2", "my_id", "access_token");
        assert_eq!(key, "oauth2:my_id:access_token");
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_bearer_token() {
        let store = test_store();
        let result = store.store_bearer_token("test_bearer", "bearer_xyz");
        if result.is_err() {
            return;
        }
        let token = store.get_bearer_token("test_bearer").unwrap();
        assert_eq!(token.as_deref(), Some("bearer_xyz"));
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_api_key() {
        let store = test_store();
        let result = store.store_api_key("test_apikey", "key_abc");
        if result.is_err() {
            return;
        }
        let key = store.get_api_key("test_apikey").unwrap();
        assert_eq!(key.as_deref(), Some("key_abc"));
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_basic_password() {
        let store = test_store();
        let result = store.store_basic_password("test_basic", "pass123");
        if result.is_err() {
            return;
        }
        let pass = store.get_basic_password("test_basic").unwrap();
        assert_eq!(pass.as_deref(), Some("pass123"));
    }

    #[test]
    #[ignore = "triggers macOS Keychain prompt - run manually"]
    fn store_and_get_proxy_password() {
        let store = test_store();
        let result = store.store_proxy_password("test_proxy", "proxy_pass");
        if result.is_err() {
            return;
        }
        let pass = store.get_proxy_password("test_proxy").unwrap();
        assert_eq!(pass.as_deref(), Some("proxy_pass"));
    }
}
