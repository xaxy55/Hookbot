use colored::Colorize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Debug, serde::Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

fn severity_colored(s: Severity) -> colored::ColoredString {
    match s {
        Severity::Info => format!("[{s}]").dimmed(),
        Severity::Low => format!("[{s}]").blue(),
        Severity::Medium => format!("[{s}]").yellow(),
        Severity::High => format!("[{s}]").red(),
        Severity::Critical => format!("[{s}]").red().bold(),
    }
}

type Findings = Arc<Mutex<Vec<Finding>>>;

fn should_run(check: &str, only: Option<&str>, skip: Option<&str>) -> bool {
    if let Some(only) = only {
        return only.split(',').any(|c| c.trim().eq_ignore_ascii_case(check));
    }
    if let Some(skip) = skip {
        return !skip.split(',').any(|c| c.trim().eq_ignore_ascii_case(check));
    }
    true
}

pub async fn run(
    api_url: &str,
    frontend_url: Option<&str>,
    only: Option<&str>,
    skip: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if !json {
        println!("{}", "=== Hookbot Security Audit ===".bold());
        println!("API target:      {api_url}");
        if let Some(fe) = frontend_url {
            println!("Frontend target: {fe}");
        }
        println!();
    }

    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?,
    );

    let start = Instant::now();
    let findings: Findings = Arc::new(Mutex::new(Vec::new()));

    if !json { println!("{}", "Running checks (parallel)...".dimmed()); }

    // Launch all independent checks concurrently
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    macro_rules! spawn_check {
        ($name:expr, $fn:expr) => {
            if should_run($name, only, skip) {
                handles.push(tokio::spawn($fn));
            }
        };
    }

    let api = api_url.to_string();
    let fe = frontend_url.map(|s| s.to_string());

    // Reachability
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("reachability", async move { check_server_reachable(&c, &u, &f).await; });
    }
    // API security headers
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("headers", async move { check_security_headers(&c, &u, &f, "API").await; });
    }
    // Frontend security headers
    if let Some(ref fe_url) = fe {
        let c = client.clone(); let f = findings.clone(); let u = fe_url.clone();
        spawn_check!("headers", async move { check_security_headers(&c, &u, &f, "Frontend").await; });
    }
    // CORS
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("cors", async move { check_cors(&c, &u, &f).await; });
    }
    // CORS preflight
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("cors", async move { check_cors_preflight(&c, &u, &f).await; });
    }
    // Auth enforcement
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("auth", async move { check_auth_enforcement(&c, &u, &f).await; });
    }
    // Cookie security
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("cookies", async move { check_cookie_security(&c, &u, &f).await; });
    }
    // HTTP methods
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("methods", async move { check_http_methods(&c, &u, &f).await; });
    }
    // Info disclosure
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("disclosure", async move { check_info_disclosure(&c, &u, &f).await; });
    }
    // Rate limiting
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("ratelimit", async move { check_rate_limiting(&c, &u, &f).await; });
    }
    // TLS - API
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("tls", async move { check_tls(&c, &u, &f).await; });
    }
    // TLS - Frontend
    if let Some(ref fe_url) = fe {
        let c = client.clone(); let f = findings.clone(); let u = fe_url.clone();
        spawn_check!("tls", async move { check_tls(&c, &u, &f).await; });
    }
    // Sensitive paths
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("paths", async move { check_common_paths(&c, &u, &f).await; });
    }
    // Frontend sensitive paths
    if let Some(ref fe_url) = fe {
        let c = client.clone(); let f = findings.clone(); let u = fe_url.clone();
        spawn_check!("paths", async move { check_common_paths(&c, &u, &f).await; });
    }
    // Injection
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("injection", async move { check_injection_vectors(&c, &u, &f).await; });
    }
    // Open redirect
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("redirect", async move { check_open_redirect(&c, &u, &f).await; });
    }
    // Directory traversal
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("traversal", async move { check_path_traversal(&c, &u, &f).await; });
    }
    // IDOR
    {
        let c = client.clone(); let f = findings.clone(); let u = api.clone();
        spawn_check!("idor", async move { check_idor(&c, &u, &f).await; });
    }
    // SSL cert
    {
        let f = findings.clone(); let u = api.clone();
        spawn_check!("cert", async move { check_ssl_cert(&u, &f).await; });
    }

    // Wait for all checks to complete
    for h in handles {
        let _ = h.await;
    }

    let elapsed = start.elapsed();
    let mut findings = Arc::try_unwrap(findings).unwrap().into_inner();

    // Sort by severity (critical first)
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));

    if json {
        let report = serde_json::json!({
            "target": api_url,
            "frontend": frontend_url,
            "duration_secs": elapsed.as_secs_f64(),
            "score": compute_score(&findings),
            "summary": {
                "critical": findings.iter().filter(|f| f.severity == Severity::Critical).count(),
                "high": findings.iter().filter(|f| f.severity == Severity::High).count(),
                "medium": findings.iter().filter(|f| f.severity == Severity::Medium).count(),
                "low": findings.iter().filter(|f| f.severity == Severity::Low).count(),
                "info": findings.iter().filter(|f| f.severity == Severity::Info).count(),
            },
            "findings": findings,
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return Ok(());
    }

    // Text report
    println!();
    println!("{}", "=== Results ===".bold());
    println!();

    if findings.is_empty() {
        println!("  {}", "No issues found.".green().bold());
    } else {
        let crit = findings.iter().filter(|f| f.severity == Severity::Critical).count();
        let high = findings.iter().filter(|f| f.severity == Severity::High).count();
        let med = findings.iter().filter(|f| f.severity == Severity::Medium).count();
        let low = findings.iter().filter(|f| f.severity == Severity::Low).count();
        let info = findings.iter().filter(|f| f.severity == Severity::Info).count();

        println!(
            "  {} critical, {} high, {} medium, {} low, {} info",
            crit.to_string().red().bold(),
            high.to_string().red(),
            med.to_string().yellow(),
            low.to_string().blue(),
            info.to_string().dimmed(),
        );
        println!();

        for (i, f) in findings.iter().enumerate() {
            println!(
                "  {}  {} {} — {}",
                format!("#{}", i + 1).dimmed(),
                severity_colored(f.severity),
                f.title.bold(),
                f.category.dimmed(),
            );
            println!("      {}", f.detail);
            println!("      {} {}", "Fix:".green(), f.remediation);
            println!();
        }
    }

    let score = compute_score(&findings);
    let score_color = if score >= 80.0 {
        format!("{score:.0}/100").green().bold()
    } else if score >= 50.0 {
        format!("{score:.0}/100").yellow().bold()
    } else {
        format!("{score:.0}/100").red().bold()
    };

    println!("{}", "=== Security Score ===".bold());
    println!("  {score_color}");
    println!();
    println!("  Completed in {:.1}s", elapsed.as_secs_f64());
    Ok(())
}

fn compute_score(findings: &[Finding]) -> f64 {
    let penalty: f64 = findings.iter().map(|f| match f.severity {
        Severity::Critical => 25.0,
        Severity::High => 15.0,
        Severity::Medium => 8.0,
        Severity::Low => 3.0,
        Severity::Info => 0.0,
    }).sum();
    (100.0 - penalty).max(0.0)
}

async fn add(findings: &Findings, severity: Severity, category: &str, title: &str, detail: &str, remediation: &str) {
    findings.lock().await.push(Finding {
        severity,
        category: category.to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        remediation: remediation.to_string(),
    });
}

fn is_down(status: u16) -> bool {
    status == 522 || status == 521 || status == 523 || status == 520
}

async fn check_server_reachable(client: &reqwest::Client, url: &str, findings: &Findings) {
    match client.get(&format!("{url}/api/health")).send().await {
        Ok(resp) => {
            if is_down(resp.status().as_u16()) {
                add(findings, Severity::High, "A07:Security Misconfiguration",
                    "Origin server unreachable",
                    &format!("Cloudflare returns {} — origin is down.", resp.status()),
                    "Ensure the server process is running and the port is open."
                ).await;
            }
        }
        Err(e) => {
            add(findings, Severity::High, "Availability",
                "Server unreachable",
                &format!("Connection failed: {e}"),
                "Verify the server is running and the URL is correct."
            ).await;
        }
    }
}

async fn check_security_headers(client: &reqwest::Client, url: &str, findings: &Findings, label: &str) {
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return,
    };

    if is_down(resp.status().as_u16()) { return; }

    let headers = resp.headers().clone();

    let required: &[(&str, Severity, &str)] = &[
        ("strict-transport-security", Severity::High,
         "Add Strict-Transport-Security: max-age=31536000; includeSubDomains"),
        ("content-security-policy", Severity::High,
         "Add Content-Security-Policy: default-src 'self'; script-src 'self'"),
        ("x-frame-options", Severity::Medium,
         "Add X-Frame-Options: DENY to prevent clickjacking"),
        ("x-content-type-options", Severity::Low,
         "Add X-Content-Type-Options: nosniff"),
        ("referrer-policy", Severity::Low,
         "Add Referrer-Policy: strict-origin-when-cross-origin"),
        ("permissions-policy", Severity::Low,
         "Add Permissions-Policy: camera=(), microphone=(), geolocation=()"),
    ];

    for (header, sev, fix) in required {
        if !headers.contains_key(*header) {
            add(findings, *sev, "A05:Security Misconfiguration",
                &format!("{label}: Missing {header}"),
                &format!("The {header} header is not set on {url}"),
                fix
            ).await;
        }
    }

    // Check for weak CSP
    if let Some(csp) = headers.get("content-security-policy").and_then(|v| v.to_str().ok()) {
        if csp.contains("unsafe-inline") || csp.contains("unsafe-eval") {
            add(findings, Severity::Medium, "A05:Security Misconfiguration",
                &format!("{label}: Weak CSP policy"),
                &format!("CSP contains unsafe directives: {csp}"),
                "Remove 'unsafe-inline' and 'unsafe-eval' from CSP."
            ).await;
        }
    }

    // Server version disclosure
    if let Some(server) = headers.get("server").and_then(|v| v.to_str().ok()) {
        if server.contains('/') && !server.starts_with("cloudflare") {
            add(findings, Severity::Low, "A05:Security Misconfiguration",
                &format!("{label}: Server version disclosed"),
                &format!("Server header reveals: {server}"),
                "Remove version info from Server header."
            ).await;
        }
    }

    // X-Powered-By disclosure
    if let Some(powered) = headers.get("x-powered-by").and_then(|v| v.to_str().ok()) {
        add(findings, Severity::Low, "A05:Security Misconfiguration",
            &format!("{label}: Technology stack disclosed"),
            &format!("X-Powered-By: {powered}"),
            "Remove the X-Powered-By header."
        ).await;
    }
}

