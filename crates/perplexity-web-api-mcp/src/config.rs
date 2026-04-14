use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{
    fs::OpenOptions,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

#[cfg(not(unix))]
use std::fs::OpenOptions;

use crate::auth::AuthTokens;

const CONFIG_DIR_NAME: &str = "perplexity-web-api-mcp";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Clone, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default)]
    auth: Option<StoredAuth>,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredAuth {
    session_token: String,
    csrf_token: String,
    updated_at: String,
}

pub(crate) fn default_config_path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|path| path.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
        .ok_or_else(|| io::Error::other("Unable to determine the user config directory"))
}

pub(crate) fn load_auth_from_path(path: &Path) -> io::Result<Option<AuthTokens>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    if raw.trim().is_empty() {
        return Ok(None);
    }

    let config = match serde_json::from_str::<AppConfig>(&raw) {
        Ok(config) => config,
        Err(err) => {
            tracing::warn!("Ignoring malformed auth config at {}: {}", path.display(), err);
            return Ok(None);
        }
    };

    let Some(auth) = config.auth else {
        return Ok(None);
    };

    match AuthTokens::try_new(auth.session_token, auth.csrf_token) {
        Ok(tokens) => Ok(Some(tokens)),
        Err(err) => {
            tracing::warn!("Ignoring invalid auth config at {}: {}", path.display(), err);
            Ok(None)
        }
    }
}

pub(crate) fn save_auth_to_path(path: &Path, auth: &AuthTokens) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::other(format!(
            "Auth config path {} has no parent directory",
            path.display()
        ))
    })?;

    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    set_permissions(parent, 0o700)?;

    let payload = AppConfig {
        auth: Some(StoredAuth {
            session_token: auth.session_token().to_owned(),
            csrf_token: auth.csrf_token().to_owned(),
            updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        }),
    };
    let json = serde_json::to_string_pretty(&payload)
        .map_err(|err| io::Error::other(format!("Failed to serialize auth config: {err}")))?;

    let temp_path = temp_config_path(path);
    let mut open_options = OpenOptions::new();
    open_options.write(true).create_new(true);
    #[cfg(unix)]
    open_options.mode(0o600);

    let mut file = match open_options.open(&temp_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(io::Error::other(format!("Failed to create temp auth config: {err}")));
        }
    };
    file.write_all(json.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;

    #[cfg(unix)]
    set_permissions(&temp_path, 0o600)?;

    replace_file(&temp_path, path)?;
    #[cfg(unix)]
    set_permissions(path, 0o600)?;

    Ok(())
}

fn temp_config_path(path: &Path) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let filename = format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|name| name.to_str()).unwrap_or("config"),
        unique
    );
    path.with_file_name(filename)
}

fn replace_file(temp_path: &Path, final_path: &Path) -> io::Result<()> {
    match fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        #[cfg(windows)]
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(final_path)?;
            fs::rename(temp_path, final_path)
        }
        Err(err) => {
            let _ = fs::remove_file(temp_path);
            Err(err)
        }
    }
}

#[cfg(unix)]
fn set_permissions(path: &Path, mode: u32) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use crate::auth::AuthTokens;
    use crate::test_utils::TempDir;

    use super::{load_auth_from_path, save_auth_to_path};

    #[test]
    fn saves_and_loads_auth_config() {
        let temp_dir = TempDir::new("config-round-trip");
        let config_path = temp_dir.path().join("config.json");
        let auth = AuthTokens::try_new("session-token".into(), "csrf-token".into()).unwrap();

        save_auth_to_path(&config_path, &auth).unwrap();
        let loaded = load_auth_from_path(&config_path).unwrap().unwrap();

        assert_eq!(loaded, auth);
    }

    #[test]
    fn ignores_malformed_config_files() {
        let temp_dir = TempDir::new("config-malformed");
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{not valid json").unwrap();

        let loaded = load_auth_from_path(&config_path).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn overwrites_existing_auth_config() {
        let temp_dir = TempDir::new("config-overwrite");
        let config_path = temp_dir.path().join("config.json");

        save_auth_to_path(
            &config_path,
            &AuthTokens::try_new("old-session".into(), "old-csrf".into()).unwrap(),
        )
        .unwrap();
        save_auth_to_path(
            &config_path,
            &AuthTokens::try_new("new-session".into(), "new-csrf".into()).unwrap(),
        )
        .unwrap();

        let loaded = load_auth_from_path(&config_path).unwrap().unwrap();
        assert_eq!(loaded.session_token(), "new-session");
        assert_eq!(loaded.csrf_token(), "new-csrf");

        let lingering_temp_files = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect::<Vec<_>>();
        assert!(lingering_temp_files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn saves_auth_config_with_restrictive_permissions() {
        let temp_dir = TempDir::new("config-permissions");
        let config_path = temp_dir.path().join("config.json");
        let auth = AuthTokens::try_new("session-token".into(), "csrf-token".into()).unwrap();

        save_auth_to_path(&config_path, &auth).unwrap();

        let dir_mode = fs::metadata(temp_dir.path()).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(&config_path).unwrap().permissions().mode() & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }
}
