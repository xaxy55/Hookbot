use colored::Colorize;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Claude Code events the Hookbot hook script listens for.
const HOOK_EVENTS: [&str; 4] = ["PreToolUse", "PostToolUse", "UserPromptSubmit", "Stop"];

/// Substrings that identify a hook command as ours, so re-installing replaces
/// the old entry instead of stacking a duplicate next to it.
const HOOK_MARKERS: [&str; 2] = ["hookbot-hook", "deskbot-hook"];

/// Filename of the hook script shipped in `hooks/`.
const HOOK_SCRIPT_NAME: &str = "deskbot-hook.js";

pub enum HookAction {
    Install {
        /// Write to `.claude/settings.json` in the current project instead of `~/.claude`.
        project: bool,
        /// Explicit settings file path (overrides the scope flags).
        settings: Option<String>,
        /// Explicit path to the hook script.
        script: Option<String>,
        /// Bind the hooks to a specific device ID.
        device: Option<String>,
    },
}

pub async fn run(
    action: HookAction,
    base: &str,
    key: Option<&str>,
    json: bool,
) -> Result<(), String> {
    match action {
        HookAction::Install {
            project,
            settings,
            script,
            device,
        } => install(project, settings, script, device, base, key, json),
    }
}

#[allow(clippy::too_many_arguments)]
fn install(
    project: bool,
    settings: Option<String>,
    script: Option<String>,
    device: Option<String>,
    base: &str,
    key: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let script_path = match script {
        Some(s) => {
            let p = PathBuf::from(&s);
            if !p.exists() {
                return Err(format!("Hook script not found: {s}"));
            }
            absolute(&p)
        }
        None => find_hook_script()?,
    };
    let script_str = script_path.to_string_lossy().to_string();

    let settings_path = match settings {
        Some(s) => PathBuf::from(s),
        None if project => std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(".claude")
            .join("settings.json"),
        None => PathBuf::from(home_dir()?).join(".claude").join("settings.json"),
    };

    if !json {
        println!("{}", "=== Claude Code Hook Setup ===".bold());
        println!();
        println!("  Script:    {}", script_str.dimmed());
        println!("  Settings:  {}", settings_path.display().to_string().dimmed());
        println!("  Server:    {}", base.dimmed());
    }

    // Read existing settings (an empty or missing file starts from `{}`).
    let existing = std::fs::read_to_string(&settings_path).unwrap_or_default();
    let mut doc: Value = if existing.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&existing)
            .map_err(|e| format!("{} is not valid JSON: {e}", settings_path.display()))?
    };

    // Back up before touching anything the user already had.
    let mut backup: Option<PathBuf> = None;
    if !existing.is_empty() {
        let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
        let mut path = with_suffix(&settings_path, &format!(".bak.{stamp}"));
        // Two runs in the same second must not clobber the earlier backup.
        let mut n = 1;
        while path.exists() {
            path = with_suffix(&settings_path, &format!(".bak.{stamp}-{n}"));
            n += 1;
        }
        std::fs::copy(&settings_path, &path)
            .map_err(|e| format!("Failed to back up {}: {e}", settings_path.display()))?;
        backup = Some(path);
    }

    let added = merge_hook_entries(&mut doc, &script_str);

    if let Some(dir) = settings_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let rendered = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, format!("{rendered}\n"))
        .map_err(|e| format!("Failed to write {}: {e}", settings_path.display()))?;

    // The hook script reads its own JSON config: `.hookbot` in the project it
    // runs from, otherwise `hookbot-config.json` next to the script.
    let hook_config_path = if project {
        std::env::current_dir().map_err(|e| e.to_string())?.join(".hookbot")
    } else {
        script_path
            .parent()
            .ok_or("Could not resolve the hook script directory")?
            .join("hookbot-config.json")
    };
    write_hook_config(&hook_config_path, base, key, device.as_deref())?;

    if json {
        let report = json!({
            "settings": settings_path.to_string_lossy(),
            "backup": backup.as_ref().map(|p| p.to_string_lossy()),
            "hook_config": hook_config_path.to_string_lossy(),
            "script": script_str,
            "events": added,
            "scope": if project { "project" } else { "user" },
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return Ok(());
    }

    if let Some(path) = backup {
        println!("  Backup:    {}", path.display().to_string().dimmed());
    }
    println!("  Config:    {}", hook_config_path.display().to_string().dimmed());
    println!();
    println!("  Registered hooks:");
    for event in &added {
        println!("    {} {event}", "OK".green().bold());
    }
    println!();
    if key.is_none() {
        println!(
            "  {} No API key stored — run {} first if your server requires auth.",
            "!".yellow().bold(),
            "hookbot login --url <url>".cyan(),
        );
        println!();
    }
    println!("  Restart Claude Code (or start a new session) to pick up the hooks.");

    Ok(())
}

/// Merge the Hookbot hook entries into a Claude Code settings document.
///
/// Existing settings — including other people's hooks — are preserved. Any
/// previous Hookbot entry for an event is replaced rather than duplicated, so
/// running this repeatedly is a no-op after the first time. Returns the events
/// that were wired up.
pub fn merge_hook_entries(settings: &mut Value, script: &str) -> Vec<String> {
    if !settings.is_object() {
        *settings = Value::Object(Map::new());
    }
    let root = settings.as_object_mut().expect("settings is an object");

    let hooks = root
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !hooks.is_object() {
        *hooks = Value::Object(Map::new());
    }
    let hooks = hooks.as_object_mut().expect("hooks is an object");

    let mut added = Vec::new();
    for event in HOOK_EVENTS {
        let existing = hooks
            .get(event)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut matchers: Vec<Value> = existing
            .into_iter()
            .filter(|m| !is_hookbot_matcher(m))
            .collect();

        matchers.push(json!({
            "hooks": [{
                "type": "command",
                "command": format!("node {} {}", shell_quote(script), event),
            }],
        }));

        hooks.insert(event.to_string(), Value::Array(matchers));
        added.push(event.to_string());
    }

    added
}

/// True if any command inside this matcher belongs to Hookbot.
fn is_hookbot_matcher(matcher: &Value) -> bool {
    matcher
        .get("hooks")
        .and_then(|v| v.as_array())
        .map(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| HOOK_MARKERS.iter().any(|m| c.contains(m)))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Quote a path for the shell only when it needs it — an unquoted path keeps
/// the settings file readable, which matters when users hand-edit it.
fn shell_quote(path: &str) -> String {
    let safe = path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '~'));
    if safe {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', r"'\''"))
    }
}

