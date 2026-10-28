//! Linking a Discord account by QR code, or by a token the person pastes,
//! beside the chat backend, so a build without Matrix can link before
//! anything else runs. The login goes into the OS credential store; see
//! `mincord_discord::remote_auth`.

use mincord_discord::{
    api::{Api, Refused},
    remote_auth::{self, Step, Stopped},
    token,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::runtime::{BackgroundRuntime, BackgroundTask};

/// How linking goes, for [`crate::application::Application::linking`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The code to show, as the address it carries.
    Code(String),
    /// Someone scanned it, by their username, and confirms on their phone.
    Scanned(String),
    /// Discord wants a captcha solved, on the page at this address.
    Captcha(String),
    /// The account is linked and its login saved.
    Linked,
    /// The code expired before anyone confirmed it.
    Expired,
    Failed(String),
}

/// Starts linking an account on `runtime`: by QR code, or with `token`
/// when the person pasted one. Its progress arrives on the receiver until it
/// ends, and dropping the task stops it.
pub fn start(
    runtime: &BackgroundRuntime,
    token: Option<String>,
) -> (BackgroundTask<()>, UnboundedReceiver<Progress>) {
    let (sender, progress) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    let task = match token {
        Some(token) => runtime.spawn(with_token(token, report)),
        None => runtime.spawn(run(report)),
    };
    (task, progress)
}

/// Links the account a pasted token belongs to, once Discord accepts it.
async fn with_token(token: String, progress: impl Fn(Progress)) {
    let token = match token::clean(&token) {
        Ok(token) => token,
        Err(error) => return progress(Progress::Failed(error.to_string())),
    };
    let checked = async {
        let api = Api::new(token.clone())?;
        api.learn_build_number().await;
        api.me().await?;
        token::store(&token)
    };
    progress(match checked.await {
        Ok(()) => Progress::Linked,
        Err(error) => match error.downcast_ref::<Refused>() {
            Some(Refused(status)) if status.as_u16() == 401 => {
                Progress::Failed("Discord didn't accept that token.".into())
            }
            _ => {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "could not link Discord by token"
                );
                Progress::Failed(format!("Mincord couldn't link Discord: {error}"))
            }
        },
    });
}

/// Links an account, telling `progress` how it goes until it ends.
async fn run(progress: impl Fn(Progress)) {
    let linked = remote_auth::link(|step| {
        progress(match step {
            Step::Code(address) => Progress::Code(address),
            Step::Scanned { name, .. } => Progress::Scanned(name),
            Step::Captcha(page) => Progress::Captcha(page),
        })
    })
    .await;
    progress(match linked.and_then(|login| token::store(&login)) {
        Ok(()) => Progress::Linked,
        Err(error) => stopped(error),
    });
}

/// What to show when linking by code ended in `error`.
fn stopped(error: anyhow::Error) -> Progress {
    match error.downcast_ref::<Stopped>() {
        Some(Stopped::Expired) => return Progress::Expired,
        Some(stopped) => return Progress::Failed(stopped.to_string()),
        None => {}
    }
    // Discord forgets a confirmed sign-in's ticket soon after the phone
    // approves it, and answers a late trade with 404. Solving a captcha can
    // take longer than it waits, and the only way on from a lost ticket is a
    // new code, so this shows one rather than stopping at an error.
    if matches!(error.downcast_ref::<Refused>(), Some(Refused(status)) if status.as_u16() == 404) {
        tracing::warn!("Discord forgot the sign-in's ticket; showing a new code");
        return Progress::Expired;
    }
    tracing::warn!(error = format!("{error:#}"), "could not link Discord");
    Progress::Failed(format!("Mincord couldn't link Discord: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lost_ticket_asks_for_a_new_code_rather_than_stopping() {
        let gone = anyhow::Error::new(Refused(
            mincord_discord::api::StatusCode::NOT_FOUND,
        ))
        .context("Discord forgot the sign-in before Mincord could finish it.");
        assert_eq!(stopped(gone), Progress::Expired);
        assert_eq!(stopped(Stopped::Expired.into()), Progress::Expired);
        assert_eq!(
            stopped(Stopped::Cancelled.into()),
            Progress::Failed(Stopped::Cancelled.to_string())
        );
        // Anything else is still worth showing as it is.
        let refused = anyhow::Error::new(Refused(
            mincord_discord::api::StatusCode::INTERNAL_SERVER_ERROR,
        ));
        assert!(matches!(stopped(refused), Progress::Failed(_)));
    }
}
