use dialoguer::{Confirm, Password, theme::ColorfulTheme};
use perplexity_web_api::{AuthCookies, Client};
use std::{future::Future, io, path::Path, pin::Pin};

use crate::{auth::AuthTokens, config};

pub(crate) async fn run_first_run_setup(config_path: &Path) -> io::Result<Option<AuthTokens>> {
    tracing::info!(
        "No Perplexity authentication found in the environment or local config cache"
    );
    tracing::info!(
        "Interactive first-run setup can save your session token and CSRF token locally for future runs"
    );

    run_first_run_setup_with(
        config_path,
        || async { run_blocking_prompt(prompt_for_auth).await },
        || async { run_blocking_prompt(prompt_retry_after_validation_failure).await },
        |auth| Box::pin(validate_auth(auth)),
    )
    .await
}

type ValidationFuture<'a> = Pin<Box<dyn Future<Output = io::Result<()>> + 'a>>;

async fn run_first_run_setup_with<Prompt, PromptFuture, RetryPrompt, RetryFuture, Validate>(
    config_path: &Path,
    mut prompt: Prompt,
    mut retry_prompt: RetryPrompt,
    mut validate: Validate,
) -> io::Result<Option<AuthTokens>>
where
    Prompt: FnMut() -> PromptFuture,
    PromptFuture: Future<Output = io::Result<Option<AuthTokens>>>,
    RetryPrompt: FnMut() -> RetryFuture,
    RetryFuture: Future<Output = io::Result<bool>>,
    Validate: for<'a> FnMut(&'a AuthTokens) -> ValidationFuture<'a>,
{
    loop {
        let Some(auth) = prompt().await? else {
            tracing::info!("Interactive first-run setup skipped without saving auth");
            return Ok(None);
        };

        match validate(&auth).await {
            Ok(()) => {
                config::save_auth_to_path(config_path, &auth)?;
                tracing::info!(
                    "Saved validated Perplexity authentication to {}",
                    config_path.display()
                );
                return Ok(Some(auth));
            }
            Err(err) => {
                tracing::warn!(
                    "Authentication validation failed; the provided tokens were not saved"
                );
                tracing::info!("{}", err);
                if !retry_prompt().await? {
                    tracing::info!(
                        "Interactive first-run setup cancelled after validation failure"
                    );
                    return Ok(None);
                }
            }
        }
    }
}

async fn run_blocking_prompt<Prompt, Output>(prompt: Prompt) -> io::Result<Output>
where
    Prompt: FnOnce() -> io::Result<Output> + Send + 'static,
    Output: Send + 'static,
{
    tokio::task::spawn_blocking(prompt)
        .await
        .map_err(|err| io::Error::other(format!("Interactive setup task failed: {err}")))?
}

fn prompt_for_auth() -> io::Result<Option<AuthTokens>> {
    let theme = ColorfulTheme::default();

    let should_configure = prompt_confirm(
        &theme,
        "Set up saved Perplexity authentication now?",
        true,
        "setup confirmation",
    )?;

    if !should_configure {
        return Ok(None);
    }

    let session_token = prompt_password(
        &theme,
        "Perplexity session token (leave blank to skip)",
        "session token",
    )?;

    if session_token.trim().is_empty() {
        return Ok(None);
    }

    let csrf_token =
        prompt_password(&theme, "Perplexity CSRF token (leave blank to skip)", "CSRF token")?;

    if csrf_token.trim().is_empty() {
        return Ok(None);
    }

    AuthTokens::try_new(session_token, csrf_token).map(Some)
}

fn prompt_retry_after_validation_failure() -> io::Result<bool> {
    let theme = ColorfulTheme::default();
    prompt_confirm(
        &theme,
        "Authentication validation failed. Try entering the tokens again?",
        true,
        "retry confirmation",
    )
}

fn prompt_confirm(
    theme: &ColorfulTheme,
    prompt: &str,
    default: bool,
    context: &str,
) -> io::Result<bool> {
    match Confirm::with_theme(theme).with_prompt(prompt).default(default).interact() {
        Ok(value) => Ok(value),
        Err(err) if prompt_cancelled(&err) => Ok(false),
        Err(err) => Err(io::Error::other(format!("Failed to read {context}: {err}"))),
    }
}

