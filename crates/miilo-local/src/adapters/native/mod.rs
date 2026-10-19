use crate::codex::discovery;
use miilo_adapter_sdk::runtime::observation::Observation;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{self, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;
mod configuration;
mod configuration_migration;
mod files;
mod groups;
mod import_flow;
mod inventory;
mod local_run;
mod observation_ports;
mod pairing;
mod remote_run;
mod selection;
mod service;
mod session_databases;
mod sessions;
mod wakeups;
use files::{cache_path, config_path, home, private_read, save_config};
use inventory::inventory;
use sessions::sessions;
type Result<T> = std::result::Result<T, String>;

pub async fn execute(args: &[String]) -> Result<()> {
    let directory = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| home()));
    match args.first().map(String::as_str) {
        Some("share") => pairing::share(&args[1..]).await,
        Some("pair") => pairing::pair(&args[1..]).await,
        Some("--version") => {
            println!("miilo-local {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("config") => configuration::execute(&args[1..]),
        Some("discover") => {
            let mut found = inventory(&directory).await;
            if let Ok(text) = private_read(&config_path()?) {
                if let Ok(saved) = serde_json::from_str::<Value>(&text) {
                    found["savedGroups"] = saved["groups"].clone();
                }
            }
            println!("{found}");
            Ok(())
        }
        Some("groups") => groups::execute().await,
        Some("import") => import_flow::execute().await,
        Some("setup") => import_flow::setup(&directory).await,
        Some("run") => {
            let config: Value = serde_json::from_str(&private_read(&config_path()?)?)
                .map_err(|_| "Invalid local configuration")?;
            let once = args.get(1).is_some_and(|a| a == "--once");
            if config["version"] == 1 {
                remote_run::run(once).await
            } else {
                local_run::run(config, once).await
            }
        }
        Some("doctor") => {
            let config: Value = serde_json::from_str(&private_read(&config_path()?)?)
                .map_err(|_| "Invalid local configuration")?;
            if config["version"] == 1 {
                println!(
                    "{}",
                    json!({"mode":"remote","configurationVersion":1,"selectedSessions":config["sessions"].as_array().map_or(0,Vec::len),"message":"Legacy CE publication configuration; use explicit remote doctor for API health"})
                );
                return Ok(());
            }
            match local_run::read_snapshot() {
                Ok(snapshot) => {
                    println!(
                        "{}",
                        json!({"mode":"local","configurationVersion":2,"readOnly":true,"status":if snapshot.providers.iter().all(|p|p.health==miilo_adapter_sdk::Health::Healthy){"current"}else{"degraded"},"selectedSessions":config["sessions"].as_array().map_or(0,Vec::len),"providers":snapshot.providers.len(),"workItems":snapshot.work_items.len(),"observedAt":snapshot.observed_at})
                    );
                    Ok(())
                }
                Err(error) => {
                    println!(
                        "{}",
                        json!({"mode":"local","status":"unavailable","message":error})
                    );
                    Err("Standalone observation is unavailable".into())
                }
            }
        }
        Some("snapshot") => {
            let snapshot = local_run::read_snapshot()?;
            println!(
                "{}",
                serde_json::to_string(&snapshot).map_err(|_| "Cannot encode snapshot")?
            );
            Ok(())
        }
        Some("explain") => {
            let target = args.get(1).ok_or("Use explain <provider>/<work>")?;
            let (p, w) = target
                .split_once('/')
                .ok_or("Use explain <provider>/<work>")?;
            let snapshot = local_run::read_snapshot()?;
            let result =
                miilo_adapter_sdk::runtime::explain(&snapshot, p, w).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&result).map_err(|_| "Cannot encode explanation")?
            );
            Ok(())
        }
        _ => {
            println!(
                "miilo local discover [PROJECT] | setup [PROJECT] | import | groups | config paths|init|migrate|check | run [--once] | doctor | snapshot | explain <provider>/<work> | share --bind ADDRESS:PORT --url HTTPS_ORIGIN --name NAME | pair invite|requests|approve|deny|devices|revoke|connect|list|snapshot|disconnect"
            );
            Ok(())
        }
    }
}
