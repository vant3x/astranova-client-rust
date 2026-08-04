use std::collections::HashMap;

fn parse_cookie_expiry(s: &str) -> Option<i64> {
    // Try RFC 3339 / ISO 8601
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
        .or_else(|| {
            chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%:z")
                .ok()
                .map(|dt| dt.timestamp())
        })
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|dt| dt.and_utc().timestamp())
        })
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| dt.and_utc().timestamp())
        })
        .or_else(|| {
            // Manual parse for RFC 1123: "Thu, 01 Jan 2020 00:00:00 GMT"
            let s = s.trim();
            // Skip day-of-week and comma if present
            let s = if let Some(idx) = s.find(',') {
                s[idx + 1..].trim()
            } else {
                s
            };
            // Parse "DD Mon YYYY HH:MM:SS" and optional timezone
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() < 4 {
                return None;
            }
            let day: u32 = parts[0].parse().ok()?;
            let month = match parts[1] {
                "Jan" => 1u32,
                "Feb" => 2,
                "Mar" => 3,
                "Apr" => 4,
                "May" => 5,
                "Jun" => 6,
                "Jul" => 7,
                "Aug" => 8,
                "Sep" => 9,
                "Oct" => 10,
                "Nov" => 11,
                "Dec" => 12,
                _ => return None,
            };
            let year: i32 = parts[2].parse().ok()?;
            let time_parts: Vec<&str> = parts[3].split(':').collect();
            if time_parts.len() != 3 {
                return None;
            }
            let hour: u32 = time_parts[0].parse().ok()?;
            let min: u32 = time_parts[1].parse().ok()?;
            let sec: u32 = time_parts[2].parse().ok()?;
            let naive =
                chrono::NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, min, sec)?;
            Some(naive.and_utc().timestamp())
        })
}

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    #[allow(dead_code)]
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    #[default]
    Lax,
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "Strict"),
            Self::Lax => write!(f, "Lax"),
            Self::None => write!(f, "None"),
        }
    }
}

impl std::fmt::Display for Cookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}={}; domain={}; path={}",
            self.name, self.value, self.domain, self.path
        )?;
        if self.secure {
            write!(f, "; Secure")?;
        }
        if self.http_only {
            write!(f, "; HttpOnly")?;
        }
        write!(f, "; SameSite={}", self.same_site)
    }
}

