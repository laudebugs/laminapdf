//! Sending the outbox's reports to the update Worker's `POST /crash`, from the main process
//! only: the webview has no network permission for it. The request carries the report and nothing
//! else: no install ID (the updater's `Lamina-Install-Id` is never set on this client), no cookie,
//! no user agent beyond the client's own.

use std::time::Duration;

use super::queue::{Queue, Shelf};
use super::report::Report;
use crate::network::{NetworkLog, RequestKind, RequestOutcome};

/// Where reports go.
pub(crate) const CRASH_ENDPOINT: &str = "https://updates.laminapdf.com/crash";

/// The largest body the Worker accepts.
pub(crate) const MAX_BODY: usize = 64 * 1024;

/// How long one request may take.
const TIMEOUT: Duration = Duration::from_secs(20);

/// What sending one report came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sent {
    /// The Worker took it (202).
    Taken,
    /// The Worker refused it for good (another 4xx): sending it again would fail again.
    Refused,
    /// Offline, rate-limited or a server error: try again later.
    Later,
}

impl Sent {
    /// What an HTTP `status` from the Worker means.
    pub(crate) const fn of(status: u16) -> Self {
        match status {
            200..300 => Self::Taken,
            429 => Self::Later,
            400..500 => Self::Refused,
            _ => Self::Later,
        }
    }
}

/// Sends every report in `queue`'s outbox to `endpoint`, oldest first, until one must wait.
/// Taken and refused reports leave the outbox. Returns whether it emptied.
pub(crate) async fn send_outbox(queue: &Queue, endpoint: &str, log: &NetworkLog) -> bool {
    let Some(client) = client() else { return false };
    let mut reports = queue.list(Shelf::Outbox);
    reports.reverse();
    for report in reports {
        match send(&client, endpoint, &report, log).await {
            // A report that can't leave the outbox is sent again later, not at once.
            Sent::Taken | Sent::Refused => {
                if queue.take(Shelf::Outbox, &report.report_id).is_none() {
                    return false;
                }
            }
            Sent::Later => return false,
        }
    }
    true
}

async fn send(client: &reqwest::Client, endpoint: &str, report: &Report, log: &NetworkLog) -> Sent {
    let Ok(body) = serde_json::to_vec(report) else {
        return Sent::Refused;
    };
    if body.len() > MAX_BODY {
        return Sent::Refused;
    }
    let bytes = u64::try_from(body.len()).unwrap_or(u64::MAX);
    let request = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body);
    let sent = match request.send().await {
        Ok(response) => Sent::of(response.status().as_u16()),
        Err(error) => {
            tracing::info!("a crash report waits to be sent: {}", error.without_url());
            Sent::Later
        }
    };
    let outcome = match sent {
        Sent::Taken => RequestOutcome::Ok,
        Sent::Refused => RequestOutcome::Refused,
        Sent::Later => RequestOutcome::Failed,
    };
    log.record(RequestKind::CrashReport, endpoint, outcome, bytes);
    sent
}

/// The client, on the updater's TLS: rustls with the ring provider, installed as the updater
/// installs it.
fn client() -> Option<reqwest::Client> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // Fails only when another thread installed one first, which is as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .inspect_err(|error| tracing::warn!("no client for crash reports: {error}"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workers_answers_should_say_whether_to_try_again() {
        let answers = [202, 400, 413, 415, 429, 500, 503].map(Sent::of);
        assert_eq!(
            answers,
            [
                Sent::Taken,
                Sent::Refused,
                Sent::Refused,
                Sent::Refused,
                Sent::Later,
                Sent::Later,
                Sent::Later
            ]
        );
    }
}