async fn check_cors(client: &reqwest::Client, url: &str, findings: &Findings) {
    let evil_origin = "https://evil-attacker.com";
    let resp = match client
        .get(&format!("{url}/api/health"))
        .header("Origin", evil_origin)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    if is_down(resp.status().as_u16()) { return; }

    let headers = resp.headers();
    let acao = headers.get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if acao == "*" || acao == evil_origin {
        let with_creds = headers.get("access-control-allow-credentials")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("") == "true";

        let severity = if with_creds { Severity::Critical } else { Severity::High };
        let detail = if with_creds {
            format!("CORS allows '{acao}' WITH credentials — any site can make authenticated requests.")
        } else {
            format!("CORS allows origin '{acao}' — cross-origin requests possible.")
        };

        add(findings, severity, "A01:Broken Access Control",
            "Permissive CORS policy",
            &detail,
            "Set ALLOWED_ORIGINS to your frontend domain(s) only."
        ).await;
    }
}

async fn check_cors_preflight(client: &reqwest::Client, url: &str, findings: &Findings) {
    let resp = match client
        .request(reqwest::Method::OPTIONS, &format!("{url}/api/devices"))
        .header("Origin", "https://evil.com")
        .header("Access-Control-Request-Method", "DELETE")
        .header("Access-Control-Request-Headers", "X-Custom-Header")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    if is_down(resp.status().as_u16()) { return; }

    let headers = resp.headers();
    let methods = headers.get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if methods.contains("*") {
        add(findings, Severity::Medium, "A01:Broken Access Control",
            "CORS preflight allows all methods",
            &format!("Access-Control-Allow-Methods: {methods}"),
            "Restrict allowed methods to GET, POST, PUT, DELETE."
        ).await;
    }

    let allow_headers = headers.get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if allow_headers.contains("*") {
        add(findings, Severity::Low, "A01:Broken Access Control",
            "CORS preflight allows all headers",
            &format!("Access-Control-Allow-Headers: {allow_headers}"),
            "Restrict allowed headers to specific known values."
        ).await;
    }
}