fn prompt_password(theme: &ColorfulTheme, prompt: &str, context: &str) -> io::Result<String> {
    match Password::with_theme(theme).with_prompt(prompt).allow_empty_password(true).interact()
    {
        Ok(value) => Ok(value),
        Err(err) if prompt_cancelled(&err) => Ok(String::new()),
        Err(err) => Err(io::Error::other(format!("Failed to read {context}: {err}"))),
    }
}

fn prompt_cancelled(err: &dialoguer::Error) -> bool {
    matches!(
        err,
        dialoguer::Error::IO(io_err)
            if matches!(io_err.kind(), io::ErrorKind::Interrupted | io::ErrorKind::UnexpectedEof)
    )
}

async fn validate_auth(auth: &AuthTokens) -> io::Result<()> {
    Client::builder()
        .cookies(AuthCookies::new(auth.session_token(), auth.csrf_token()))
        .build()
        .await
        .map(|_| ())
        .map_err(|err| {
            io::Error::other(format!(
                "Authentication validation failed: {err}. The provided tokens were not saved."
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, io};

    use super::{prompt_cancelled, run_first_run_setup_with};
    use crate::{auth::AuthTokens, config, test_utils::TempDir};

    #[tokio::test]
    async fn setup_returns_none_when_prompt_is_skipped() {
        let temp_dir = TempDir::new("setup-skip");
        let config_path = temp_dir.path().join("config.json");

        let result = run_first_run_setup_with(
            &config_path,
            || async { Ok(None) },
            || async { Ok(false) },
            |_| Box::pin(async { Ok(()) }),
        )
        .await
        .unwrap();

        assert!(result.is_none());
        assert!(config::load_auth_from_path(&config_path).unwrap().is_none());
    }

    #[tokio::test]
    async fn setup_retries_after_validation_failure() {
        let temp_dir = TempDir::new("setup-retry");
        let config_path = temp_dir.path().join("config.json");
        let prompt_calls = Cell::new(0);
        let validate_calls = Cell::new(0);

        let result = run_first_run_setup_with(
            &config_path,
            || async {
                prompt_calls.set(prompt_calls.get() + 1);
                let auth = if prompt_calls.get() == 1 {
                    AuthTokens::try_new("bad-session".into(), "bad-csrf".into())
                } else {
                    AuthTokens::try_new("good-session".into(), "good-csrf".into())
                }?;
                Ok(Some(auth))
            },
            || async { Ok(true) },
            |auth| {
                validate_calls.set(validate_calls.get() + 1);
                Box::pin(async move {
                    if auth.session_token() == "bad-session" {
                        Err(io::Error::other("validation failed"))
                    } else {
                        Ok(())
                    }
                })
            },
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(prompt_calls.get(), 2);
        assert_eq!(validate_calls.get(), 2);
        assert_eq!(result.session_token(), "good-session");
        assert_eq!(result.csrf_token(), "good-csrf");

        let saved = config::load_auth_from_path(&config_path).unwrap().unwrap();
        assert_eq!(saved, result);
    }

    #[tokio::test]
    async fn setup_returns_none_when_retry_is_declined() {
        let temp_dir = TempDir::new("setup-decline");
        let config_path = temp_dir.path().join("config.json");

        let result = run_first_run_setup_with(
            &config_path,
            || async {
                Ok(Some(AuthTokens::try_new("bad-session".into(), "bad-csrf".into())?))
            },
            || async { Ok(false) },
            |_| Box::pin(async { Err(io::Error::other("validation failed")) }),
        )
        .await
        .unwrap();

        assert!(result.is_none());
        assert!(config::load_auth_from_path(&config_path).unwrap().is_none());
    }

    #[test]
    fn interrupted_prompt_is_treated_as_cancellation() {
        let error =
            dialoguer::Error::IO(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        assert!(prompt_cancelled(&error));
    }
}
