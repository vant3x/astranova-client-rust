use crate::error::AppError;
use crate::http_client::config::RequestConfig;
use crate::http_client::request::{HttpMethod, HttpRequest};
use crate::persistence::database::{self, CollectionRequest, Environment};
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "astraio-cli")]
#[command(about = "Astraio CLI - HTTP client for automation and testing")]
#[command(version)]
pub struct Cli {
    /// Path to Astraio database file
    #[arg(short, long, default_value = "~/.astraio/data.db")]
    pub database: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Environment name to use
    #[arg(short, long)]
    pub environment: Option<String>,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,

    /// Allow insecure TLS connections (skip certificate verification)
    #[arg(long)]
    pub insecure: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a single request by ID
    Run {
        /// Request ID from collection
        #[arg(short, long)]
        id: i32,

        /// Override URL
        #[arg(short, long)]
        url: Option<String>,

        /// Override method (GET, POST, etc.)
        #[arg(short, long)]
        method: Option<String>,

        /// Request body
        #[arg(short, long)]
        body: Option<String>,

        /// Custom headers (key:value)
        #[arg(short, long, value_delimiter = ',')]
        headers: Option<Vec<String>>,
    },

    /// Run all requests in a collection
    Collection {
        /// Collection ID or name
        #[arg(short, long)]
        id: Option<i32>,

        /// Collection name (if ID not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Stop on first failure
        #[arg(short, long)]
        stop_on_failure: bool,

        /// Delay between requests in milliseconds
        #[arg(short, long, default_value = "0")]
        delay: u64,
    },

    /// Run a request from raw parameters
    Quick {
        /// URL to request
        url: String,

        /// HTTP method
        #[arg(short, long, default_value = "GET")]
        method: String,

        /// Request body
        #[arg(short, long)]
        body: Option<String>,

        /// Custom headers (key:value)
        #[arg(short, long, value_delimiter = ',')]
        headers: Option<Vec<String>>,
    },

    /// List collections and requests
    List {
        /// List type: collections, requests, environments
        #[arg(short, long, default_value = "collections")]
        r#type: String,

        /// Filter by collection ID
        #[arg(short, long)]
        collection: Option<i32>,
    },

