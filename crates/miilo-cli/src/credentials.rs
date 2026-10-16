use std::{fs::File, io::Read, path::Path};

pub fn resolve(token: Option<String>, path: Option<&Path>) -> Result<Option<String>, &'static str> {
    if token.is_some() && path.is_some() {
        return Err("choose either a token or a token file");
    }
    let value = if let Some(path) = path {
        let file = File::open(path).map_err(|_| "credential file cannot be opened")?;
        let metadata = file
            .metadata()
            .map_err(|_| "credential file cannot be inspected")?;
        if !metadata.is_file() || metadata.len() > 4096 {
            return Err("credential file must be regular and at most 4096 bytes");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("credential file must not be accessible to group or other users");
            }
        }
        let mut value = String::new();
        file.take(4097)
            .read_to_string(&mut value)
            .map_err(|_| "credential file cannot be read")?;
        Some(value.trim_end_matches(['\r', '\n']).to_owned())
    } else {
        token
    };
    if value.as_ref().is_some_and(|value| {
        value.is_empty() || value.len() > 4096 || value.chars().any(char::is_whitespace)
    }) {
        return Err("credential must contain one nonempty token");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::resolve;
    #[test]
    fn conflicts_and_invalid_tokens_do_not_echo_credentials() {
        assert!(
            resolve(Some("private value".into()), None)
                .unwrap_err()
                .contains("nonempty")
        );
        assert!(resolve(Some(String::new()), None).is_err());
        assert!(resolve(Some("secret".into()), Some(std::path::Path::new("unused"))).is_err());
        assert_eq!(resolve(None, None).unwrap(), None);
    }
}