async fn check_auth_enforcement(client: &reqwest::Client, url: &str, findings: &Findings) {
    let protected = [
        ("GET", "/api/devices"),
        ("GET", "/api/settings"),
        ("GET", "/api/users"),
        ("GET", "/api/firmware"),
        ("GET", "/api/logs"),
        ("GET", "/api/ota/jobs"),
        ("GET", "/api/automation/rules"),
        ("GET", "/api/community/plugins"),
        ("DELETE", "/api/devices/1"),
        ("PUT", "/api/settings"),
    ];

    for (method, path) in &protected {
        let full = format!("{url}{path}");
        let resp = match *method {
            "GET" => client.get(&full).send().await,
            "DELETE" => client.delete(&full).send().await,
            "PUT" => client.put(&full).json(&serde_json::json!({})).send().await,
            _ => continue,
        };

        if let Ok(r) = resp {
            let status = r.status().as_u16();
            if status != 401 && status != 403 && !is_down(status) && status != 404 && status != 405 {
                add(findings, Severity::Critical, "A01:Broken Access Control",
                    "Unauthenticated access to protected endpoint",
                    &format!("{method} {path} => HTTP {status}"),
                    "Ensure auth middleware covers all sensitive routes."
                ).await;
            }
        }
    }
}

async fn check_cookie_security(client: &reqwest::Client, url: &str, findings: &Findings) {
    // Trigger a login attempt to see what cookies come back
    let resp = match client
        .post(&format!("{url}/api/auth/login"))
        .json(&serde_json::json!({"password": "test"}))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    if is_down(resp.status().as_u16()) { return; }

    for cookie_header in resp.headers().get_all("set-cookie") {
        let val = match cookie_header.to_str() {
            Ok(v) => v.to_lowercase(),
            Err(_) => continue,
        };

        let name = val.split('=').next().unwrap_or("unknown");

        if !val.contains("httponly") {
            add(findings, Severity::Medium, "A05:Security Misconfiguration",
                &format!("Cookie '{name}' missing HttpOnly flag"),
                "Session cookie accessible to JavaScript — XSS can steal sessions.",
                "Set HttpOnly flag on all session cookies."
            ).await;
        }

        if url.starts_with("https://") && !val.contains("secure") {
            add(findings, Severity::Medium, "A02:Cryptographic Failures",
                &format!("Cookie '{name}' missing Secure flag"),
                "Cookie can be sent over plain HTTP, exposing it to interception.",
                "Set Secure flag on all cookies when using HTTPS."
            ).await;
        }

        if !val.contains("samesite") {
            add(findings, Severity::Low, "A01:Broken Access Control",
                &format!("Cookie '{name}' missing SameSite attribute"),
                "Missing SameSite may allow CSRF attacks.",
                "Set SameSite=Lax or SameSite=Strict on cookies."
            ).await;
        }
    }
}