    /// Export collection to JSON
    Export {
        /// Collection ID
        #[arg(short, long)]
        id: i32,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import collection from JSON
    Import {
        /// Input file path
        file: PathBuf,

        /// Collection name
        #[arg(short, long)]
        name: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct CliResult {
    pub success: bool,
    pub request: RequestInfo,
    pub response: Option<ResponseInfo>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RequestInfo {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseInfo {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub size: usize,
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    let db_path = expand_tilde(&cli.database.unwrap_or_else(|| "~/.astraio/data.db".to_string()));
    let conn = Connection::open(&db_path)
        .map_err(|e| AppError::Database(format!("Failed to open database: {}", e)))?;

    // Run keyring migration if needed
    let store = crate::services::secret_store::SecretStore::new();
    let _ = crate::services::secret_store::migrate_plaintext_tokens_to_keyring(&store, &conn);

    match cli.command {
        Commands::Run { id, url, method, body, headers } => {
            run_single_request(&conn, id, url, method, body, headers, &cli)?;
        }
        Commands::Collection { id, name, stop_on_failure, delay } => {
            run_collection(&conn, id, name, stop_on_failure, delay, &cli)?;
        }
        Commands::Quick { url, method, body, headers } => {
            run_quick_request(&url, &method, body, headers, &cli)?;
        }
        Commands::List { r#type, collection } => {
            list_items(&conn, &r#type, collection)?;
        }
        Commands::Export { id, output } => {
            export_collection(&conn, id, output)?;
        }
        Commands::Import { file, name } => {
            import_collection(&conn, file, name)?;
        }
    }

    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

fn parse_header(raw: &str) -> Option<(String, String)> {
    let (key, value) = raw.split_once(':')?;
    let key = key.trim().to_string();
    let value = value.trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

fn run_single_request(
    conn: &Connection,
    id: i32,
    url: Option<String>,
    method: Option<String>,
    body: Option<String>,
    headers: Option<Vec<String>>,
    cli: &Cli,
) -> Result<(), AppError> {
    let entry = database::get_collection_request_by_id(conn, id)?
        .ok_or_else(|| AppError::Parse(format!("Request with ID {} not found", id)))?;

    let mut request = build_request_from_entry(&entry)?;

    // Apply overrides
    if let Some(u) = url {
        request.url = u;
    }
    if let Some(m) = method {
        request.method = m.parse().map_err(|_| AppError::Parse(format!("Invalid HTTP method: {}", m)))?;
    }
    if let Some(b) = body {
        request.body = Some(b);
    }
    if let Some(h) = headers {
        for header in h {
            if let Some((key, value)) = parse_header(&header) {
                request.headers.push((key, value));
            } else {
                eprintln!("Warning: skipping malformed header: {}", header);
            }
        }
    }

    // Apply environment variables if specified
    if let Some(env_name) = &cli.environment {
        if let Some(env) = get_environment_by_name(conn, env_name) {
            apply_environment_to_request(&mut request, &env);
        }
    }

    let result = execute_request(&request, cli.timeout, cli.insecure)?;
    print_result(&result, &cli.format);

    Ok(())
}

fn run_collection(
    conn: &Connection,
    id: Option<i32>,
    name: Option<String>,
    stop_on_failure: bool,
    delay: u64,
    cli: &Cli,
) -> Result<(), AppError> {
    let collection_id = if let Some(id) = id {
        id
    } else if let Some(name) = &name {
        database::get_collection_by_name(conn, name)?
            .ok_or_else(|| AppError::Parse(format!("Collection '{}' not found", name)))?
            .id
    } else {
        return Err(AppError::Validation("Either --id or --name must be provided".to_string()));
    };

    let requests = database::get_collection_requests(conn, collection_id)?;
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    println!("Running collection {} ({} requests)\n", collection_id, requests.len());

    for (i, entry) in requests.iter().enumerate() {
        print!("[{}/{}] {} {} ... ", i + 1, requests.len(), entry.method, entry.name);

        let request = build_request_from_entry(entry)?;
        match execute_request(&request, cli.timeout, cli.insecure) {
            Ok(result) => {
                if let Some(status) = result.response.as_ref().map(|r| r.status) {
                    if result.success {
                        println!("✓ {} ({}ms)", status, result.duration_ms);
                        passed += 1;
                    } else {
                        println!("✗ {} ({}ms)", status, result.duration_ms);
                        failed += 1;
                        if stop_on_failure {
                            break;
                        }
                    }
                } else {
                    eprintln!("✗ ERROR: {}", result.error.as_deref().unwrap_or("unknown error"));
                    failed += 1;
                    if stop_on_failure {
                        break;
                    }
                }
                results.push(result);
            }
            Err(e) => {
                eprintln!("✗ ERROR: {}", e);
                failed += 1;
                results.push(CliResult {
                    success: false,
                    request: RequestInfo {
                        method: entry.method.clone(),
                        url: entry.url.clone(),
                        headers: entry.headers.clone(),
                        body: entry.body.clone(),
                    },
                    response: None,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                });
                if stop_on_failure {
                    break;
                }
            }
        }

        if delay > 0 && i < requests.len() - 1 {
            std::thread::sleep(Duration::from_millis(delay));
        }
    }

    println!("\n--- Summary ---");
    println!("Total: {}, Passed: {}, Failed: {}", requests.len(), passed, failed);

    if cli.format == OutputFormat::Json {
        let summary = serde_json::json!({
            "total": requests.len(),
            "passed": passed,
            "failed": failed,
            "results": results,
        });
        println!("\n{}", serde_json::to_string_pretty(&summary)?);
    }

    Ok(())
}

fn run_quick_request(
    url: &str,
    method: &str,
    body: Option<String>,
    headers: Option<Vec<String>>,
    cli: &Cli,
) -> Result<(), AppError> {
    let mut request = HttpRequest {
        method: method.parse().map_err(|_| AppError::Parse(format!("Invalid HTTP method: {}", method)))?,
        url: url.to_string(),
        headers: Vec::new(),
        body,
        config: RequestConfig::default(),
        multipart_fields: Vec::new(),
        auth: None,
    };

    if let Some(h) = headers {
        for header in h {
            if let Some((key, value)) = parse_header(&header) {
                request.headers.push((key, value));
            } else {
                eprintln!("Warning: skipping malformed header: {}", header);
            }
        }
    }

    let result = execute_request(&request, cli.timeout, cli.insecure)?;
    print_result(&result, &cli.format);

    Ok(())
}

fn list_items(
    conn: &Connection,
    r#type: &str,
    collection_id: Option<i32>,
) -> Result<(), AppError> {
    match r#type {
        "collections" => {
            let collections = database::get_collections(conn)?;
            println!("Collections ({}):\n", collections.len());
            for c in &collections {
                println!("  [{}] {}", c.id, c.name);
            }
        }
        "requests" => {
            let cid = collection_id.ok_or_else(|| AppError::Validation("--collection required for listing requests".to_string()))?;
            let requests = database::get_collection_requests(conn, cid)?;
            println!("Requests in collection {} ({}):\n", cid, requests.len());
            for r in &requests {
                println!("  [{}] {} {} {}", r.id, r.method, r.name, r.url);
            }
        }
        "environments" => {
            let envs = database::get_environments(conn)?;
            println!("Environments ({}):\n", envs.len());
            for e in &envs {
                println!("  [{}] {}", e.id, e.name);
            }
        }
        _ => return Err(AppError::Parse(format!("Unknown list type: {}", r#type))),
    }
    Ok(())
}

fn export_collection(
    conn: &Connection,
    id: i32,
    output: Option<String>,
) -> Result<(), AppError> {
    let collection = database::get_collection_by_id(conn, id)?
        .ok_or_else(|| AppError::Parse(format!("Collection {} not found", id)))?;
    let requests = database::get_collection_requests(conn, id)?;

    let export_data = serde_json::json!({
        "collection": collection,
        "requests": requests,
    });

    let json = serde_json::to_string_pretty(&export_data)?;

    if let Some(path) = output {
        std::fs::write(&path, &json)?;
        println!("Exported to {}", path);
    } else {
        println!("{}", json);
    }

    Ok(())
}

fn import_collection(
    conn: &Connection,
    file: PathBuf,
    name: Option<String>,
) -> Result<(), AppError> {
    let content = std::fs::read_to_string(&file)
        .map_err(|e| AppError::Io(format!("Failed to read file {}: {}", file.display(), e)))?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let collection_name = name.unwrap_or_else(|| {
        data.get("collection")
            .and_then(|c| c.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Imported Collection")
            .to_string()
    });

    let collection = database::create_collection(conn, &collection_name, None)?;
    println!("Created collection: [{}] {}", collection.id, collection.name);

    if let Some(requests) = data.get("requests").and_then(|r| r.as_array()) {
        for req in requests {
            let name = req.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed");
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
            let url = req.get("url").and_then(|u| u.as_str()).unwrap_or("");

            database::save_collection_request(
                conn,
                &database::SaveRequestParams::new(collection.id, name, method, url),
            )?;
            println!("  Imported: {} {} {}", method, name, url);
        }
    }

    println!("\nImport complete!");
    Ok(())
}

fn build_request_from_entry(entry: &CollectionRequest) -> Result<HttpRequest, AppError> {
    let mut headers = entry.headers.clone();

    // Add content type for body
    if entry.body.is_some() && entry.body_type == crate::persistence::database::CollectionBodyType::Text {
        let content_type = if let Some(body) = &entry.body {
            if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
                "application/json"
            } else if body.trim_start().starts_with('<') {
                "application/xml"
            } else {
                "text/plain"
            }
        } else {
            "text/plain"
        };
        headers.push(("Content-Type".to_string(), content_type.to_string()));
    }

    let method = entry.method.parse().map_err(|_| AppError::Parse(format!("Invalid HTTP method in collection: {}", entry.method)))?;

    Ok(HttpRequest {
        method,
        url: entry.url.clone(),
        headers,
        body: entry.body.clone(),
        config: RequestConfig::default(),
        multipart_fields: Vec::new(),
        auth: None,
    })
}

fn apply_environment_to_request(request: &mut HttpRequest, env: &Environment) {
    let vars = parse_env_variables(&env.variables);

    // Replace in URL
    for (key, value) in &vars {
        request.url = request.url.replace(&format!("{{{{{}}}}}", key), value);
    }

    // Replace in headers
    for (_, value) in &mut request.headers {
        for (key, val) in &vars {
            *value = value.replace(&format!("{{{{{}}}}}", key), val);
        }
    }

    // Replace in body
    if let Some(body) = &mut request.body {
        for (key, value) in &vars {
            *body = body.replace(&format!("{{{{{}}}}}", key), value);
        }
    }
}

fn parse_env_variables(variables: &str) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    for line in variables.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                vars.push((key, value));
            }
        }
    }
    vars
}

fn get_environment_by_name(conn: &Connection, name: &str) -> Option<Environment> {
    database::get_environments(conn)
        .ok()?
        .into_iter()
        .find(|e| e.name == name)
}

fn execute_request(
    request: &HttpRequest,
    timeout: u64,
    insecure: bool,
) -> Result<CliResult, AppError> {
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .danger_accept_invalid_certs(insecure)
        .build()?;

    let method = match request.method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
        _ => reqwest::Method::GET,
    };

    let mut req_builder = client.request(method, &request.url);

    for (key, value) in &request.headers {
        req_builder = req_builder.header(key, value);
    }

    if let Some(body) = &request.body {
        req_builder = req_builder.body(body.clone());
    }

    let response = req_builder.send()?;
    let duration = start.elapsed();
    let status = response.status().as_u16();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = response.text()?;
    let size = body.len();

    Ok(CliResult {
        success: status >= 200 && status < 300,
        request: RequestInfo {
            method: request.method.to_string(),
            url: request.url.clone(),
            headers: request.headers.clone(),
            body: request.body.clone(),
        },
        response: Some(ResponseInfo {
            status,
            headers,
            body,
            size,
        }),
        error: None,
        duration_ms: duration.as_millis() as u64,
    })
}

fn print_result(result: &CliResult, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result).unwrap_or_default());
        }
        OutputFormat::Csv => {
            if let Some(resp) = &result.response {
                println!("status,body_size,duration_ms");
                println!("{},{},{}", resp.status, resp.size, result.duration_ms);
                println!("\n{}", resp.body);
            } else if let Some(err) = &result.error {
                eprintln!("error: {}", err);
            }
        }
        OutputFormat::Text => {
            if let Some(resp) = &result.response {
                println!("Status: {}", resp.status);
                println!("Size: {} bytes", resp.size);
                println!("Duration: {}ms", result.duration_ms);
                println!("\nHeaders:");
                for (k, v) in &resp.headers {
                    println!("  {}: {}", k, v);
                }
                println!("\nBody:");
                println!("{}", resp.body);
            } else if let Some(err) = &result.error {
                eprintln!("Error: {}", err);
            }
        }
    }
}
