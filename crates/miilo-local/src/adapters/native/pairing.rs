use super::{Result, local_run};
use miilo_pairing::{
    adapters::{client, control, files, server, system::SystemClock},
    application::Clock,
    domain::PairedHost,
};
use serde_json::{Value, json};
use std::{
    env,
    io::{self, BufRead, Read},
    path::PathBuf,
    sync::Arc,
};

fn directory() -> Result<PathBuf> {
    let path = if let Some(path) = env::var_os("MIILO_PAIRING_DIR") {
        PathBuf::from(path)
    } else {
        super::files::home().join(".local/state/miilo/pairing")
    };
    files::prepare(&path)?;
    Ok(path)
}
struct SelectedSnapshot;
impl miilo_pairing::application::Snapshot for SelectedSnapshot {
    fn read(&self) -> Result<Value> {
        let mut snapshot = local_run::read_snapshot()?;
        for p in &mut snapshot.providers {
            p.capabilities.control.clear();
            p.capabilities.speech = None;
            p.capabilities.model_routes.clear();
            p.capabilities.features.clear();
        }
        for w in &mut snapshot.work_items {
            w.actions.clear();
        }
        for a in &mut snapshot.attention {
            a.actions.clear();
        }
        for actor in &mut snapshot.actors {
            actor.capabilities.clear();
        }
        serde_json::to_value(snapshot).map_err(|_| "Cannot encode selected snapshot".into())
    }
}
pub(super) async fn share(args: &[String]) -> Result<()> {
    let value = |key: &str| {
        args.windows(2)
            .find(|p| p[0] == key)
            .map(|p| p[1].clone())
            .ok_or_else(|| format!("Missing {key}"))
    };
    if args.len() != 6
        || args
            .chunks(2)
            .any(|p| !matches!(p[0].as_str(), "--bind" | "--url" | "--name"))
    {
        return Err(
            "Usage: miilo local share --bind ADDRESS:PORT --url https://HOST:PORT --name NAME"
                .into(),
        );
    }
    // Validate a current, explicitly configured local selection before opening any listener.
    local_run::read_snapshot()?;
    server::run(
        server::Options {
            bind: value("--bind")?
                .parse()
                .map_err(|_| "Invalid bind address")?,
            advertised: value("--url")?,
            name: value("--name")?,
            directory: directory()?,
        },
        Arc::new(SelectedSnapshot),
    )
    .await
}
pub(super) async fn pair(args: &[String]) -> Result<()> {
    let directory = directory()?;
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    let command = match words.as_slice() {
        ["invite"] | ["requests"] | ["devices"] => Some(json!({"command":words[0]})),
        ["approve", id, code] => Some(json!({"command":"approve","id":id,"code":code})),
        ["deny", id] | ["revoke", id] => Some(json!({"command":words[0],"id":id})),
        _ => None,
    };
    if let Some(command) = command {
        println!("{}", control::request(&directory, command).await?);
        return Ok(());
    }
    let args = args.to_vec();
    tokio::task::spawn_blocking(move || client_command(&directory, &args))
        .await
        .map_err(|_| "Pairing client failed")?
}
fn client_command(directory: &std::path::Path, args: &[String]) -> Result<()> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["connect", name] => {
            eprintln!("Paste the private pairing invitation (it expires after five minutes):");
            let mut invitation = String::new(); io::stdin().lock().take(16_385).read_line(&mut invitation).map_err(|_| "Cannot read invitation")?;
            let i = client::decode(invitation.trim(), SystemClock.now())?;
            if directory.join(format!("peer-{}.json", i.host_id)).exists() {
                return Err("Computer is already saved; disconnect it explicitly before pairing again".into());
            }
            eprintln!("Connecting to {}. Only approve if both devices show the same code.", i.name);
            let peer = client::connect(i, name, |code| { eprintln!("Comparison code: {code}. Approve this request on the host."); Ok(()) })?;
            let file = directory.join(format!("peer-{}.json", peer.host_id));
            if file.exists() { return Err("Computer is already saved; disconnect it explicitly before pairing again".into()); }
            files::write(&file, &peer)?;
            println!("{}", json!({"status":"paired","hostId":peer.host_id,"name":peer.name,"readOnly":true}));
            Ok(())
        }
        ["list"] => {
            let mut peers = vec![];
            for entry in std::fs::read_dir(directory).map_err(|_| "Cannot list paired computers")?.take(128) {
                let entry = entry.map_err(|_| "Cannot list paired computers")?;
                if entry.file_name().to_string_lossy().starts_with("peer-") {
                    let p: PairedHost = files::read(&entry.path())?;
                    peers.push(json!({"hostId":p.host_id,"name":p.name,"url":p.url,"expiresAt":p.expires_at,"readOnly":true}));
                }
            }
            println!("{}", json!(peers)); Ok(())
        }
        ["snapshot", id] => {
            miilo_pairing::domain::validate_id(id)?;
            let p: PairedHost = files::read(&directory.join(format!("peer-{id}.json")))?;
            println!("{}", client::snapshot(&p)?); Ok(())
        }
        ["disconnect", id] => {
            miilo_pairing::domain::validate_id(id)?;
            std::fs::remove_file(directory.join(format!("peer-{id}.json"))).map_err(|_| "Cannot forget computer")?;
            println!("{}", json!({"status":"forgotten","hostRevocation":"Revoke this device on the host to invalidate its credential"})); Ok(())
        }
        _ => Err("Usage: miilo local pair invite|requests|approve ID CODE|deny ID|devices|revoke ID|connect NAME|list|snapshot HOST_ID|disconnect HOST_ID".into()),
    }
}
