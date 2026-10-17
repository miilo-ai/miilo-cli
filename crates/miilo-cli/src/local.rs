#[tokio::main]
async fn main() -> std::process::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("miilo-local {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    match miilo_local::execute(&std::env::args().skip(1).collect::<Vec<_>>()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("miilo-local: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
