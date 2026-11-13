#[path = "../tests/telemetry_fixture/mod.rs"]
mod fixture;
#[path = "../../miilo-telemetry/tests/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let binary = args.next().ok_or("supply the miilo binary path")?;
    let collector = args
        .next()
        .ok_or("supply a local collector's full traces endpoint")?;
    let api = support::Server::respond(std::time::Duration::ZERO, |_, request| {
        let body = if request.headers.starts_with("GET ") {
            fixture::snapshot()
        } else {
            fixture::explanation()
        };
        (200, body.to_string().into_bytes())
    });
    let result = std::process::Command::new(binary)
        .args([
            "--endpoint",
            api.url.trim_end_matches("v1/traces"),
            "--json",
            "work",
            "explain",
            "private-provider/private-work",
            "--question",
            "PRIVATE QUESTION",
            "--telemetry-endpoint",
            &collector,
        ])
        .env_remove("MIILO_TOKEN")
        .env_remove("MIILO_TOKEN_FILE")
        .output()?;
    if !result.status.success() || !result.stderr.is_empty() {
        return Err("CLI collector rehearsal failed".into());
    }
    let output: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    if output != fixture::explanation() {
        return Err("explanation output changed".into());
    }
    println!("CLI explanation completed and collector accepted its trace");
    Ok(())
}
