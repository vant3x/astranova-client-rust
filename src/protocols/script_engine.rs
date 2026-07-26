use crate::error::AppError;
use crate::http_client::request::HttpRequest;
use crate::http_client::response::HttpResponse;
use rquickjs::{Context, Function, Object, Runtime, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ScriptOutput {
    pub variables: HashMap<String, String>,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
    pub test_results: Vec<TestResult>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

struct EngineState {
    variables: HashMap<String, String>,
    logs: Vec<String>,
    errors: Vec<String>,
    test_results: Vec<TestResult>,
    request_url: String,
    request_method: String,
    request_body: Option<String>,
    request_headers: Vec<(String, String)>,
}

impl Clone for EngineState {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            logs: self.logs.clone(),
            errors: self.errors.clone(),
            test_results: self.test_results.clone(),
            request_url: self.request_url.clone(),
            request_method: self.request_method.clone(),
            request_body: self.request_body.clone(),
            request_headers: self.request_headers.clone(),
        }
    }
}

impl EngineState {
    fn new_for_request(request: &HttpRequest, variables: &HashMap<String, String>) -> Self {
        Self {
            variables: variables.clone(),
            logs: Vec::new(),
            errors: Vec::new(),
            test_results: Vec::new(),
            request_url: request.url.clone(),
            request_method: format!("{}", request.method),
            request_body: request.body.clone(),
            request_headers: request.headers.clone(),
        }
    }

    fn new_for_response(response: &HttpResponse, variables: &HashMap<String, String>) -> Self {
        Self {
            variables: variables.clone(),
            logs: Vec::new(),
            errors: Vec::new(),
            test_results: Vec::new(),
            request_url: response.url.clone(),
            request_method: String::new(),
            request_body: None,
            request_headers: Vec::new(),
        }
    }

    fn apply_to_request(&self, request: &mut HttpRequest) {
        request.url = self.request_url.clone();
        if let Ok(method) = self.request_method.parse() {
            request.method = method;
        }
        request.body = self.request_body.clone();
        request.headers = self.request_headers.clone();
    }
}

pub struct ScriptEngineV2;

impl ScriptEngineV2 {
    pub fn execute_pre_request(
        js_code: &str,
        request: &mut HttpRequest,
        variables: &mut HashMap<String, String>,
    ) -> Result<ScriptOutput, AppError> {
        if js_code.trim().is_empty() {
            return Ok(ScriptOutput {
                variables: variables.clone(),
                logs: Vec::new(),
                errors: Vec::new(),
                test_results: Vec::new(),
            });
        }

        let state = Arc::new(Mutex::new(EngineState::new_for_request(request, variables)));

        let rt = Runtime::new().map_err(|e| {
            AppError::Http(format!("Failed to create QuickJS runtime: {}", e))
        })?;
        let ctx = Context::full(&rt).map_err(|e| {
            AppError::Http(format!("Failed to create QuickJS context: {}", e))
        })?;

        let exec_result = ctx.with(|ctx| {
            setup_pm_api(&ctx, state.clone(), false, None)?;
            execute_user_code(&ctx, js_code)
        });

        if let Err(e) = exec_result {
            let errs = state.lock().unwrap().errors.clone();
            if errs.is_empty() {
                return Err(e);
            }
        }

        let final_state = state.lock().unwrap().clone();
        final_state.apply_to_request(request);
        *variables = final_state.variables.clone();

        Ok(ScriptOutput {
            variables: final_state.variables,
            logs: final_state.logs,
            errors: final_state.errors,
            test_results: final_state.test_results,
        })
    }

