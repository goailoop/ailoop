//! Doctor checks for `ailoop doctor`.

use ailoop_core::models::Configuration;
use cli_framework::app::context::AppContext;
use cli_framework::doctor::check::{CheckSeverity, DoctorCheck, DoctorFinding, DoctorFuture};

pub struct ConfigFileCheck;

impl DoctorCheck for ConfigFileCheck {
    fn id(&self) -> &'static str {
        "ailoop.config"
    }
    fn title(&self) -> &'static str {
        "Config file"
    }
    fn description(&self) -> Option<&'static str> {
        Some("Checks that ~/.config/ailoop/config.toml exists and is valid")
    }
    fn run(&self, _ctx: &dyn AppContext) -> DoctorFuture {
        Box::pin(async move {
            let config_path = match Configuration::default_config_path() {
                Ok(p) => p,
                Err(e) => {
                    return DoctorFinding {
                        check_id: "ailoop.config".to_string(),
                        title: "Config file".to_string(),
                        severity: CheckSeverity::Error,
                        message: format!("Cannot resolve config path: {}", e),
                        detail: None,
                        remediation: Some("Run: ailoop config --init".to_string()),
                    };
                }
            };

            if !config_path.exists() {
                return DoctorFinding {
                    check_id: "ailoop.config".to_string(),
                    title: "Config file".to_string(),
                    severity: CheckSeverity::Warning,
                    message: format!("Config file not found at {}", config_path.display()),
                    detail: None,
                    remediation: Some("Run: ailoop config --init".to_string()),
                };
            }

            match Configuration::load_from_file(&config_path) {
                Ok(_) => DoctorFinding {
                    check_id: "ailoop.config".to_string(),
                    title: "Config file".to_string(),
                    severity: CheckSeverity::Ok,
                    message: format!("Config loaded from {}", config_path.display()),
                    detail: None,
                    remediation: None,
                },
                Err(e) => DoctorFinding {
                    check_id: "ailoop.config".to_string(),
                    title: "Config file".to_string(),
                    severity: CheckSeverity::Error,
                    message: format!("Config file is invalid TOML: {}", e),
                    detail: None,
                    remediation: Some("Run: ailoop config --init".to_string()),
                },
            }
        })
    }
}

pub struct ServerConnectivityCheck;

impl DoctorCheck for ServerConnectivityCheck {
    fn id(&self) -> &'static str {
        "ailoop.server"
    }
    fn title(&self) -> &'static str {
        "Server connectivity"
    }
    fn description(&self) -> Option<&'static str> {
        Some("Pings the configured AILOOP_SERVER if set")
    }
    fn run(&self, _ctx: &dyn AppContext) -> DoctorFuture {
        Box::pin(async move {
            let server_url = std::env::var("AILOOP_SERVER").unwrap_or_default();

            if server_url.is_empty() {
                return DoctorFinding {
                    check_id: "ailoop.server".to_string(),
                    title: "Server connectivity".to_string(),
                    severity: CheckSeverity::Skipped,
                    message: "AILOOP_SERVER not set; skipping connectivity check".to_string(),
                    detail: None,
                    remediation: Some(
                        "Set AILOOP_SERVER=http://host:port or run: ailoop serve".to_string(),
                    ),
                };
            }

            let health_url = format!("{}/health", server_url.trim_end_matches('/'));
            match reqwest::get(&health_url).await {
                Ok(resp) if resp.status().is_success() => DoctorFinding {
                    check_id: "ailoop.server".to_string(),
                    title: "Server connectivity".to_string(),
                    severity: CheckSeverity::Ok,
                    message: format!("Server reachable at {}", server_url),
                    detail: None,
                    remediation: None,
                },
                Ok(resp) => DoctorFinding {
                    check_id: "ailoop.server".to_string(),
                    title: "Server connectivity".to_string(),
                    severity: CheckSeverity::Warning,
                    message: format!(
                        "Server responded with HTTP {} at {}",
                        resp.status(),
                        server_url
                    ),
                    detail: None,
                    remediation: None,
                },
                Err(e) => DoctorFinding {
                    check_id: "ailoop.server".to_string(),
                    title: "Server connectivity".to_string(),
                    severity: CheckSeverity::Error,
                    message: format!("Cannot reach server at {}: {}", server_url, e),
                    detail: None,
                    remediation: Some("Check that ailoop serve is running".to_string()),
                },
            }
        })
    }
}
