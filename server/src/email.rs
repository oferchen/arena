use lettre::address::AddressError;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static RATE_LIMITS: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static CLEANUP: Lazy<()> = Lazy::new(|| {
    std::thread::spawn(|| loop {
        std::thread::sleep(CLEANUP_INTERVAL);
        let now = Instant::now();
        let mut map = match RATE_LIMITS.lock() {
            Ok(m) => m,
            Err(poison) => poison.into_inner(),
        };
        map.retain(|_, &mut instant| now.duration_since(instant) < RATE_LIMIT);
    });
});
const RATE_LIMIT: Duration = Duration::from_secs(60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum EmailError {
    RateLimited,
    Smtp(lettre::transport::smtp::Error),
    Address(AddressError),
    Build(lettre::error::Error),
    LockPoisoned,
}

impl From<lettre::transport::smtp::Error> for EmailError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        EmailError::Smtp(err)
    }
}

pub struct EmailService {
    mailer: SmtpTransport,
    from: String,
}

impl EmailService {
    pub fn new(smtp_server: &str, username: &str, password: &str, from: &str) -> Self {
        let creds = Credentials::new(username.to_string(), password.to_string());
        let mailer = SmtpTransport::relay(smtp_server)
            .expect("invalid SMTP server")
            .credentials(creds)
            .build();
        // Start periodic cleanup once
        Lazy::force(&CLEANUP);
        Self {
            mailer,
            from: from.to_string(),
        }
    }

    fn allowed(to: &str) -> Result<bool, EmailError> {
        let mut map = RATE_LIMITS
            .lock()
            .map_err(|_| EmailError::LockPoisoned)?;
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

        self.mailer.send(&email)?;
        Ok(())
    }

    pub fn send_registration_password(&self, to: &str, password: &str) -> Result<(), EmailError> {
        let subject = "Registration Password";
        let body = format!("Your registration password is: {}", password);
        self.send_mail(to, subject, &body)
    }

    pub fn send_verification_link(&self, to: &str, link: &str) -> Result<(), EmailError> {
        let subject = "Verify Your Account";
        let body = format!("Click the following link to verify your account: {}", link);
        self.send_mail(to, subject, &body)
    }

    pub fn send_otp_code(&self, to: &str, code: &str) -> Result<(), EmailError> {
        let subject = "Your OTP Code";
        let body = format!("Your one-time passcode is: {}", code);
        self.send_mail(to, subject, &body)
    }

    pub fn send_password_reset(&self, to: &str, link: &str) -> Result<(), EmailError> {
        let subject = "Password Reset";
        let body = format!("Reset your password using the following link: {}", link);
        self.send_mail(to, subject, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn invalid_address() {
        clear_limits();
        let svc = EmailService::new("localhost", "", "", "noreply@example.com");
        match svc.send_registration_password("not-an-email", "pw") {
            Err(EmailError::Address(_)) => {}
            _ => panic!("expected address error"),
        }
    }

    #[test]
    fn lock_poisoned() {
        clear_limits();
        let _ = std::panic::catch_unwind(|| {
            let _guard = RATE_LIMITS.lock().unwrap();
            panic!();
        });
        match EmailService::allowed("b@example.com") {
            Err(EmailError::LockPoisoned) => {}
            _ => panic!("expected lock poisoned"),
        }
        let mut guard = RATE_LIMITS.lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
}