    pub fn execute_post_response(
        js_code: &str,
        response: &HttpResponse,
        variables: &mut HashMap<String, String>,
    ) -> Result<ScriptOutput, AppError> {
        if js_code.trim().is_empty() {
            return Ok(ScriptOutput {
                variables: variables.clone(),
                logs: Vec::new(),
                errors: Vec::new(),
                test_results: Vec::new(),
            });
        }

        let state = Arc::new(Mutex::new(EngineState::new_for_response(response, variables)));

        let rt = Runtime::new().map_err(|e| {
            AppError::Http(format!("Failed to create QuickJS runtime: {}", e))
        })?;
        let ctx = Context::full(&rt).map_err(|e| {
            AppError::Http(format!("Failed to create QuickJS context: {}", e))
        })?;

        let exec_result = ctx.with(|ctx| {
            setup_pm_api(&ctx, state.clone(), true, Some(response))?;
            execute_user_code(&ctx, js_code)
        });

        if let Err(e) = exec_result {
            let errs = state.lock().unwrap().errors.clone();
            if errs.is_empty() {
                return Err(e);
            }
        }

        let final_state = state.lock().unwrap();
        *variables = final_state.variables.clone();

        Ok(ScriptOutput {
            variables: final_state.variables.clone(),
            logs: final_state.logs.clone(),
            errors: final_state.errors.clone(),
            test_results: final_state.test_results.clone(),
        })
    }
}

fn execute_user_code(
    ctx: &rquickjs::Ctx<'_>,
    js_code: &str,
) -> Result<(), AppError> {
    ctx.eval::<(), _>(js_code).map_err(|e| {
        AppError::Validation(format!("Script error: {}", e))
    })
}