/// Write the JSON config the hook script reads, merging into whatever is there
/// so a hand-set `device_id` or `mode` survives.
fn write_hook_config(
    path: &Path,
    base: &str,
    key: Option<&str>,
    device: Option<&str>,
) -> Result<(), String> {
    let mut doc: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| json!({}));
    if !doc.is_object() {
        doc = json!({});
    }
    let obj = doc.as_object_mut().expect("config is an object");

    obj.insert("host".into(), json!(base));
    obj.insert("mode".into(), json!("server"));
    if let Some(k) = key {
        obj.insert("api_key".into(), json!(k));
    }
    if let Some(d) = device {
        obj.insert("device_id".into(), json!(d));
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let rendered = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(path, format!("{rendered}\n"))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    // The config can hold an API key — keep it owner-only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Walk up from the working directory looking for `hooks/deskbot-hook.js`.
fn find_hook_script() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let mut dir = cwd.as_path();
    loop {
        let candidate = dir.join("hooks").join(HOOK_SCRIPT_NAME);
        if candidate.exists() {
            return Ok(absolute(&candidate));
        }
        let direct = dir.join(HOOK_SCRIPT_NAME);
        if direct.exists() {
            return Ok(absolute(&direct));
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    Err(format!(
        "Could not find hooks/{HOOK_SCRIPT_NAME}. Run this from the Hookbot checkout or pass --script <path>."
    ))
}

fn absolute(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME is not set".to_string())
}

/// `foo/settings.json` + `.bak.123` -> `foo/settings.json.bak.123`
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = "/home/dev/Hookbot/hooks/deskbot-hook.js";

    fn commands(settings: &Value, event: &str) -> Vec<String> {
        settings["hooks"][event]
            .as_array()
            .expect("event array")
            .iter()
            .flat_map(|m| m["hooks"].as_array().cloned().unwrap_or_default())
            .map(|h| h["command"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    #[test]
    fn installs_every_event_from_empty_settings() {
        let mut settings = json!({});
        let added = merge_hook_entries(&mut settings, SCRIPT);

        assert_eq!(added, HOOK_EVENTS.to_vec());
        for event in HOOK_EVENTS {
            assert_eq!(
                commands(&settings, event),
                vec![format!("node {SCRIPT} {event}")],
            );
        }
    }

    #[test]
    fn is_idempotent() {
        let mut once = json!({});
        merge_hook_entries(&mut once, SCRIPT);
        let mut twice = once.clone();
        merge_hook_entries(&mut twice, SCRIPT);
        merge_hook_entries(&mut twice, SCRIPT);

        assert_eq!(once, twice, "re-running must not change the settings");
        for event in HOOK_EVENTS {
            assert_eq!(commands(&twice, event).len(), 1, "{event} duplicated");
        }
    }

    #[test]
    fn preserves_unrelated_settings_keys() {
        let mut settings = json!({
            "model": "opus",
            "permissions": { "allow": ["Bash(ls:*)"] },
            "env": { "FOO": "bar" },
        });
        merge_hook_entries(&mut settings, SCRIPT);

        assert_eq!(settings["model"], json!("opus"));
        assert_eq!(settings["permissions"]["allow"], json!(["Bash(ls:*)"]));
        assert_eq!(settings["env"]["FOO"], json!("bar"));
    }

    #[test]
    fn preserves_other_peoples_hooks() {
        let mut settings = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{ "type": "command", "command": "node /opt/other/guard.js" }],
                }],
                "SessionStart": [{
                    "hooks": [{ "type": "command", "command": "echo hi" }],
                }],
            },
        });
        merge_hook_entries(&mut settings, SCRIPT);

        let pre = commands(&settings, "PreToolUse");
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0], "node /opt/other/guard.js");
        assert!(pre[1].contains("deskbot-hook.js PreToolUse"));

        // An event we do not manage is left exactly as it was.
        assert_eq!(commands(&settings, "SessionStart"), vec!["echo hi"]);
        assert_eq!(settings["hooks"]["PreToolUse"][0]["matcher"], json!("Bash"));
    }

    #[test]
    fn replaces_a_stale_hookbot_entry() {
        let mut settings = json!({
            "hooks": {
                "PostToolUse": [
                    { "hooks": [{ "type": "command", "command": "node /old/path/hookbot-hook.js PostToolUse" }] },
                    { "hooks": [{ "type": "command", "command": "node /old/path/deskbot-hook.js PostToolUse" }] },
                    { "hooks": [{ "type": "command", "command": "make lint" }] },
                ],
            },
        });
        merge_hook_entries(&mut settings, SCRIPT);

        let post = commands(&settings, "PostToolUse");
        assert_eq!(post, vec![
            "make lint".to_string(),
            format!("node {SCRIPT} PostToolUse"),
        ]);
    }

    #[test]
    fn recovers_from_a_non_object_hooks_value() {
        let mut settings = json!({ "hooks": "nonsense", "model": "opus" });
        merge_hook_entries(&mut settings, SCRIPT);

        assert!(settings["hooks"].is_object());
        assert_eq!(settings["model"], json!("opus"));
        assert_eq!(commands(&settings, "Stop").len(), 1);
    }

    #[test]
    fn quotes_paths_that_need_it() {
        let mut settings = json!({});
        merge_hook_entries(&mut settings, "/home/my dev/hooks/deskbot-hook.js");
        assert_eq!(
            commands(&settings, "Stop"),
            vec!["node '/home/my dev/hooks/deskbot-hook.js' Stop"],
        );
    }

    #[test]
    fn backup_suffix_keeps_the_original_name() {
        assert_eq!(
            with_suffix(Path::new("/a/b/settings.json"), ".bak.42"),
            PathBuf::from("/a/b/settings.json.bak.42"),
        );
    }
}
