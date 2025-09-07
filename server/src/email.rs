//! Email utilities and rate limiting.
//!
//! A background task periodically purges expired entries from the rate-limit
//! map. The task runs on the Tokio runtime and its [`JoinHandle`] is stored so
//! it can be aborted during shutdown if necessary.

use clap::{Args, ValueEnum};
use lettre::address::AddressError;
use lettre::transport::smtp::{
    authentication::Credentials,
    client::{Tls, TlsParameters},
};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use once_cell::sync::Lazy;
use prometheus::{
    Gauge, IntCounter, IntGaugeVec, register_gauge, register_int_counter, register_int_gauge_vec,
};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;

// -- Configuration ---------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[clap(rename_all = "lowercase")]
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

#[derive(Debug, Clone)]
pub struct ParseStartTlsError(String);

impl fmt::Display for ParseStartTlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid value for --smtp-starttls: {}; expected 'auto', 'always', or 'never'",
            self.0
        )
    }
}

impl std::error::Error for ParseStartTlsError {}

impl std::str::FromStr for StartTls {
    type Err = ParseStartTlsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(StartTls::Auto),
            "always" => Ok(StartTls::Always),
            "never" => Ok(StartTls::Never),
            _ => Err(ParseStartTlsError(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct SmtpConfig {
    #[arg(
        long = "smtp-host",
        env = "ARENA_SMTP_HOST",
        default_value = "localhost"
    )]
    pub host: String,
    #[arg(long = "smtp-port", env = "ARENA_SMTP_PORT", default_value_t = 25)]
    pub port: u16,
    #[arg(
        long = "smtp-from",
        env = "ARENA_SMTP_FROM",
        default_value = "arena@localhost"
    )]
    pub from: String,
    #[arg(long = "smtp-starttls", env = "ARENA_SMTP_STARTTLS", value_enum, default_value_t = StartTls::Auto)]
    pub starttls: StartTls,
    #[arg(long = "smtp-smtps", env = "ARENA_SMTP_SMTPS", default_value_t = false)]
    pub smtps: bool,
    #[arg(
        long = "smtp-timeout-ms",
        env = "ARENA_SMTP_TIMEOUT_MS",
        default_value_t = 10000
    )]
    pub timeout: u64,
    #[arg(long = "smtp-user", env = "ARENA_SMTP_USER")]
    pub user: Option<String>,
    #[arg(long = "smtp-pass", env = "ARENA_SMTP_PASS")]
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

static EMAIL_QUEUED: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("email_queued_total", "Emails queued").unwrap());
static EMAIL_SENT: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("email_sent_total", "Emails sent").unwrap());
static EMAIL_FAILED: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("email_failed_total", "Emails failed").unwrap());
static EMAIL_LAST_ERROR: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("email_last_error", "Last email error", &["error"]).unwrap()
});
static EMAIL_LAST_LATENCY: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!("email_last_latency_seconds", "Last email latency seconds").unwrap()
});

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

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("rate limited")]
    RateLimited,
    #[error("{0}")]
    Smtp(String),
    #[error("{0}")]
    Address(AddressError),
    #[error("{0}")]
    Build(lettre::error::Error),
    #[error("lock poisoned")]
    LockPoisoned,
}

pub struct EmailService {
    from: String,
    sender: UnboundedSender<Message>,
    cleanup: &'static JoinHandle<()>,
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

    pub(crate) fn new_with_transport<T>(from: String, transport: T) -> Self
    where
        T: AsyncTransport + Clone + Send + Sync + 'static,
        T::Error: std::fmt::Display,
    {
        // Start periodic cleanup once
        let cleanup = cleanup_handle();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mailer = transport.clone();
                let start = Instant::now();
                let res = send_with_retry(|| {
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
                let latency = start.elapsed().as_secs_f64();
                EMAIL_LAST_LATENCY.set(latency);
                match res {
                    Ok(()) => {
                        EMAIL_SENT.inc();
                        EMAIL_LAST_ERROR.reset();
                    }
                    Err(e) => {
                        EMAIL_FAILED.inc();
                        EMAIL_LAST_ERROR.reset();
                        EMAIL_LAST_ERROR.with_label_values(&[&e]).set(1);
                    }
                }
            }
        });
        Self {
            from,
            sender: tx,
            cleanup,
        }
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
        EMAIL_QUEUED.inc();
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

