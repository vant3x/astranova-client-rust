use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration: Duration,
    pub size: u64,
}