fn setup_pm_api(
    ctx: &rquickjs::Ctx<'_>,
    state: Arc<Mutex<EngineState>>,
    has_response: bool,
    response: Option<&HttpResponse>,
) -> Result<(), AppError> {
    let globals = ctx.globals();
    let pm = Object::new(ctx.clone()).map_err(|e| {
        AppError::Http(format!("Failed to create pm: {}", e))
    })?;

    // ---- pm.environment ----
    {
        let env = Object::new(ctx.clone()).map_err(|e| {
            AppError::Http(format!("Failed to create env: {}", e))
        })?;

        let s = state.clone();
        let get_fn = Function::new(ctx.clone(), move |name: String| -> String {
            s.lock().unwrap().variables.get(&name).cloned().unwrap_or_default()
        }).map_err(|e| AppError::Http(format!("env.get: {}", e)))?;

        let s = state.clone();
        let set_fn = Function::new(ctx.clone(), move |name: String, value: String| {
            s.lock().unwrap().variables.insert(name, value);
        }).map_err(|e| AppError::Http(format!("env.set: {}", e)))?;

        env.set("get", get_fn).map_err(|e| AppError::Http(format!("env.get set: {}", e)))?;
        env.set("set", set_fn).map_err(|e| AppError::Http(format!("env.set set: {}", e)))?;
        pm.set("environment", env).map_err(|e| AppError::Http(format!("pm.environment: {}", e)))?;
    }

    // ---- pm.request ----
    {
        let req = Object::new(ctx.clone()).map_err(|e| {
            AppError::Http(format!("Failed to create req: {}", e))
        })?;

        let s = state.clone();
        let set_url = Function::new(ctx.clone(), move |url: String| {
            s.lock().unwrap().request_url = url;
        }).map_err(|e| AppError::Http(format!("set_url: {}", e)))?;

        let s = state.clone();
        let set_method = Function::new(ctx.clone(), move |method: String| {
            s.lock().unwrap().request_method = method;
        }).map_err(|e| AppError::Http(format!("set_method: {}", e)))?;

        let s = state.clone();
        let set_body = Function::new(ctx.clone(), move |body: String| {
            s.lock().unwrap().request_body = Some(body);
        }).map_err(|e| AppError::Http(format!("set_body: {}", e)))?;

        let s = state.clone();
        let set_header = Function::new(ctx.clone(), move |key: String, value: String| {
            let mut st = s.lock().unwrap();
            st.request_headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&key));
            st.request_headers.push((key, value));
        }).map_err(|e| AppError::Http(format!("set_header: {}", e)))?;

        let s = state.clone();
        let get_header = Function::new(ctx.clone(), move |key: String| -> String {
            let st = s.lock().unwrap();
            st.request_headers.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&key))
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        }).map_err(|e| AppError::Http(format!("get_header: {}", e)))?;

        let s = state.clone();
        let remove_header = Function::new(ctx.clone(), move |key: String| {
            s.lock().unwrap().request_headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&key));
        }).map_err(|e| AppError::Http(format!("remove_header: {}", e)))?;

        let s = state.clone();
        let get_url = Function::new(ctx.clone(), move || -> String {
            s.lock().unwrap().request_url.clone()
        }).map_err(|e| AppError::Http(format!("get_url: {}", e)))?;

        let s = state.clone();
        let get_method = Function::new(ctx.clone(), move || -> String {
            s.lock().unwrap().request_method.clone()
        }).map_err(|e| AppError::Http(format!("get_method: {}", e)))?;

        let s = state.clone();
        let get_body = Function::new(ctx.clone(), move || -> String {
            s.lock().unwrap().request_body.clone().unwrap_or_default()
        }).map_err(|e| AppError::Http(format!("get_body: {}", e)))?;

        req.set("setUrl", set_url).map_err(|e| AppError::Http(format!("req.setUrl: {}", e)))?;
        req.set("setMethod", set_method).map_err(|e| AppError::Http(format!("req.setMethod: {}", e)))?;
        req.set("setBody", set_body).map_err(|e| AppError::Http(format!("req.setBody: {}", e)))?;
        req.set("setHeader", set_header).map_err(|e| AppError::Http(format!("req.setHeader: {}", e)))?;
        req.set("getHeader", get_header).map_err(|e| AppError::Http(format!("req.getHeader: {}", e)))?;
        req.set("removeHeader", remove_header).map_err(|e| AppError::Http(format!("req.removeHeader: {}", e)))?;
        req.set("getUrl", get_url).map_err(|e| AppError::Http(format!("req.getUrl: {}", e)))?;
        req.set("getMethod", get_method).map_err(|e| AppError::Http(format!("req.getMethod: {}", e)))?;
        req.set("getBody", get_body).map_err(|e| AppError::Http(format!("req.getBody: {}", e)))?;
        pm.set("request", req).map_err(|e| AppError::Http(format!("pm.request: {}", e)))?;
    }

    // ---- pm.response ----
    if has_response {
        if let Some(resp) = response {
            let res = Object::new(ctx.clone()).map_err(|e| {
                AppError::Http(format!("Failed to create res: {}", e))
            })?;

            res.set("status", resp.status).map_err(|e| AppError::Http(format!("res.status: {}", e)))?;
            res.set("body", resp.body.clone()).map_err(|e| AppError::Http(format!("res.body: {}", e)))?;
            res.set("url", resp.url.clone()).map_err(|e| AppError::Http(format!("res.url: {}", e)))?;
            res.set("method", format!("{}", resp.method)).map_err(|e| AppError::Http(format!("res.method: {}", e)))?;
            res.set("responseTime", resp.duration.as_millis() as u64)
                .map_err(|e| AppError::Http(format!("res.responseTime: {}", e)))?;
            res.set("size", resp.size).map_err(|e| AppError::Http(format!("res.size: {}", e)))?;

            let headers = Object::new(ctx.clone()).map_err(|e| {
                AppError::Http(format!("Failed to create res.headers: {}", e))
            })?;
            for (k, v) in &resp.headers {
                headers.set(k.as_str(), v.as_str()).ok();
            }
            res.set("headers", headers).map_err(|e| AppError::Http(format!("res.headers: {}", e)))?;

            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&resp.body) {
                let json_str = serde_json::to_string(&json_val).unwrap_or_default();
                if let Ok(js_val) = ctx.eval::<Value<'_>, _>(format!("({})", json_str)) {
                    res.set("json", js_val).ok();
                }
            }

            pm.set("response", res).map_err(|e| AppError::Http(format!("pm.response: {}", e)))?;
        }
    }

    // ---- pm.log ----
    {
        let s = state.clone();
        let log_fn = Function::new(ctx.clone(), move |args: rquickjs::function::Rest<Value<'_>>| {
            let parts: Vec<String> = args.0.into_iter().map(|v| {
                v.as_string().and_then(|s| s.to_string().ok()).unwrap_or_default()
            }).collect();
            s.lock().unwrap().logs.push(parts.join(" "));
        }).map_err(|e| AppError::Http(format!("pm.log: {}", e)))?;
        pm.set("log", log_fn).map_err(|e| AppError::Http(format!("pm.log set: {}", e)))?;
    }

    // ---- pm.test ----
    {
        let s = state.clone();
        let test_fn = Function::new(ctx.clone(), move |name: String, test_fn_val: Function<'_>| {
            let mut result = TestResult {
                name,
                passed: false,
                message: None,
            };
            match test_fn_val.call::<(), ()>(()) {
                Ok(_) => result.passed = true,
                Err(e) => result.message = Some(format!("{}", e)),
            }
            s.lock().unwrap().test_results.push(result);
        }).map_err(|e| AppError::Http(format!("pm.test: {}", e)))?;
        pm.set("test", test_fn).map_err(|e| AppError::Http(format!("pm.test set: {}", e)))?;
    }

    // ---- pm.expect (defined via JS to avoid lifetime issues) ----
    {
        let s2 = state.clone();
        let collect_error = Function::new(ctx.clone(), move |msg: String| {
            s2.lock().unwrap().errors.push(msg);
        }).map_err(|e| AppError::Http(format!("collect_error: {}", e)))?;
        pm.set("__collectError", collect_error).map_err(|e| AppError::Http(format!("pm.__collectError: {}", e)))?;
    }

    // Set pm on globals BEFORE defining pm.expect via JS eval
    globals.set("pm", pm).map_err(|e| AppError::Http(format!("Failed to set global pm: {}", e)))?;

    // ---- pm.expect (defined via JS to avoid lifetime issues) ----
    ctx.eval::<(), _>(r#"
            pm.expect = function(value) {
                var valStr = String(value);
                return {
                    toBe: function(expected) {
                        if (valStr !== expected) {
                            var msg = "Expected '" + valStr + "' to be '" + expected + "'";
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    },
                    toBeTruthy: function() {
                        if (valStr === "" || valStr === "false" || valStr === "0" || valStr === "null") {
                            var msg = "Expected value to be truthy, got '" + valStr + "'";
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    },
                    toContain: function(substr) {
                        if (valStr.indexOf(substr) === -1) {
                            var msg = "Expected '" + valStr + "' to contain '" + substr + "'";
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    },
                    toBeGreaterThan: function(expected) {
                        var num = parseFloat(valStr);
                        if (!isNaN(num) && num <= expected) {
                            var msg = "Expected " + valStr + " to be greater than " + expected;
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    },
                    toBeLessThan: function(expected) {
                        var num = parseFloat(valStr);
                        if (!isNaN(num) && num >= expected) {
                            var msg = "Expected " + valStr + " to be less than " + expected;
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    },
                    toHaveLength: function(len) {
                        if (valStr.length !== len) {
                            var msg = "Expected length " + valStr.length + " to be " + len;
                            pm.__collectError(msg);
                            throw new Error(msg);
                        }
                    }
                };
            };
        "#).map_err(|e| AppError::Http(format!("Failed to define pm.expect: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::request::{HttpMethod, HttpRequest};
    use crate::http_client::response::{BodyEncoding, HttpResponse};
    use std::time::Duration;

    fn make_request() -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![("Accept".to_string(), "application/json".to_string())],
            body: None,
            config: Default::default(),
            multipart_fields: vec![],
            auth: None,
        }
    }

    fn make_response() -> HttpResponse {
        HttpResponse {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: r#"{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}"#.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(150),
            size: 80,
            redirect_chain: vec![],
        }
    }

    #[test]
    fn test_empty_script() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request("", &mut req, &mut vars).unwrap();
        assert!(result.logs.is_empty());
    }

    #[test]
    fn test_pm_log() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(r#"pm.log("hello", "world");"#, &mut req, &mut vars).unwrap();
        assert_eq!(result.logs.len(), 1);
        assert!(result.logs[0].contains("hello"));
    }

    #[test]
    fn test_environment_set_get() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            pm.environment.set("token", "abc123");
            pm.log("token=" + pm.environment.get("token"));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("token").unwrap(), "abc123");
        assert!(result.logs[0].contains("abc123"));
    }

    #[test]
    fn test_request_set_url() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        ScriptEngineV2::execute_pre_request(r#"pm.request.setUrl("https://api.example.com/products");"#, &mut req, &mut vars).unwrap();
        assert_eq!(req.url, "https://api.example.com/products");
    }

    #[test]
    fn test_request_set_method() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        ScriptEngineV2::execute_pre_request(r#"pm.request.setMethod("POST");"#, &mut req, &mut vars).unwrap();
        assert_eq!(format!("{}", req.method), "POST");
    }

    #[test]
    fn test_request_set_body() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        ScriptEngineV2::execute_pre_request(r#"pm.request.setBody('{"name":"test"}');"#, &mut req, &mut vars).unwrap();
        assert_eq!(req.body.unwrap(), r#"{"name":"test"}"#);
    }

    #[test]
    fn test_request_set_header() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        ScriptEngineV2::execute_pre_request(r#"pm.request.setHeader("Authorization", "Bearer tok123");"#, &mut req, &mut vars).unwrap();
        assert!(req.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer tok123"));
    }

    #[test]
    fn test_request_remove_header() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        ScriptEngineV2::execute_pre_request(r#"pm.request.removeHeader("Accept");"#, &mut req, &mut vars).unwrap();
        assert!(!req.headers.iter().any(|(k, _)| k == "Accept"));
    }

    #[test]
    fn test_js_functions_and_loops() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            function fibonacci(n) {
                if (n <= 1) return n;
                return fibonacci(n - 1) + fibonacci(n - 2);
            }
            pm.environment.set("fib10", String(fibonacci(10)));
            let sum = 0;
            for (let i = 1; i <= 100; i++) sum += i;
            pm.environment.set("sum1to100", String(sum));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("fib10").unwrap(), "55");
        assert_eq!(result.variables.get("sum1to100").unwrap(), "5050");
    }

    #[test]
    fn test_js_objects_json() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            let user = { name: "Alice", age: 30 };
            pm.environment.set("userName", user.name);
            pm.environment.set("userJson", JSON.stringify(user));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("userName").unwrap(), "Alice");
        assert!(result.variables.get("userJson").unwrap().contains("Alice"));
    }

    #[test]
    fn test_js_arrays_map_filter_reduce() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            let nums = [1, 2, 3, 4, 5];
            pm.environment.set("doubled", JSON.stringify(nums.map(x => x * 2)));
            pm.environment.set("evens", JSON.stringify(nums.filter(x => x % 2 === 0)));
            pm.environment.set("sum", String(nums.reduce((a, b) => a + b, 0)));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert!(result.variables.get("doubled").unwrap().contains("10"));
        assert!(result.variables.get("evens").unwrap().contains("2"));
        assert_eq!(result.variables.get("sum").unwrap(), "15");
    }

    #[test]
    fn test_js_string_methods() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            pm.environment.set("trimmed", "  hi  ".trim());
            pm.environment.set("upper", "hello".toUpperCase());
            pm.environment.set("replaced", "hello world".replace("world", "JS"));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("trimmed").unwrap(), "hi");
        assert_eq!(result.variables.get("upper").unwrap(), "HELLO");
        assert_eq!(result.variables.get("replaced").unwrap(), "hello JS");
    }

    #[test]
    fn test_js_try_catch() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            try { JSON.parse("not json"); } catch(e) { pm.environment.set("caught", "true"); }
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("caught").unwrap(), "true");
    }

    #[test]
    fn test_js_classes() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            class User {
                constructor(name, role) { this.name = name; this.role = role; }
                isAdmin() { return this.role === "admin"; }
            }
            let u = new User("Alice", "admin");
            pm.environment.set("name", u.name);
            pm.environment.set("admin", String(u.isAdmin()));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert_eq!(result.variables.get("name").unwrap(), "Alice");
        assert_eq!(result.variables.get("admin").unwrap(), "true");
    }

    #[test]
    fn test_pm_test_passing() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(r#"pm.test("basic", function() {});"#, &mut req, &mut vars).unwrap();
        assert_eq!(result.test_results.len(), 1);
        assert!(result.test_results[0].passed);
    }

    #[test]
    fn test_pm_test_failing() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(r#"pm.test("fail", function() { throw new Error("nope"); });"#, &mut req, &mut vars).unwrap();
        assert!(!result.test_results[0].passed);
    }

    #[test]
    fn test_expect_to_be() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(
            r#"pm.test("match", function() { pm.expect("hello").toBe("hello"); });"#,
            &mut req, &mut vars,
        ).unwrap();
        assert!(result.test_results[0].passed);
    }

    #[test]
    fn test_expect_to_contain() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(
            r#"pm.test("contain", function() { pm.expect("hello world").toContain("world"); });"#,
            &mut req, &mut vars,
        ).unwrap();
        assert!(result.test_results[0].passed);
    }

    #[test]
    fn test_expect_to_be_fails() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let result = ScriptEngineV2::execute_pre_request(
            r#"pm.test("fail", function() { pm.expect("hello").toBe("world"); });"#,
            &mut req, &mut vars,
        ).unwrap();
        assert!(!result.test_results[0].passed);
    }

    #[test]
    fn test_post_response_status() {
        let resp = make_response();
        let mut vars = HashMap::new();
        let code = r#"
            pm.test("status", function() {
                if (pm.response.status !== 200) throw new Error("bad status");
            });
        "#;
        let result = ScriptEngineV2::execute_post_response(code, &resp, &mut vars).unwrap();
        assert!(result.test_results[0].passed);
    }

    #[test]
    fn test_post_response_json() {
        let resp = make_response();
        let mut vars = HashMap::new();
        let code = r#"
            let data = pm.response.json;
            pm.environment.set("firstUser", data.users[0].name);
            pm.environment.set("count", String(data.users.length));
        "#;
        let result = ScriptEngineV2::execute_post_response(code, &resp, &mut vars).unwrap();
        assert_eq!(result.variables.get("firstUser").unwrap(), "Alice");
        assert_eq!(result.variables.get("count").unwrap(), "2");
    }

    #[test]
    fn test_post_response_multiple_tests() {
        let resp = make_response();
        let mut vars = HashMap::new();
        let code = r#"
            pm.test("status 200", function() {
                pm.expect(String(pm.response.status)).toBe("200");
            });
            pm.test("has users", function() {
                let d = pm.response.json;
                if (!d.users || d.users.length === 0) throw new Error("no users");
            });
            pm.test("fast enough", function() {
                if (pm.response.responseTime > 5000) throw new Error("too slow");
            });
        "#;
        let result = ScriptEngineV2::execute_post_response(code, &resp, &mut vars).unwrap();
        assert_eq!(result.test_results.len(), 3);
        assert!(result.test_results.iter().all(|t| t.passed));
    }

    #[test]
    fn test_complex_workflow() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        let code = r#"
            pm.environment.set("requestId", "req_" + Date.now());
            pm.request.setHeader("X-Request-Id", pm.environment.get("requestId"));
            pm.request.setHeader("Content-Type", "application/json");
            pm.request.setBody(JSON.stringify({ query: "users", page: 1 }));
            pm.request.setMethod("POST");
            function formatCurrency(amount) { return "$" + amount.toFixed(2); }
            pm.environment.set("price", formatCurrency(42.5));
            let data = [10, 20, 30, 40, 50];
            pm.environment.set("filtered", String(data.filter(x => x > 25).length));
        "#;
        let result = ScriptEngineV2::execute_pre_request(code, &mut req, &mut vars).unwrap();
        assert!(req.headers.iter().any(|(k, _)| k == "X-Request-Id"));
        assert!(req.headers.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
        assert_eq!(format!("{}", req.method), "POST");
        assert_eq!(result.variables.get("price").unwrap(), "$42.50");
        assert_eq!(result.variables.get("filtered").unwrap(), "3");
    }

    #[test]
    fn test_syntax_error() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        assert!(ScriptEngineV2::execute_pre_request(r#"invalid @@@"#, &mut req, &mut vars).is_err());
    }

    #[test]
    fn test_runtime_error() {
        let mut req = make_request();
        let mut vars = HashMap::new();
        assert!(ScriptEngineV2::execute_pre_request(r#"undefinedVar.foo;"#, &mut req, &mut vars).is_err());
    }
}
