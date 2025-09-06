use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static RATE_LIMITS: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));
const RATE_LIMIT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum EmailError {
    RateLimited,
    Smtp(lettre::transport::smtp::Error),
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
        Self {
            mailer,
            from: from.to_string(),
        }
    }

    fn allowed(to: &str) -> bool {
        let mut map = RATE_LIMITS.lock().unwrap();
        let now = Instant::now();
        match map.get(to) {
            Some(last) if now.duration_since(*last) < RATE_LIMIT => false,
            _ => {
                map.insert(to.to_string(), now);
                true
            }
        }
    }

    fn send_mail(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        if !Self::allowed(to) {
            return Err(EmailError::RateLimited);
        }

        let email = Message::builder()
            .from(self.from.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(body.to_string())
            .unwrap();

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