async fn check_http_methods(client: &reqwest::Client, url: &str, findings: &Findings) {
    for method in ["TRACE", "TRACK"] {
        let resp = match client
            .request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), &format!("{url}/api/health"))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        let status = resp.status().as_u16();
        if status != 405 && status != 501 && !is_down(status) {
            add(findings, Severity::Low, "A05:Security Misconfiguration",
                &format!("{method} method enabled"),
                &format!("{method} /api/health => HTTP {status}"),
                &format!("Disable {method} method.")
            ).await;
        }
    }
}

async fn check_info_disclosure(client: &reqwest::Client, url: &str, findings: &Findings) {
    // Test various error paths
    let test_paths = [
        "/api/nonexistent-path-1234",
        "/api/devices/999999999",
        "/api/../../../etc/passwd",
    ];

    let suspicious_words = [
        "stack trace", "at /", "panic", "thread '", "rust_backtrace",
        "sqlite", "rusqlite", "src/main.rs", "traceback", "internal server error",
        "diesel", "actix", "axum::", "tower::",
    ];

    for path in &test_paths {
        let resp = match client.get(&format!("{url}{path}")).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if is_down(resp.status().as_u16()) { return; }

        let body = resp.text().await.unwrap_or_default().to_lowercase();
        let leaked: Vec<&&str> = suspicious_words.iter()
            .filter(|kw| body.contains(&kw.to_lowercase()))
            .collect();

        if !leaked.is_empty() {
            add(findings, Severity::Medium, "A05:Security Misconfiguration",
                "Error response leaks internal info",
                &format!("{path} response contains: {}", leaked.iter().map(|s| **s).collect::<Vec<_>>().join(", ")),
                "Return generic error messages in production."
            ).await;
        }
    }
}

