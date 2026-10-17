//! Explicit local artifact I/O. No account, provider enrollment or network access.
mod files;
use miilo_adapter_sdk::compression::{Codec, Limits, bundle, codecs};
use std::path::Path;
const USAGE: &str = "miilo archive <export INPUT OUTPUT [--codec gzip|brotli|zstd]|restore INPUT OUTPUT|inspect INPUT|compress CODEC INPUT OUTPUT|decompress CODEC INPUT OUTPUT> [--max-bytes N]";
pub fn execute(values: &[String]) -> Result<(), String> {
    let mut args = Vec::new();
    let mut limit = 64 * 1024 * 1024u64;
    let mut codec = Codec::Zstd;
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--max-bytes" => {
                i += 1;
                limit = values.get(i).ok_or(USAGE)?.parse().map_err(|_| USAGE)?;
            }
            "--codec" => {
                i += 1;
                codec = parse_codec(values.get(i).ok_or(USAGE)?)?;
            }
            _ => args.push(values[i].as_str()),
        }
        i += 1;
    }
    if limit == 0 || limit > 256 * 1024 * 1024 {
        return Err("Byte limit must be between 1 and 268435456".into());
    }
    let limits = Limits {
        input_bytes: limit,
        output_bytes: limit,
    };
    match args.as_slice() {
        ["export", input, output] => {
            let data = files::read(Path::new(input), limit)?;
            let encoded = bundle::encode(codec, &data, limits).map_err(|e| e.to_string())?;
            // Verify before publishing, including codec finalization and both hashes.
            let (manifest, restored) =
                bundle::decode(&encoded, limits).map_err(|e| e.to_string())?;
            if restored != data {
                return Err("Export verification failed".into());
            }
            files::publish(Path::new(output), |writer| {
                std::io::Write::write_all(writer, &encoded)
            })?;
            println!(
                "{}",
                serde_json::json!({"verified":true,"manifest":manifest})
            );
        }
        ["restore", input, output] => {
            let encoded = files::read(Path::new(input), limit + 4108)?;
            let (manifest, data) = bundle::decode(&encoded, limits).map_err(|e| e.to_string())?;
            files::publish(Path::new(output), |writer| {
                std::io::Write::write_all(writer, &data)
            })?;
            println!(
                "{}",
                serde_json::json!({"verified":true,"manifest":manifest})
            );
        }
        ["inspect", input] => {
            let encoded = files::read(Path::new(input), limit + 4108)?;
            let (manifest, _) = bundle::decode(&encoded, limits).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::json!({"verified":true,"manifest":manifest})
            );
        }
        [mode @ ("compress" | "decompress"), name, input, output] => {
            let codec = parse_codec(name)?;
            let file = files::open(Path::new(input))?;
            files::publish(Path::new(output), |writer| {
                if *mode == "compress" {
                    codecs::compress_to(codec, file, writer, limits)
                } else {
                    codecs::decompress_to(codec, file, writer, limits)
                }
            })?;
        }
        ["--help"] | ["-h"] => println!("{USAGE}"),
        _ => return Err(USAGE.into()),
    }
    Ok(())
}
fn parse_codec(value: &str) -> Result<Codec, String> {
    match value {
        "gzip" => Ok(Codec::Gzip),
        "brotli" | "br" => Ok(Codec::Brotli),
        "zstd" => Ok(Codec::Zstd),
        _ => Err("Unsupported codec: choose gzip, brotli or zstd".into()),
    }
}
