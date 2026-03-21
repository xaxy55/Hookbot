mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod services;

use axum::routing::{delete, get, post, put};
use axum::{middleware, Extension, Router};
use axum::response::Redirect;
use axum::http::header;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;
use std::net::SocketAddr;
use std::sync::Arc;

use config::AppConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env();
    info!("Starting hookbot server on {}", config.bind_addr);

    let pool = db::init(&config.database_url);
    info!("Database initialized at {:?}", config.database_url);

    // Start background services
    services::device_poller::start(pool.clone(), config.poll_interval_secs, config.log_retention_hours);

    // Auto-discover devices on startup
    let discover_db = pool.clone();
    let mdns_prefix = config.mdns_prefix.clone();
    tokio::spawn(async move {
        routes::discovery::discover_and_register(&discover_db, &mdns_prefix).await;
    });

    // Create rate limiter for login
    let login_rate_limiter = auth::LoginRateLimiter::new(
        config.login_rate_limit_max,
        config.login_rate_limit_window_secs,
    );
    info!(
        "Login rate limit: {} attempts per {} seconds",
        config.login_rate_limit_max, config.login_rate_limit_window_secs
    );

    let config = Arc::new(config);

    // CORS: use explicit origins if configured, otherwise fall back to mirror (permissive)
    let cors = {
        let allow_origin = if config.allowed_origins.is_empty() {
            AllowOrigin::mirror_request()
        } else {
            let origins: Vec<header::HeaderValue> = config
                .allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            AllowOrigin::list(origins)
        };

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::COOKIE,
                header::HeaderName::from_static("x-api-key"),
            ])
            .allow_credentials(true)
    };

    // Public routes — no auth required
    let public_routes = Router::new()
        .route("/api/health", get(|| async {
            axum::Json(serde_json::json!({
                "status": "ok",
                "uptime_secs": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }))
        }))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/status", get(auth::auth_status))
        .route("/auth/login", get(auth::workos_login))
        .route("/auth/callback", get(auth::workos_callback));

    // Device routes use just the pool
    let device_routes = Router::new()
        .route("/api/devices", get(routes::devices::list_devices).post(routes::devices::create_device))
        .route("/api/devices/{id}", get(routes::devices::get_device).put(routes::devices::update_device).delete(routes::devices::delete_device))
        .route("/api/devices/{id}/state", post(routes::devices::forward_state))
        .route("/api/devices/{id}/tasks", post(routes::devices::forward_tasks))
        .route("/api/devices/{id}/servos", get(routes::devices::get_servos).post(routes::devices::forward_servos))
        .route("/api/devices/{id}/servos/config", post(routes::devices::forward_servo_config))
        .route("/api/devices/{id}/status", get(routes::devices::get_device_status))
        .route("/api/devices/{id}/history", get(routes::devices::get_device_history))
        .route("/api/devices/{id}/config", get(routes::devices::get_config).put(routes::devices::update_config))
        .route("/api/devices/{id}/config/push", post(routes::devices::push_config))
        .route("/api/devices/{id}/config/export", get(routes::devices::export_config))
        .route("/api/devices/{id}/config/import", post(routes::devices::import_config))
        .route("/api/devices/{id}/animation", post(routes::devices::forward_animation))
        .route("/api/devices/{id}/animation/stop", post(routes::devices::stop_animation))
        .route("/api/devices/{id}/notifications", get(routes::notifications::get_notifications).post(routes::notifications::forward_notification))
        .route("/api/devices/{id}/notifications/{nid}", delete(routes::notifications::delete_notification))
        .route("/api/devices/{id}/sensors", get(routes::sensors::get_sensors).put(routes::sensors::update_sensors))
        .route("/api/devices/{id}/sensors/readings", get(routes::sensors::get_sensor_readings).delete(routes::sensors::delete_sensor_readings))
        .route("/api/devices/{id}/sensors/readings/latest", get(routes::sensors::get_latest_readings))
        .route("/api/devices/{id}/rules", get(routes::automation::list_rules).post(routes::automation::create_rule))
        .route("/api/devices/{id}/rules/{rule_id}", put(routes::automation::update_rule).delete(routes::automation::delete_rule))
        .route("/api/context", get(routes::context::get_context))
        .route("/api/notifications/webhook", post(routes::notifications::webhook_notification))
        .route("/api/discovery", get(routes::discovery::scan))
        .route("/api/diagnostics", get(routes::diagnostics::run_diagnostics))
        .route("/api/hook", post(routes::hooks::handle_hook))
        .route("/api/hook/github", post(routes::github::handle_github_hook))
        .route("/api/gamification/stats", get(routes::gamification::get_stats))
        .route("/api/gamification/activity", get(routes::gamification::get_activity))
        .route("/api/gamification/analytics", get(routes::gamification::get_analytics))
        .route("/api/gamification/achievements", get(routes::gamification::get_achievements))
        .route("/api/gamification/leaderboard", get(routes::gamification::get_leaderboard))
        .route("/api/gamification/streaks", get(routes::gamification::get_streaks))
        .with_state(pool.clone());

    // Firmware/OTA/Build routes use both pool and config
    let firmware_routes = Router::new()
        .route("/api/firmware", post(routes::firmware::upload_firmware).get(routes::firmware::list_firmware))
        .route("/api/firmware/{id}/binary", get(routes::firmware::serve_firmware_binary))
        .route("/api/firmware/build", post(routes::build::build_firmware))
        .route("/api/ota/deploy", post(routes::ota::deploy))
        .route("/api/ota/jobs", get(routes::ota::list_jobs))
        .with_state((pool.clone(), (*config).clone()));

    // Settings & log management routes
    let settings_routes = Router::new()
        .route("/api/settings", get(routes::settings::get_settings).put(routes::settings::update_settings))
        .route("/api/logs/stats", get(routes::settings::get_log_stats))
        .route("/api/logs/prune", delete(routes::settings::prune_logs))
        .with_state((pool.clone(), (*config).clone()));

    // Store routes
    let store_routes = Router::new()
        .route("/api/store", get(routes::store::list_items))
        .route("/api/store/buy", post(routes::store::buy_item))
        .route("/api/store/owned", get(routes::store::owned_items))
        .with_state(pool.clone());

    // Pet / token tracking routes
    let pet_routes = Router::new()
        .route("/api/pet", get(routes::pet::get_pet_state))
        .route("/api/pet/feed", post(routes::pet::feed_pet))
        .route("/api/pet/pet", post(routes::pet::pet_pet))
        .route("/api/pet/tokens", get(routes::pet::get_token_usage).post(routes::pet::record_token_usage))
        .with_state(pool.clone());

    // Mood journal routes
    let mood_routes = Router::new()
        .route("/api/mood", get(routes::mood::get_entries).post(routes::mood::create_entry))
        .route("/api/mood/stats", get(routes::mood::get_stats))
        .with_state(pool.clone());

    // Community plugin store routes
    let community_routes = Router::new()
        .route("/api/community/plugins", get(routes::community_store::list_plugins).post(routes::community_store::publish_plugin))
        .route("/api/community/plugins/{id}/install", post(routes::community_store::install_plugin).delete(routes::community_store::uninstall_plugin))
        .route("/api/community/plugins/{id}/rate", post(routes::community_store::rate_plugin))
        .route("/api/community/assets", get(routes::shared_assets::list_assets).post(routes::shared_assets::publish_asset))
        .route("/api/community/assets/{id}/install", post(routes::shared_assets::install_asset).delete(routes::shared_assets::uninstall_asset))
        .route("/api/community/assets/{id}/rate", post(routes::shared_assets::rate_asset))
        .route("/api/community/publishers", get(routes::community_store::list_publishers).post(routes::community_store::add_publisher))
        .route("/api/community/publishers/{id}", delete(routes::community_store::remove_publisher))
        .with_state(pool.clone());

    // Project routing routes
    let project_route_routes = Router::new()
        .route("/api/routes", get(routes::project_routes::list_routes).post(routes::project_routes::create_route))
        .route("/api/routes/{id}", put(routes::project_routes::update_route).delete(routes::project_routes::delete_route))
        .with_state(pool.clone());

    // Device group routes
    let group_routes = Router::new()
        .route("/api/groups", get(routes::groups::list_groups).post(routes::groups::create_group))
        .route("/api/groups/{id}", put(routes::groups::update_group).delete(routes::groups::delete_group))
        .route("/api/groups/{id}/members", post(routes::groups::add_member))
        .route("/api/groups/{id}/members/{device_id}", delete(routes::groups::remove_member))
        .route("/api/groups/{id}/state", post(routes::groups::send_group_state))
        .route("/api/groups/{id}/command", post(routes::groups::send_group_command))
        .with_state(pool.clone());

    // Sandbox routes (plugin permission scoping)
    let sandbox_routes = Router::new()
        .route("/api/community/sandboxes", get(routes::sandbox::list_sandboxes).post(routes::sandbox::create_sandbox))
        .route("/api/community/sandboxes/{id}", put(routes::sandbox::update_sandbox).delete(routes::sandbox::delete_sandbox))
        .route("/api/community/sandboxes/check", post(routes::sandbox::check_permission))
        .with_state(pool.clone());

    // Device-to-device communication routes
    let device_link_routes = Router::new()
        .route("/api/device-links", get(routes::device_links::list_links).post(routes::device_links::create_link))
        .route("/api/device-links/{id}", put(routes::device_links::update_link).delete(routes::device_links::delete_link))
        .with_state(pool.clone());

    // User management routes
    let user_routes = Router::new()
        .route("/api/users", get(routes::users::list_users).post(routes::users::create_user))
        .route("/api/users/{id}", get(routes::users::get_user).put(routes::users::update_user).delete(routes::users::delete_user))
        .route("/api/users/{id}/devices", post(routes::users::assign_device))
        .route("/api/users/{id}/devices/{device_id}", delete(routes::users::unassign_device))
        .with_state(pool.clone());

    // Tunnel / remote access routes
    let tunnel_routes = Router::new()
        .route("/api/tunnels", get(routes::tunnels::list_tunnels).post(routes::tunnels::create_tunnel))
        .route("/api/tunnels/{id}", get(routes::tunnels::get_tunnel).put(routes::tunnels::update_tunnel).delete(routes::tunnels::delete_tunnel))
        .route("/api/tunnels/{id}/start", post(routes::tunnels::start_tunnel))
        .route("/api/tunnels/{id}/stop", post(routes::tunnels::stop_tunnel))
        .with_state(pool.clone());

    // Mood learning routes
    let mood_learning_routes = Router::new()
        .route("/api/mood/feedback", post(routes::mood_learning::record_feedback))
        .route("/api/mood/preferences", get(routes::mood_learning::get_preferences))
        .route("/api/mood/patterns", get(routes::mood_learning::get_patterns))
        .route("/api/mood/suggest", get(routes::mood_learning::get_suggestion))
        .with_state(pool.clone());

    // Social & Multiplayer routes (Phase 7)
    let social_routes = Router::new()
        .route("/api/social/buddies", get(routes::social::list_buddies).post(routes::social::create_buddy))
        .route("/api/social/buddies/{id}/accept", post(routes::social::accept_buddy))
        .route("/api/social/buddies/{id}", delete(routes::social::delete_buddy))
        .route("/api/social/raids", get(routes::social::list_raids).post(routes::social::create_raid))
        .route("/api/social/shared-streaks", get(routes::social::list_shared_streaks).post(routes::social::create_shared_streak))
        .route("/api/social/shared-streaks/{id}", delete(routes::social::delete_shared_streak))
        .route("/api/social/presence", get(routes::social::list_presence).post(routes::social::update_presence))
        .route("/api/social/team", get(routes::social::get_team_dashboard))
        .route("/api/social/reactions", get(routes::social::list_reactions).post(routes::social::send_reaction))
        .route("/api/social/events", get(routes::social::list_global_events).post(routes::social::create_global_event))
        .with_state(pool.clone());

    // Voice control routes (need pool + config for API keys)
    let voice_routes = Router::new()
        .route("/api/voice/transcribe", post(routes::voice::transcribe))
        .route("/api/voice/tts", post(routes::voice::text_to_speech))
        .route("/api/voice/command", post(routes::voice::voice_command))
        .route("/api/voice/history", get(routes::voice::get_history))
        .route("/api/voice/config", get(routes::voice::get_config).put(routes::voice::update_config))
        .with_state((pool.clone(), (*config).clone()));

    // Auth management routes (protected — require current API key)
    let auth_mgmt_routes = Router::new()
        .route("/api/auth/rotate-key", post(auth::rotate_api_key))
        .route("/api/auth/me", get(auth::get_me));

    // Protected routes — require API key or session cookie
    let protected_routes = Router::new()
        .merge(device_routes)
        .merge(firmware_routes)
        .merge(settings_routes)
        .merge(store_routes)
        .merge(pet_routes)
        .merge(mood_routes)
        .merge(community_routes)
        .merge(project_route_routes)
        .merge(group_routes)
        .merge(sandbox_routes)
        .merge(device_link_routes)
        .merge(user_routes)
        .merge(tunnel_routes)
        .merge(mood_learning_routes)
        .merge(voice_routes)
        .merge(social_routes)
        .merge(auth_mgmt_routes)
        .route_layer(middleware::from_fn(auth::require_auth));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(Extension(pool.clone()))
        .layer(Extension(login_rate_limiter))
        .layer(Extension(config.clone()))
        .layer(cors);

    // If TLS cert and key are provided, serve HTTPS on 443 + HTTP redirect on 80
    if let (Some(cert_path), Some(key_path)) = (&config.tls_cert_path, &config.tls_key_path) {
        info!("TLS enabled: cert={}, key={}", cert_path, key_path);

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .expect("Failed to load TLS cert/key");

        // Spawn HTTP->HTTPS redirect on port 80
        let redirect_app = Router::new().fallback(|req: axum::extract::Request| async move {
            let host = req.headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost")
                .split(':')
                .next()
                .unwrap_or("localhost");
            let uri = req.uri();
            let redirect_url = format!("https://{}{}", host, uri);
            Redirect::permanent(&redirect_url)
        });
        tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], 80));
            info!("HTTP redirect listening on {}", addr);
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, redirect_app).await.unwrap();
        });

        // Serve HTTPS on 443
        let addr = SocketAddr::from(([0, 0, 0, 0], 443));
        info!("HTTPS server listening on {}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        // Plain HTTP
        let listener = tokio::net::TcpListener::bind(&config.bind_addr).await.unwrap();
        info!("Server listening on {}", config.bind_addr);
        axum::serve(listener, app).await.unwrap();
    }
}
