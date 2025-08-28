#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
