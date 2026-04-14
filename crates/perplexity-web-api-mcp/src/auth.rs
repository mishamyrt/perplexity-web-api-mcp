use perplexity_web_api::{AuthCookies, REDACTED_SECRET};
use std::{
    env,
    env::VarError,
    fmt,
    future::Future,
    io,
    path::{Path, PathBuf},
};

use crate::{config, setup, tty};

pub(crate) const SESSION_TOKEN_ENV: &str = "PERPLEXITY_SESSION_TOKEN";
pub(crate) const CSRF_TOKEN_ENV: &str = "PERPLEXITY_CSRF_TOKEN";

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AuthTokens {
    session_token: String,
    csrf_token: String,
}

impl fmt::Debug for AuthTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthTokens")
            .field("session_token", &REDACTED_SECRET)
            .field("csrf_token", &REDACTED_SECRET)
            .finish()
    }
}

impl AuthTokens {
    pub(crate) fn try_new(session_token: String, csrf_token: String) -> io::Result<Self> {
        let session_token = session_token.trim().to_owned();
        let csrf_token = csrf_token.trim().to_owned();

        if session_token.is_empty() || csrf_token.is_empty() {
            return Err(io::Error::other(
                "Saved Perplexity authentication must include both a non-empty session token and CSRF token",
            ));
        }

        Ok(Self { session_token, csrf_token })
    }

    pub(crate) fn session_token(&self) -> &str {
        &self.session_token
    }

    pub(crate) fn csrf_token(&self) -> &str {
        &self.csrf_token
    }

