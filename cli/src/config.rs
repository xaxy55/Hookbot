use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

struct ConfigCheck {
    key: &'static str,
    required: bool,
    is_secret: bool,
    description: &'static str,
    check: Option<fn(&str) -> Option<String>>,
}

pub async fn run(env_file: &str, json: bool) -> Result<(), String> {
    if !json {
        println!("{}", "=== Configuration Audit ===".bold());
        println!();
    }

    let path = Path::new(env_file);
    if !path.exists() {
        return Err(format!("{env_file} not found. Run from the project root."));
    }

    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let vars = parse_env(&content);

    let checks: Vec<ConfigCheck> = vec![
        ConfigCheck {
            key: "API_KEY",
            required: true,
            is_secret: true,
            description: "API authentication key",
            check: Some(|v| {
                if v.len() < 16 { Some("Too short (min 16 chars recommended)".into()) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "ADMIN_PASSWORD",
            required: false,
            is_secret: true,
            description: "Admin login password",
            check: Some(|v| {
                if v.len() < 12 { Some("Weak password (min 12 chars recommended)".into()) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "ADMIN_PASSWORD_HASH",
            required: false,
            is_secret: true,
            description: "Bcrypt hash of admin password (preferred over plaintext)",
            check: None,
        },
        ConfigCheck {
            key: "ALLOWED_ORIGINS",
            required: true,
            is_secret: false,
            description: "CORS allowed origins (comma-separated)",
            check: Some(|v| {
                if v == "*" { Some("Wildcard CORS is insecure for production".into()) }
                else if v.is_empty() { Some("Empty — CORS will allow any origin".into()) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "HOOKBOT_SERVER_URL",
            required: false,
            is_secret: false,
            description: "Public server URL",
            check: Some(|v| {
                if v.starts_with("http://") { Some("Using HTTP — should be HTTPS in production".into()) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "TLS_CERT_PATH",
            required: false,
            is_secret: false,
            description: "TLS certificate path",
            check: Some(|v| {
                if !Path::new(v).exists() { Some(format!("File not found: {v}")) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "TLS_KEY_PATH",
            required: false,
            is_secret: false,
            description: "TLS private key path",
            check: Some(|v| {
                if !Path::new(v).exists() { Some(format!("File not found: {v}")) }
                else { None }
            }),
        },
        ConfigCheck {
            key: "WORKOS_API_KEY",
            required: false,
            is_secret: true,
            description: "WorkOS API key for multi-tenant auth",
            check: None,
        },
        ConfigCheck {
            key: "WORKOS_CLIENT_ID",
            required: false,
            is_secret: false,
            description: "WorkOS client ID",
            check: None,
        },
        ConfigCheck {
            key: "OPENAI_API_KEY",
            required: false,
            is_secret: true,
            description: "OpenAI API key",
            check: None,
        },
        ConfigCheck {
            key: "ANTHROPIC_API_KEY",
            required: false,
            is_secret: true,
            description: "Anthropic API key",
            check: None,
        },
        ConfigCheck {
            key: "DOCKERHUB_TOKEN",
            required: false,
            is_secret: true,
            description: "Docker Hub access token",
            check: None,
        },
        ConfigCheck {
            key: "CLOUDFLARE_API_TOKEN",
            required: false,
            is_secret: true,
            description: "Cloudflare API token",
            check: None,
        },
        ConfigCheck {
            key: "LOGIN_RATE_LIMIT_MAX",
            required: false,
            is_secret: false,
            description: "Max login attempts per window",
            check: Some(|v| {
                if let Ok(n) = v.parse::<u32>() {
                    if n > 10 { Some(format!("Rate limit too lenient ({n}). Recommend <= 5.")) }
                    else { None }
                } else {
                    Some("Invalid number".into())
                }
            }),
        },
        ConfigCheck {
            key: "DATABASE_URL",
            required: false,
            is_secret: false,
            description: "SQLite database path",
            check: None,
        },
    ];

    let mut warnings = 0;
    let mut errors = 0;

    println!("  {:<28} {:<10} {}", "Variable", "Status", "Notes");
    println!("  {}", "-".repeat(70));

    for check in &checks {
        let value = vars.get(check.key);
        let display_val = match (value, check.is_secret) {
            (Some(v), true) => {
                if v.len() > 4 {
                    format!("{}****", &v[..4])
                } else {
                    "****".into()
                }
            }
            (Some(v), false) => v.clone(),
            (None, _) => String::new(),
        };

        let (status, note) = match value {
            None if check.required => {
                errors += 1;
                ("MISSING".red().to_string(), format!("{} (required)", check.description))
            }
            None => {
                ("—".dimmed().to_string(), check.description.to_string())
            }
            Some(v) => {
                if let Some(checker) = check.check {
                    if let Some(warning) = checker(v) {
                        warnings += 1;
                        ("WARN".yellow().to_string(), warning)
                    } else {
                        ("OK".green().to_string(), display_val)
                    }
                } else {
                    ("SET".green().to_string(), display_val)
                }
            }
        };

        println!("  {:<28} {:<10} {}", check.key, status, note.dimmed());
    }

    // Git history check
    println!();
    print!("  .env in git history: ");
    let output = std::process::Command::new("git")
        .args(["log", "--all", "--oneline", "--", ".env"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                println!("{}", "clean".green());
            } else {
                let commits: Vec<&str> = stdout.lines().collect();
                println!("{} — found in {} commit(s)!", "LEAKED".red().bold(), commits.len());
                for c in &commits[..commits.len().min(5)] {
                    println!("    {c}");
                }
                println!();
                println!(
                    "  {} Secrets were committed to git. Rotate ALL credentials and use",
                    "!!!".red().bold()
                );
                println!("      git-filter-repo or BFG Repo-Cleaner to purge from history.");
                errors += 1;
            }
        }
        Err(_) => println!("{}", "could not check (git not available)".dimmed()),
    }

    // .gitignore check
    print!("  .env in .gitignore:  ");
    let gitignore = std::fs::read_to_string(".gitignore").unwrap_or_default();
    if gitignore.lines().any(|l| l.trim() == ".env") {
        println!("{}", "yes".green());
    } else {
        println!("{}", "NO — add .env to .gitignore!".red().bold());
        errors += 1;
    }

    println!();
    if errors > 0 {
        println!(
            "  {} {} error(s), {} warning(s)",
            "FAIL".red().bold(),
            errors,
            warnings
        );
    } else if warnings > 0 {
        println!(
            "  {} {} warning(s)",
            "WARN".yellow().bold(),
            warnings
        );
    } else {
        println!("  {} Configuration looks good.", "PASS".green().bold());
    }

    Ok(())
}

fn parse_env(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}