    pub fn send_registration_password(&self, to: &str, password: &str) -> Result<(), EmailError> {
        let subject = "Registration Password";
        let body = format!("Your registration password is: {}", password);
        self.send_mail(to, subject, &body)
    }

    #[allow(dead_code)]
    pub fn send_verification_link(&self, to: &str, link: &str) -> Result<(), EmailError> {
        let subject = "Verify Your Account";
        let body = format!("Click the following link to verify your account: {}", link);
        self.send_mail(to, subject, &body)
    }

    #[allow(dead_code)]
    pub fn send_otp_code(&self, to: &str, code: &str) -> Result<(), EmailError> {
        let subject = "Your OTP Code";
        let body = format!("Your one-time passcode is: {}", code);
        self.send_mail(to, subject, &body)
    }

    #[allow(dead_code)]
    pub fn send_password_reset(&self, to: &str, link: &str) -> Result<(), EmailError> {
        let subject = "Password Reset";
        let body = format!("Reset your password using the following link: {}", link);
        self.send_mail(to, subject, &body)
    }

    pub fn send_test(&self, to: &str) -> Result<(), EmailError> {
        self.send_mail(to, "Test email", "Arena test message")
    }

    pub fn from_address(&self) -> &str {
        &self.from
    }

    pub fn abort_cleanup(&self) {
        self.cleanup.abort();
    }
}

async fn send_with_retry<F, Fut, E>(mut send: F) -> Result<(), E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), E>>,
    E: std::fmt::Display,
{
    let mut delay = RETRY_BASE;
    let mut last_err = None;
    for _ in 0..MAX_RETRIES {
        match send().await {
            Ok(_) => return Ok(()),
            Err(e) => {
                log::warn!(
                    "failed to send email: {e}; retrying in {}ms",
                    delay.as_millis()
                );
                last_err = Some(e);
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }
    log::warn!("giving up after {MAX_RETRIES} attempts");
    Err(last_err.expect("no error recorded"))
}

// -- Tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::error::Error as _;
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
    #[serial]
    fn rate_limiting() {
        clear_limits();
        assert!(EmailService::allowed("a@example.com").unwrap());
        assert!(!EmailService::allowed("a@example.com").unwrap());
    }

    #[tokio::test]
    #[serial]
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
    #[ignore]
    fn lock_poisoned() {
        clear_limits();
        let _ = std::thread::spawn(|| {
            let _guard = RATE_LIMITS.lock().unwrap();
            panic!();
        })
        .join();
        let err = EmailService::allowed("b@example.com").unwrap_err();
        assert!(matches!(err, EmailError::LockPoisoned));
        assert!(err.source().is_none());
        let mut guard = RATE_LIMITS.lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
        RATE_LIMITS.clear_poison();
    }

    #[tokio::test]
    #[serial]
    async fn retries_on_failure() {
        let attempts = AtomicUsize::new(0);
        let res = send_with_retry(|| {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            async move { if n < 2 { Err("fail") } else { Ok(()) } }
        })
        .await;
        assert!(res.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    #[serial]
    async fn metrics_counters_increment() {
        EMAIL_QUEUED.reset();
        EMAIL_SENT.reset();
        EMAIL_FAILED.reset();
        EMAIL_LAST_ERROR.reset();
        EMAIL_LAST_LATENCY.set(0.0);

        let transport = lettre::transport::stub::AsyncStubTransport::new_ok();
        let svc = EmailService::new_with_transport("noreply@example.com".into(), transport);
        svc.send_test("ok@example.com").unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(EMAIL_QUEUED.get(), 1);
        assert_eq!(EMAIL_SENT.get(), 1);
        assert_eq!(EMAIL_FAILED.get(), 0);

        EMAIL_QUEUED.reset();
        EMAIL_SENT.reset();
        EMAIL_FAILED.reset();
        EMAIL_LAST_ERROR.reset();
        let transport = lettre::transport::stub::AsyncStubTransport::new_error();
        let svc = EmailService::new_with_transport("noreply@example.com".into(), transport);
        svc.send_test("fail@example.com").unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(EMAIL_QUEUED.get(), 1);
        assert_eq!(EMAIL_SENT.get(), 0);
        assert_eq!(EMAIL_FAILED.get(), 1);
        let gauge = EMAIL_LAST_ERROR
            .get_metric_with_label_values(&["stub error"])
            .unwrap();
        assert_eq!(gauge.get(), 1);
    }
}