    pub(crate) fn into_cookies(self) -> AuthCookies {
        AuthCookies::new(self.session_token, self.csrf_token)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthSource {
    Environment,
    CachedConfig,
    InteractiveSetup,
    Tokenless,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedAuth {
    pub(crate) cookies: Option<AuthCookies>,
    pub(crate) source: AuthSource,
}

impl ResolvedAuth {
    fn authenticated(tokens: AuthTokens, source: AuthSource) -> Self {
        Self { cookies: Some(tokens.into_cookies()), source }
    }

    fn tokenless() -> Self {
        Self { cookies: None, source: AuthSource::Tokenless }
    }

    pub(crate) fn tokenless_mode(&self) -> bool {
        self.cookies.is_none()
    }
}

pub(crate) async fn resolve_auth() -> io::Result<ResolvedAuth> {
    let config_path = match config::default_config_path() {
        Ok(path) => Some(path),
        Err(err) => {
            tracing::warn!("Unable to resolve the local auth config path: {}", err);
            None
        }
    };

    resolve_auth_with(
        |name| env::var(name),
        config_path.as_deref(),
        tty::is_interactive(),
        |path| async move { setup::run_first_run_setup(&path).await },
    )
    .await
}

pub(crate) async fn resolve_auth_with<GetEnv, Setup, SetupFuture>(
    get_env: GetEnv,
    config_path: Option<&Path>,
    interactive: bool,
    setup_runner: Setup,
) -> io::Result<ResolvedAuth>
where
    GetEnv: for<'a> Fn(&'a str) -> Result<String, VarError>,
    Setup: FnOnce(PathBuf) -> SetupFuture,
    SetupFuture: Future<Output = io::Result<Option<AuthTokens>>>,
{
    if let Some(tokens) = load_auth_from_env_with(&get_env)? {
        return Ok(ResolvedAuth::authenticated(tokens, AuthSource::Environment));
    }

    if let Some(path) = config_path
        && let Some(tokens) = config::load_auth_from_path(path)?
    {
        return Ok(ResolvedAuth::authenticated(tokens, AuthSource::CachedConfig));
    }

    if interactive {
        if let Some(path) = config_path {
            if let Some(tokens) = setup_runner(path.to_path_buf()).await? {
                return Ok(ResolvedAuth::authenticated(tokens, AuthSource::InteractiveSetup));
            }
        } else {
            tracing::warn!(
                "Interactive first-run setup is unavailable because the local config path could not be determined"
            );
        }
    } else {
        tracing::warn!(
            "No Perplexity authentication found and the terminal is non-interactive; starting in tokenless mode"
        );
    }

    Ok(ResolvedAuth::tokenless())
}

pub(crate) fn load_auth_from_env_with<GetEnv>(
    get_env: GetEnv,
) -> io::Result<Option<AuthTokens>>
where
    GetEnv: for<'a> Fn(&'a str) -> Result<String, VarError>,
{
    let session_token = read_optional_env(&get_env, SESSION_TOKEN_ENV)?;
    let csrf_token = read_optional_env(&get_env, CSRF_TOKEN_ENV)?;

    match (session_token, csrf_token) {
        (Some(session_token), Some(csrf_token)) => {
            Ok(Some(AuthTokens { session_token, csrf_token }))
        }
        (None, None) => Ok(None),
        _ => Err(io::Error::other(format!(
            "Perplexity authentication environment is partially configured. \
             Set both {SESSION_TOKEN_ENV} and {CSRF_TOKEN_ENV}, or unset both to use saved local auth or tokenless mode."
        ))),
    }
}

fn read_optional_env<GetEnv>(get_env: &GetEnv, name: &str) -> io::Result<Option<String>>
where
    GetEnv: Fn(&str) -> Result<String, VarError>,
{
    match get_env(name) {
        Ok(value) => {
            let trimmed = value.trim().to_owned();
            if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => {
            Err(io::Error::other(format!("Environment variable {name} must be valid UTF-8")))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        env::VarError,
        fs, future,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use super::{
        AuthSource, AuthTokens, CSRF_TOKEN_ENV, SESSION_TOKEN_ENV, load_auth_from_env_with,
        resolve_auth_with,
    };
    use crate::{config, test_utils::TempDir};

    #[tokio::test]
    async fn env_takes_precedence_over_cached_config() {
        let temp_dir = TempDir::new("env-precedence");
        let config_path = temp_dir.path().join("config.json");
        config::save_auth_to_path(
            &config_path,
            &AuthTokens::try_new("cached-session".into(), "cached-csrf".into()).unwrap(),
        )
        .unwrap();

        let env = HashMap::from([
            (SESSION_TOKEN_ENV, "env-session".to_owned()),
            (CSRF_TOKEN_ENV, "env-csrf".to_owned()),
        ]);

        let resolved = resolve_auth_with(env_lookup(&env), Some(&config_path), false, |_| {
            future::ready(Ok(None))
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::Environment);
        let cookies = resolved.cookies.unwrap();
        assert_eq!(cookies.session_token(), "env-session");
        assert_eq!(cookies.csrf_token(), "env-csrf");
    }

    #[test]
    fn partial_env_configuration_is_rejected() {
        let env = HashMap::from([(SESSION_TOKEN_ENV, "session-only".to_owned())]);

        let error = load_auth_from_env_with(env_lookup(&env)).unwrap_err();

        assert!(error.to_string().contains(SESSION_TOKEN_ENV));
        assert!(error.to_string().contains(CSRF_TOKEN_ENV));
    }

    #[tokio::test]
    async fn non_interactive_mode_skips_setup_and_falls_back_to_tokenless() {
        let temp_dir = TempDir::new("non-interactive");
        let config_path = temp_dir.path().join("config.json");
        let prompt_called = Arc::new(AtomicBool::new(false));
        let prompt_called_for_closure = Arc::clone(&prompt_called);

        let resolved = resolve_auth_with(empty_env, Some(&config_path), false, move |_| {
            prompt_called_for_closure.store(true, Ordering::SeqCst);
            future::ready(AuthTokens::try_new("session".into(), "csrf".into()).map(Some))
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::Tokenless);
        assert!(resolved.cookies.is_none());
        assert!(!prompt_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cached_config_is_used_when_env_is_missing() {
        let temp_dir = TempDir::new("cached-config");
        let config_path = temp_dir.path().join("config.json");
        config::save_auth_to_path(
            &config_path,
            &AuthTokens::try_new("cached-session".into(), "cached-csrf".into()).unwrap(),
        )
        .unwrap();

        let resolved = resolve_auth_with(empty_env, Some(&config_path), false, |_| {
            future::ready(Ok(None))
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::CachedConfig);
        let cookies = resolved.cookies.unwrap();
        assert_eq!(cookies.session_token(), "cached-session");
        assert_eq!(cookies.csrf_token(), "cached-csrf");
    }

    #[tokio::test]
    async fn interactive_setup_is_used_when_available() {
        let temp_dir = TempDir::new("interactive-setup");
        let config_path = temp_dir.path().join("config.json");

        let resolved = resolve_auth_with(empty_env, Some(&config_path), true, |_| {
            future::ready(
                AuthTokens::try_new("prompt-session".into(), "prompt-csrf".into()).map(Some),
            )
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::InteractiveSetup);
        let cookies = resolved.cookies.unwrap();
        assert_eq!(cookies.session_token(), "prompt-session");
        assert_eq!(cookies.csrf_token(), "prompt-csrf");
    }

    #[tokio::test]
    async fn interactive_mode_without_config_path_skips_setup_and_falls_back_to_tokenless() {
        let prompt_called = Arc::new(AtomicBool::new(false));
        let prompt_called_for_closure = Arc::clone(&prompt_called);

        let resolved = resolve_auth_with(empty_env, None, true, move |_| {
            prompt_called_for_closure.store(true, Ordering::SeqCst);
            future::ready(AuthTokens::try_new("session".into(), "csrf".into()).map(Some))
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::Tokenless);
        assert!(resolved.cookies.is_none());
        assert!(!prompt_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn malformed_cached_config_falls_back_to_interactive_setup() {
        let temp_dir = TempDir::new("malformed-config");
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{not json").unwrap();

        let resolved = resolve_auth_with(empty_env, Some(&config_path), true, |_| {
            future::ready(
                AuthTokens::try_new("prompt-session".into(), "prompt-csrf".into()).map(Some),
            )
        })
        .await
        .unwrap();

        assert_eq!(resolved.source, AuthSource::InteractiveSetup);
        let cookies = resolved.cookies.unwrap();
        assert_eq!(cookies.session_token(), "prompt-session");
        assert_eq!(cookies.csrf_token(), "prompt-csrf");
    }

    fn env_lookup<'a>(
        values: &'a HashMap<&'static str, String>,
    ) -> impl Fn(&str) -> Result<String, VarError> + 'a {
        move |name| values.get(name).cloned().ok_or(VarError::NotPresent)
    }

    fn empty_env(_: &str) -> Result<String, VarError> {
        Err(VarError::NotPresent)
    }
}
