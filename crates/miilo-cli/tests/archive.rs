use std::{fs, process::Command};
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_miilo"))
        .args(args)
        .env("MIILO_URL", "not-a-url")
        .env("MIILO_TOKEN", "fixture-conflicting-token")
        .env("MIILO_TOKEN_FILE", "/missing-credential")
        .output()
        .unwrap()
}
#[test]
fn all_codecs_restore_exactly_without_account_and_never_clobber() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input");
    let data = "Synthetic café 🦉 evidence\n".repeat(1000).into_bytes();
    fs::write(&input, &data).unwrap();
    for codec in ["gzip", "brotli", "zstd"] {
        let archive = dir.path().join(format!("{codec}.miilo"));
        let output = dir.path().join(format!("{codec}.restored"));
        assert!(
            run(&[
                "archive",
                "export",
                input.to_str().unwrap(),
                archive.to_str().unwrap(),
                "--codec",
                codec
            ])
            .status
            .success()
        );
        assert!(
            run(&["archive", "inspect", archive.to_str().unwrap()])
                .status
                .success()
        );
        assert!(
            run(&[
                "archive",
                "restore",
                archive.to_str().unwrap(),
                output.to_str().unwrap()
            ])
            .status
            .success()
        );
        assert_eq!(fs::read(&output).unwrap(), data);
        assert!(
            !run(&[
                "archive",
                "restore",
                archive.to_str().unwrap(),
                output.to_str().unwrap()
            ])
            .status
            .success()
        );
        assert_eq!(fs::read(&output).unwrap(), data);
        let mut bytes = fs::read(&archive).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&archive, bytes).unwrap();
        let failed = dir.path().join("failed");
        assert!(
            !run(&[
                "archive",
                "restore",
                archive.to_str().unwrap(),
                failed.to_str().unwrap()
            ])
            .status
            .success()
        );
        assert!(!failed.exists());
    }
}
#[test]
fn raw_streams_and_limits_publish_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input");
    fs::write(&input, vec![42; 100000]).unwrap();
    for codec in ["gzip", "brotli", "zstd"] {
        let compressed = dir.path().join(codec);
        let restored = dir.path().join(format!("{codec}.decoded"));
        assert!(
            run(&[
                "archive",
                "compress",
                codec,
                input.to_str().unwrap(),
                compressed.to_str().unwrap()
            ])
            .status
            .success()
        );
        assert!(
            run(&[
                "archive",
                "decompress",
                codec,
                compressed.to_str().unwrap(),
                restored.to_str().unwrap()
            ])
            .status
            .success()
        );
        assert_eq!(fs::read(&restored).unwrap(), fs::read(&input).unwrap());
        let fail = dir.path().join("too-large");
        assert!(
            !run(&[
                "archive",
                "decompress",
                codec,
                compressed.to_str().unwrap(),
                fail.to_str().unwrap(),
                "--max-bytes",
                "1024"
            ])
            .status
            .success()
        );
        assert!(!fail.exists());
    }
}