async fn check_rate_limiting(client: &reqwest::Client, url: &str, findings: &Findings) {
    let login_url = format!("{url}/api/auth/login");

    for _ in 0..8 {
        let resp = match client
            .post(&login_url)
            .json(&serde_json::json!({"password": "wrong-password-test"}))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        if resp.status().as_u16() == 429 { return; }
        if is_down(resp.status().as_u16()) { return; }
    }

    add(findings, Severity::Medium, "A07:Security Misconfiguration",
        "Login rate limiting may be insufficient",
        "8 rapid login attempts did not trigger rate limiting (expected 429).",
        "Set LOGIN_RATE_LIMIT_MAX=5 and LOGIN_RATE_LIMIT_WINDOW_SECS=300."
    ).await;
}

async fn check_tls(_client: &reqwest::Client, url: &str, findings: &Findings) {
    if !url.starts_with("https://") {
        add(findings, Severity::High, "A02:Cryptographic Failures",
            "No TLS/HTTPS",
            &format!("{url} uses plain HTTP."),
            "Enable TLS (e.g. via Cloudflare or Let's Encrypt)."
        ).await;
        return;
    }

    // HTTP -> HTTPS redirect check
    let http_url = url.replace("https://", "http://");
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    match http_client.get(&http_url).send().await {
        Ok(r) => {
            let status = r.status().as_u16();
            if !(301..=308).contains(&status) {
                add(findings, Severity::Medium, "A02:Cryptographic Failures",
                    &format!("No HTTP->HTTPS redirect for {}", url.split("//").last().unwrap_or(url)),
                    &format!("HTTP returns {status} instead of 301/308 redirect."),
                    "Configure HTTP->HTTPS redirect."
                ).await;
            } else {
                let loc = r.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
                if !loc.starts_with("https://") {
                    add(findings, Severity::Medium, "A02:Cryptographic Failures",
                        "HTTP redirect not to HTTPS",
                        &format!("Redirects to {loc} (not HTTPS)."),
                        "Redirect to HTTPS URL."
                    ).await;
                }
            }
        }
        Err(_) => {}
    }
}