impl Cookie {
    pub fn is_expired(&self, now_timestamp: i64) -> bool {
        match &self.expires {
            Some(expires_str) => {
                if let Some(ts) = parse_cookie_expiry(expires_str) {
                    ts <= now_timestamp
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn path_matches(&self, request_path: &str) -> bool {
        if self.path == "/" {
            return true;
        }
        if request_path == self.path {
            return true;
        }
        if request_path.starts_with(&self.path) {
            if self.path.ends_with('/') {
                return true;
            }
            let next_char = request_path.as_bytes().get(self.path.len());
            return next_char == Some(&b'/') || next_char.is_none();
        }
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    cookies: HashMap<String, Vec<Cookie>>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_from_set_cookie(&mut self, set_cookie_header: &str, request_url: &str) {
        let parts: Vec<&str> = set_cookie_header.split(';').collect();
        if parts.is_empty() {
            return;
        }

        let name_value = parts[0].trim();
        let (name, value) = match name_value.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
            None => (name_value.to_string(), String::new()),
        };

        if name.is_empty() {
            return;
        }

        let mut domain = String::new();
        let mut path = "/".to_string();
        let mut secure = false;
        let mut http_only = false;
        let mut same_site = SameSite::default();
        let mut expires = None;

        for part in &parts[1..] {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower == "httponly" {
                http_only = true;
            } else if lower == "secure" {
                secure = true;
            } else if let Some(val) = lower.strip_prefix("samesite=") {
                same_site = match val {
                    "strict" => SameSite::Strict,
                    "none" => SameSite::None,
                    _ => SameSite::Lax,
                };
            } else if let Some(val) = lower.strip_prefix("domain=") {
                domain = val.trim().to_string();
            } else if let Some(val) = lower.strip_prefix("path=") {
                path = val.trim().to_string();
            } else if let Some(val) = lower.strip_prefix("expires=") {
                expires = Some(val.trim().to_string());
            }
        }

        if domain.is_empty() {
            if let Ok(url) = reqwest::Url::parse(request_url) {
                domain = url.host_str().unwrap_or("").to_string();
            }
        }

        let cookie = Cookie {
            name,
            value,
            domain,
            path,
            secure,
            http_only,
            same_site,
            expires,
        };

        let domain_key = cookie.domain.clone();
        let entry = self.cookies.entry(domain_key).or_default();

        entry.retain(|c| !(c.name == cookie.name && c.path == cookie.path));
        entry.push(cookie);
    }

    pub fn get_cookies_for_url(&self, url: &str) -> Vec<&Cookie> {
        let parsed = match reqwest::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return Vec::new(),
        };

        let host = parsed.host_str().unwrap_or("");
        let is_secure = parsed.scheme() == "https";
        let request_path = parsed.path();
        let now = chrono::Utc::now().timestamp();

        let mut result = Vec::new();

        for (domain, cookies) in &self.cookies {
            if Self::domain_matches(host, domain) {
                for cookie in cookies {
                    if cookie.is_expired(now) {
                        continue;
                    }
                    if cookie.path_matches(request_path) && (!cookie.secure || is_secure) {
                        result.push(cookie);
                    }
                }
            }
        }

        result
    }

    pub fn to_cookie_header(&self, url: &str) -> Option<String> {
        let cookies = self.get_cookies_for_url(url);
        if cookies.is_empty() {
            return None;
        }

        let header = cookies
            .iter()
            .map(|c| {
                let escaped_value = c.value.replace('\\', "\\\\").replace('"', "\\\"");
                format!("{}=\"{}\"", c.name, escaped_value)
            })
            .collect::<Vec<_>>()
            .join("; ");

        Some(header)
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    #[allow(dead_code)]
    pub fn remove_expired(&mut self) {
        let now = chrono::Utc::now().timestamp();
        for cookies in self.cookies.values_mut() {
            cookies.retain(|c| !c.is_expired(now));
        }
        self.cookies.retain(|_, v| !v.is_empty());
    }

    #[allow(dead_code)]
    pub fn clear_domain(&mut self, domain: &str) {
        self.cookies.remove(domain);
    }

    pub fn domain_count(&self) -> usize {
        self.cookies.len()
    }

    pub fn total_count(&self) -> usize {
        self.cookies.values().map(std::vec::Vec::len).sum()
    }

    #[allow(dead_code)]
    pub fn domains(&self) -> Vec<(&str, usize)> {
        self.cookies
            .iter()
            .map(|(d, c)| (d.as_str(), c.len()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn cookies_for_domain(&self, domain: &str) -> Vec<&Cookie> {
        if let Some(v) = self.cookies.get(domain) {
            return v.iter().collect();
        }
        let dotted = format!(".{domain}");
        if let Some(v) = self.cookies.get(&dotted) {
            return v.iter().collect();
        }
        if let Some(stripped) = domain.strip_prefix('.') {
            if let Some(v) = self.cookies.get(stripped) {
                return v.iter().collect();
            }
        }
        Vec::new()
    }

    pub fn cookies_for_domain_mut(&mut self, domain: &str) -> Option<&mut Vec<Cookie>> {
        if self.cookies.contains_key(domain) {
            return self.cookies.get_mut(domain);
        }
        let dotted = format!(".{domain}");
        if self.cookies.contains_key(&dotted) {
            return self.cookies.get_mut(&dotted);
        }
        None
    }

    fn domain_matches(host: &str, cookie_domain: &str) -> bool {
        let cd = cookie_domain.strip_prefix('.').unwrap_or(cookie_domain);
        let h = host.strip_prefix('.').unwrap_or(host);
        h == cd || h.ends_with(&format!(".{cd}"))
    }

    pub fn remove_cookie(&mut self, domain: &str, name: &str, path: &str) -> bool {
        if let Some(cookies) = self.cookies.get_mut(domain) {
            let len_before = cookies.len();
            cookies.retain(|c| !(c.name == name && c.path == path));
            let removed = cookies.len() < len_before;
            if cookies.is_empty() {
                let _ = self.cookies.remove(domain);
            }
            return removed;
        }
        false
    }

    pub fn all_cookies(&self) -> Vec<&Cookie> {
        self.cookies.values().flat_map(|v| v.iter()).collect()
    }

    pub fn to_netscape(&self) -> String {
        let mut lines = vec![
            "# Netscape HTTP Cookie File".to_string(),
            "# https://curl.se/docs/http-cookies.html".to_string(),
            String::new(),
        ];
        for cookie in self.all_cookies() {
            let domain = if cookie.domain.starts_with('.') {
                cookie.domain.clone()
            } else {
                format!(".{}", cookie.domain)
            };
            let include_subdomains = if cookie.domain.starts_with('.') {
                "TRUE"
            } else {
                "FALSE"
            };
            let secure = if cookie.secure { "TRUE" } else { "FALSE" };
            let expires = cookie
                .expires
                .as_ref()
                .and_then(|e| parse_cookie_expiry(e))
                .unwrap_or(0);
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                domain, include_subdomains, cookie.path, secure, expires, cookie.name
            ));
            lines.push(cookie.value.clone());
        }
        lines.join("\n")
    }

    pub fn from_netscape(content: &str) -> Result<CookieJar, String> {
        let mut jar = CookieJar::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 7 {
                i += 1;
                continue;
            }
            let domain = parts[0].to_string();
            let path = parts[2].to_string();
            let secure = parts[3].eq_ignore_ascii_case("TRUE");
            let name = parts[5].to_string();
            let value = if i + 1 < lines.len() {
                let v = lines[i + 1].trim().to_string();
                i += 2;
                v
            } else {
                i += 1;
                String::new()
            };
            jar.insert(Cookie {
                name,
                value,
                domain,
                path,
                secure,
                http_only: false,
                same_site: SameSite::Lax,
                expires: None,
            });
        }
        Ok(jar)
    }

    #[allow(dead_code)]
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        let cookies: Vec<serde_json::Value> = self
            .all_cookies()
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "value": c.value,
                    "domain": c.domain,
                    "path": c.path,
                    "secure": c.secure,
                    "httpOnly": c.http_only,
                    "sameSite": c.same_site.to_string(),
                    "expires": c.expires,
                })
            })
            .collect();
        serde_json::to_string_pretty(&cookies)
    }

    pub fn from_json(content: &str) -> Result<CookieJar, String> {
        let cookies: Vec<serde_json::Value> =
            serde_json::from_str(content).map_err(|e| e.to_string())?;
        let mut jar = CookieJar::new();
        for v in cookies {
            let name = v["name"].as_str().unwrap_or("").to_string();
            let value = v["value"].as_str().unwrap_or("").to_string();
            let domain = v["domain"].as_str().unwrap_or("").to_string();
            let path = v["path"].as_str().unwrap_or("/").to_string();
            let secure = v["secure"].as_bool().unwrap_or(false);
            let http_only = v["httpOnly"].as_bool().unwrap_or(false);
            let same_site = match v["sameSite"].as_str().unwrap_or("Lax") {
                "Strict" => SameSite::Strict,
                "None" => SameSite::None,
                _ => SameSite::Lax,
            };
            let expires = v["expires"].as_str().map(std::string::ToString::to_string);
            if name.is_empty() || domain.is_empty() {
                continue;
            }
            jar.insert(Cookie {
                name,
                value,
                domain,
                path,
                secure,
                http_only,
                same_site,
                expires,
            });
        }
        Ok(jar)
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, cookie: Cookie) {
        let domain_key = cookie.domain.clone();
        let entry = self.cookies.entry(domain_key).or_default();
        entry.retain(|c| !(c.name == cookie.name && c.path == cookie.path));
        entry.push(cookie);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_set_cookie() {
        let mut jar = CookieJar::new();
        jar.insert_from_set_cookie("session=abc123", "https://api.example.com/data");
        assert_eq!(jar.total_count(), 1);
        let cookie = &jar.cookies_for_domain("api.example.com")[0];
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.domain, "api.example.com");
        assert_eq!(cookie.path, "/");
    }

    #[test]
    fn parse_set_cookie_with_attributes() {
        let mut jar = CookieJar::new();
        jar.insert_from_set_cookie(
            "token=xyz; Path=/api; Domain=.example.com; Secure; HttpOnly; SameSite=Strict",
            "https://example.com/test",
        );
        assert_eq!(jar.total_count(), 1);
        let cookie = &jar.cookies_for_domain("example.com")[0];
        assert_eq!(cookie.name, "token");
        assert_eq!(cookie.value, "xyz");
        assert_eq!(cookie.path, "/api");
        assert!(cookie.secure);
        assert!(cookie.http_only);
        assert_eq!(cookie.same_site, SameSite::Strict);
    }

    #[test]
    fn parse_set_cookie_domain_defaults_to_host() {
        let mut jar = CookieJar::new();
        jar.insert_from_set_cookie("id=123; Path=/", "https://myhost.com/page");
        assert_eq!(jar.total_count(), 1);
        let cookie = &jar.cookies_for_domain("myhost.com")[0];
        assert_eq!(cookie.domain, "myhost.com");
    }

    #[test]
    fn get_cookies_for_url_matches_domain() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let cookies = jar.get_cookies_for_url("https://example.com/page");
        assert_eq!(cookies.len(), 1);
    }

    #[test]
    fn get_cookies_for_url_matches_subdomain() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let cookies = jar.get_cookies_for_url("https://api.example.com/data");
        assert_eq!(cookies.len(), 1);
    }

    #[test]
    fn domain_matches_exact() {
        assert!(CookieJar::domain_matches("example.com", "example.com"));
    }

    #[test]
    fn domain_matches_subdomain() {
        assert!(CookieJar::domain_matches("api.example.com", ".example.com"));
    }

    #[test]
    fn domain_matches_no_dot_prefix() {
        assert!(CookieJar::domain_matches("api.example.com", "example.com"));
    }

    #[test]
    fn domain_matches_not_related() {
        assert!(!CookieJar::domain_matches("evil.com", "example.com"));
    }

    #[test]
    fn domain_matches_not_substring() {
        assert!(!CookieJar::domain_matches("notexample.com", "example.com"));
    }

    #[test]
    fn expired_cookie_is_filtered() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "old".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: Some("Thu, 01 Jan 2020 00:00:00 GMT".to_string()),
        });
        jar.insert(Cookie {
            name: "fresh".to_string(),
            value: "2".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: Some("Thu, 01 Jan 2099 00:00:00 GMT".to_string()),
        });
        let cookies = jar.get_cookies_for_url("https://example.com/");
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "fresh");
    }

    #[test]
    fn remove_expired_cleans_jar() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "old".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: Some("Thu, 01 Jan 2020 00:00:00 GMT".to_string()),
        });
        jar.insert(Cookie {
            name: "fresh".to_string(),
            value: "2".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.remove_expired();
        assert_eq!(jar.total_count(), 1);
        assert_eq!(jar.cookies_for_domain("example.com")[0].name, "fresh");
    }

    #[test]
    fn to_cookie_header_escapes_values() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "token".to_string(),
            value: r#"val"ue"#.to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let header = jar.to_cookie_header("https://example.com/").unwrap();
        assert!(header.contains(r#"token="val\"ue""#));
    }

    #[test]
    fn get_cookies_skips_secure_on_http() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let cookies = jar.get_cookies_for_url("http://example.com/page");
        assert!(cookies.is_empty());
    }

    #[test]
    fn get_cookies_includes_secure_on_https() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let cookies = jar.get_cookies_for_url("https://example.com/page");
        assert_eq!(cookies.len(), 1);
    }

    #[test]
    fn get_cookies_path_match() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/api".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        assert_eq!(
            jar.get_cookies_for_url("https://example.com/api/data")
                .len(),
            1
        );
        assert!(jar
            .get_cookies_for_url("https://example.com/other")
            .is_empty());
    }

    #[test]
    fn to_cookie_header_format() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.insert(Cookie {
            name: "b".to_string(),
            value: "2".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        let header = jar.to_cookie_header("https://example.com/page").unwrap();
        assert!(header.contains("a=\"1\""));
        assert!(header.contains("b=\"2\""));
        assert!(header.contains("; "));
    }

    #[test]
    fn insert_replaces_same_name_and_path() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "sid".to_string(),
            value: "old".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.insert(Cookie {
            name: "sid".to_string(),
            value: "new".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        assert_eq!(jar.total_count(), 1);
        assert_eq!(jar.cookies_for_domain("example.com")[0].value, "new");
    }

    #[test]
    fn clear_removes_all() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.clear();
        assert_eq!(jar.total_count(), 0);
    }

    #[test]
    fn clear_domain_removes_only_that_domain() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "a.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.insert(Cookie {
            name: "b".to_string(),
            value: "2".to_string(),
            domain: "b.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.clear_domain("a.com");
        assert_eq!(jar.total_count(), 1);
        assert!(jar.cookies_for_domain("a.com").is_empty());
    }

    #[test]
    fn domain_count_and_total_count() {
        let mut jar = CookieJar::new();
        jar.insert(Cookie {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "a.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.insert(Cookie {
            name: "b".to_string(),
            value: "2".to_string(),
            domain: "a.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        jar.insert(Cookie {
            name: "c".to_string(),
            value: "3".to_string(),
            domain: "b.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
        });
        assert_eq!(jar.domain_count(), 2);
        assert_eq!(jar.total_count(), 3);
    }

    #[test]
    fn same_site_display() {
        assert_eq!(SameSite::Strict.to_string(), "Strict");
        assert_eq!(SameSite::Lax.to_string(), "Lax");
        assert_eq!(SameSite::None.to_string(), "None");
    }

    #[test]
    fn cookie_display() {
        let c = Cookie {
            name: "sid".to_string(),
            value: "abc".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSite::Strict,
            expires: None,
        };
        let s = c.to_string();
        assert!(s.contains("sid=abc"));
        assert!(s.contains("example.com"));
        assert!(s.contains("Secure"));
        assert!(s.contains("HttpOnly"));
    }
}
