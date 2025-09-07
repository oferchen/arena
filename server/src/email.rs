//! Email utilities and rate limiting.
//!
//! A background task periodically purges expired entries from the rate-limit
//! map. The task runs on the Tokio runtime and its [`JoinHandle`] is stored so
//! it can be aborted during shutdown if necessary.

use lettre::address::AddressError;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;

// -- Configuration ---------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartTls {
    Auto,
    Always,
    Never,
}

impl Default for StartTls {
    fn default() -> Self {
        StartTls::Auto
    }
}

impl std::str::FromStr for StartTls {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(StartTls::Auto),
            "always" => Ok(StartTls::Always),
            "never" => Ok(StartTls::Never),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from: String,
    pub starttls: StartTls,
    pub smtps: bool,
    pub timeout: u64,
    pub user: Option<String>,
    pub pass: Option<String>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 25,
            from: "arena@localhost".into(),
            starttls: StartTls::Auto,
            smtps: false,
            timeout: 10000,
            user: None,
            pass: None,
        }
    }
}

// -- Rate limiting --------------------------------------------------------

static RATE_LIMITS: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CLEANUP: Lazy<JoinHandle<()>> = Lazy::new(|| {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(CLEANUP_INTERVAL).await;
            let now = Instant::now();
            let mut map = match RATE_LIMITS.lock() {
                Ok(m) => m,
                Err(poison) => poison.into_inner(),
            };
            map.retain(|_, &mut instant| now.duration_since(instant) < RATE_LIMIT);
        }
    })
});
const RATE_LIMIT: Duration = Duration::from_secs(60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// Access the cleanup task's [`JoinHandle`].
///
/// The task is started on first use and can be aborted during shutdown
/// if necessary.
pub fn cleanup_handle() -> &'static JoinHandle<()> {
    Lazy::force(&CLEANUP)
}

// retry behaviour
const MAX_RETRIES: u32 = 5;
#[cfg(test)]
const RETRY_BASE: Duration = Duration::from_millis(1);
#[cfg(not(test))]
const RETRY_BASE: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub enum EmailError {
    RateLimited,
    Smtp(String),
    Address(AddressError),
    Build(lettre::error::Error),
    LockPoisoned,
}

pub struct EmailService {
    from: String,
    sender: UnboundedSender<Message>,
}

impl EmailService {
    pub fn new(config: SmtpConfig) -> Result<Self, EmailError> {
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port)
            .timeout(Some(Duration::from_millis(config.timeout)));

        let tls_params = TlsParameters::builder(config.host.clone())
            .build()
            .map_err(|e| {
                log::error!("failed to build TLS parameters: {e:?}");
                EmailError::Smtp(e.to_string())
            })?;

        builder = if config.smtps {
            builder.tls(Tls::Wrapper(tls_params))
        } else {
            match config.starttls {
                StartTls::Always => builder.tls(Tls::Required(tls_params)),
                StartTls::Auto => builder.tls(Tls::Opportunistic(tls_params)),
                StartTls::Never => builder.tls(Tls::None),
            }
        };

        if let (Some(user), Some(pass)) = (config.user.as_ref(), config.pass.as_ref()) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        let transport = builder.build();
        Ok(Self::new_with_transport(config.from, transport))
    }

    fn new_with_transport(from: String, transport: AsyncSmtpTransport<Tokio1Executor>) -> Self {
        // Start periodic cleanup once
        cleanup_handle();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mailer = transport.clone();
                send_with_retry(|| {
                    let mailer = mailer.clone();
                    let msg = msg.clone();
                    async move {
                        mailer
                            .send(msg)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    }
                })
                .await;
            }
        });
        Self { from, sender: tx }
    }

    fn allowed(to: &str) -> Result<bool, EmailError> {
        let mut map = RATE_LIMITS.lock().map_err(|_| EmailError::LockPoisoned)?;
        let now = Instant::now();
        let allowed = match map.get(to) {
            Some(last) if now.duration_since(*last) < RATE_LIMIT => false,
            _ => {
                map.insert(to.to_string(), now);
                true
            }
        };
        Ok(allowed)
    }

    fn queue_mail(&self, email: Message) {
        if self.sender.send(email).is_err() {
            log::warn!("email queue disconnected");
        }
    }

    fn send_mail(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        if !Self::allowed(to)? {
            return Err(EmailError::RateLimited);
        }

        let email = Message::builder()
            .from(self.from.parse().map_err(EmailError::Address)?)
            .to(to.parse().map_err(EmailError::Address)?)
            .subject(subject)
            .body(body.to_string())
            .map_err(EmailError::Build)?;

        self.queue_mail(email);
        Ok(())
    }

    pub fn send_test(&self, to: &str) -> Result<(), EmailError> {
        self.send_mail(to, "Test email", "Arena test message")
    }

    pub fn from_address(&self) -> &str {
        &self.from
    }
}

async fn send_with_retry<F, Fut, E>(mut send: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), E>>,
    E: std::fmt::Display,
{
    let mut delay = RETRY_BASE;
    for _ in 0..MAX_RETRIES {
        match send().await {
            Ok(_) => return,
            Err(e) => {
                log::warn!(
                    "failed to send email: {e}; retrying in {}ms",
                    delay.as_millis()
                );
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }
    log::warn!("giving up after {MAX_RETRIES} attempts");
}

// -- Tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn clear_limits() {
        let mut map = match RATE_LIMITS.lock() {
            Ok(guard) => guard,
            Err(poison) => {
                RATE_LIMITS.clear_poison();
                poison.into_inner()
            }
        };
        map.clear();
    }

    #[test]
    fn rate_limiting() {
        clear_limits();
        assert!(EmailService::allowed("a@example.com").unwrap());
        assert!(!EmailService::allowed("a@example.com").unwrap());
    }

    #[tokio::test]
    async fn invalid_address() {
        clear_limits();
        let mut cfg = SmtpConfig::default();
        cfg.from = "noreply@example.com".into();
        let svc = EmailService::new(cfg).unwrap();
        match svc.send_test("not-an-email") {
            Err(EmailError::Address(_)) => {}
            _ => panic!("expected address error"),
        }
    }

    #[test]
    fn lock_poisoned() {
        clear_limits();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = RATE_LIMITS.lock().unwrap();
            panic!();
        }));
        match EmailService::allowed("b@example.com") {
            Err(EmailError::LockPoisoned) => {}
            _ => panic!("expected lock poisoned"),
        }
        let mut guard = RATE_LIMITS.lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }

    #[tokio::test]
    async fn retries_on_failure() {
        let attempts = AtomicUsize::new(0);
        send_with_retry(|| {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            async move { if n < 2 { Err("fail") } else { Ok(()) } }
        })
        .await;
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