async fn check_ssl_cert(url: &str, findings: &Findings) {
    if !url.starts_with("https://") { return; }

    // Try to connect and check the certificate
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    // If the connection succeeds, cert is valid. We mainly check here
    // that it doesn't fail with a cert error.
    match client.get(url).send().await {
        Ok(_) => {
            // Connection OK, cert is valid
        }
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("certificate") || err_str.contains("ssl") || err_str.contains("tls") {
                add(findings, Severity::Critical, "A02:Cryptographic Failures",
                    "SSL/TLS certificate error",
                    &format!("Certificate validation failed: {e}"),
                    "Renew or fix the SSL certificate."
                ).await;
            }
        }
    }
}

async fn check_common_paths(client: &reqwest::Client, url: &str, findings: &Findings) {
    let sensitive = [
        ("/.env", "Environment variables", Severity::Critical),
        ("/.git/config", "Git config", Severity::Critical),
        ("/.git/HEAD", "Git HEAD", Severity::Critical),
        ("/data/deskbot.db", "SQLite database", Severity::Critical),
        ("/.DS_Store", "macOS metadata", Severity::Low),
        ("/api/debug", "Debug endpoint", Severity::High),
        ("/server-status", "Server status", Severity::Medium),
        ("/actuator", "Actuator", Severity::Medium),
        ("/actuator/health", "Actuator health", Severity::Medium),
        ("/wp-admin", "WordPress admin", Severity::Medium),
        ("/robots.txt", "Robots file", Severity::Info),
        ("/sitemap.xml", "Sitemap", Severity::Info),
        ("/.well-known/security.txt", "Security policy", Severity::Info),
        ("/backup.sql", "SQL backup", Severity::Critical),
        ("/dump.sql", "SQL dump", Severity::Critical),
        ("/config.json", "Config file", Severity::High),
        ("/package.json", "Package manifest", Severity::Medium),
        ("/Cargo.toml", "Rust manifest", Severity::Medium),
        ("/docker-compose.yml", "Docker compose", Severity::High),
    ];

    // Fetch in parallel
    let mut handles = Vec::new();
    for (path, desc, sev) in &sensitive {
        let c = client.clone();
        let full = format!("{url}{path}");
        let path = path.to_string();
        let desc = desc.to_string();
        let sev = *sev;
        let f = findings.clone();

        handles.push(tokio::spawn(async move {
            let resp = match c.get(&full).send().await {
                Ok(r) => r,
                Err(_) => return,
            };

            if resp.status().as_u16() == 200 {
                let len = resp.content_length().unwrap_or(0);
                if len > 0 && sev != Severity::Info {
                    add(&f, sev, "A01:Broken Access Control",
                        &format!("Sensitive path exposed: {path}"),
                        &format!("{desc} accessible — HTTP 200, {len} bytes"),
                        &format!("Block access to {path} via reverse proxy.")
                    ).await;
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
}

async fn check_injection_vectors(client: &reqwest::Client, url: &str, findings: &Findings) {
    // SQL injection
    let sqli_payloads = [
        "/api/devices?id=' OR 1=1--",
        "/api/devices?id=1; DROP TABLE devices--",
        "/api/devices?id=1' UNION SELECT null,null,null--",
    ];

    for payload in &sqli_payloads {
        let resp = match client.get(&format!("{url}{payload}")).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        if is_down(resp.status().as_u16()) { return; }

        let body = resp.text().await.unwrap_or_default().to_lowercase();
        if body.contains("sqlite") || body.contains("syntax error") || body.contains("sql")
            || body.contains("unrecognized token") {
            add(findings, Severity::High, "A03:Injection",
                "SQL injection indicator",
                &format!("{payload} => SQL error leaked in response"),
                "Use parameterized queries. Don't expose SQL errors."
            ).await;
        }
    }

    // XSS reflection
    let xss_payloads = [
        ("/api/devices?name=<script>alert(1)</script>", "<script>alert(1)</script>"),
        ("/api/devices?name=\"onmouseover=alert(1)", "onmouseover=alert"),
    ];

    for (path, marker) in &xss_payloads {
        if let Ok(resp) = client.get(&format!("{url}{path}")).send().await {
            if !is_down(resp.status().as_u16()) {
                let body = resp.text().await.unwrap_or_default();
                if body.contains(marker) {
                    add(findings, Severity::High, "A03:Injection",
                        "XSS reflected in response",
                        &format!("Input reflected unescaped: {marker}"),
                        "Sanitize and encode all output."
                    ).await;
                }
            }
        }
    }

    // NoSQL injection
    if let Ok(resp) = client
        .post(&format!("{url}/api/auth/login"))
        .json(&serde_json::json!({"password": {"$gt": ""}}))
        .send()
        .await
    {
        if resp.status().is_success() && !is_down(resp.status().as_u16()) {
            add(findings, Severity::Critical, "A03:Injection",
                "NoSQL injection in login",
                "Login accepted operator object as password.",
                "Validate input types strictly."
            ).await;
        }
    }
}

async fn check_open_redirect(client: &reqwest::Client, url: &str, findings: &Findings) {
    let payloads = [
        "/auth/callback?redirect=https://evil.com",
        "/auth/callback?next=https://evil.com",
        "/auth/callback?return_to=//evil.com",
    ];

    for path in &payloads {
        let resp = match client.get(&format!("{url}{path}")).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if is_down(resp.status().as_u16()) { return; }

        if (301..=308).contains(&resp.status().as_u16()) {
            let loc = resp.headers().get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if loc.contains("evil.com") {
                add(findings, Severity::Medium, "A01:Broken Access Control",
                    "Open redirect vulnerability",
                    &format!("{path} redirects to {loc}"),
                    "Validate redirect URLs against an allowlist."
                ).await;
            }
        }
    }
}

async fn check_path_traversal(client: &reqwest::Client, url: &str, findings: &Findings) {
    let payloads = [
        "/api/firmware/../../../etc/passwd",
        "/api/firmware/..%2F..%2F..%2Fetc%2Fpasswd",
        "/api/firmware/%2e%2e/%2e%2e/%2e%2e/etc/passwd",
    ];

    for path in &payloads {
        let resp = match client.get(&format!("{url}{path}")).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if is_down(resp.status().as_u16()) { return; }

        if resp.status().as_u16() == 200 {
            let body = resp.text().await.unwrap_or_default();
            if body.contains("root:") || body.contains("/bin/") {
                add(findings, Severity::Critical, "A01:Broken Access Control",
                    "Path traversal vulnerability",
                    &format!("{path} returned file system content"),
                    "Sanitize file paths. Block .. sequences."
                ).await;
            }
        }
    }
}

async fn check_idor(client: &reqwest::Client, url: &str, findings: &Findings) {
    // Try accessing sequential device IDs without auth
    let ids = ["1", "2", "3", "0", "-1"];

    for id in &ids {
        let resp = match client.get(&format!("{url}/api/devices/{id}")).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if is_down(resp.status().as_u16()) { return; }

        // If we get 200 without auth, that's an auth issue (caught elsewhere).
        // If we get 200 WITH auth, we'd need to check ownership.
        // For unauthenticated scanning, we check the response isn't leaking data.
        if resp.status().as_u16() == 200 {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if body.get("ip").is_some() || body.get("api_key").is_some() {
                add(findings, Severity::High, "A01:Broken Access Control",
                    &format!("IDOR: Device {id} data accessible without auth"),
                    "Device details including IP/keys returned without authentication.",
                    "Ensure device endpoints require authentication and ownership."
                ).await;
            }
        }
    }
}
