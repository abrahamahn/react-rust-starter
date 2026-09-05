use crate::{app::config::Config, platform::http::Error};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

// Links contain credentials: never derive Debug, persist, or log this payload.
pub struct Delivery {
    pub to: String,
    pub subject: String,
    pub text: String,
}
#[derive(Clone)]
pub struct Mailer {
    sender: mpsc::Sender<Delivery>,
}
impl Mailer {
    /// Shared with integration tests; never exposed by an HTTP endpoint.
    pub fn channel() -> (Self, mpsc::Receiver<Delivery>) {
        let (sender, receiver) = mpsc::channel(32);
        (Self { sender }, receiver)
    }
    pub fn reserve(&self) -> Result<mpsc::OwnedPermit<Delivery>, Error> {
        self.sender
            .clone()
            .try_reserve_owned()
            .map_err(|_| Error::Unavailable)
    }
    pub fn start(config: &Config) -> Result<(Self, JoinHandle<()>), Error> {
        let from: Mailbox = config
            .mail_from
            .parse()
            .map_err(|_| Error::Config("invalid MAIL_FROM"))?;
        let mut builder = if config.production {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                .map_err(|_| Error::Config("invalid SMTP host"))?
        } else {
            // Config permits unencrypted SMTP only to explicit loopback development IPs.
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
        }
        .port(config.smtp_port)
        .timeout(Some(Duration::from_secs(8)));
        if let (Some(user), Some(password)) = (&config.smtp_user, &config.smtp_password) {
            builder = builder.credentials(Credentials::new(user.clone(), password.clone()));
        }
        let transport = builder.build();
        let (mailer, mut receiver) = Self::channel();
        let handle = tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                let message = delivery.to.parse::<Mailbox>().ok().and_then(|to| {
                    Message::builder()
                        .from(from.clone())
                        .to(to)
                        .subject(delivery.subject)
                        .body(delivery.text)
                        .ok()
                });
                let success = match message {
                    Some(message) => transport.send(message).await.is_ok(),
                    None => false,
                };
                // No recipient, raw provider error, link, or token in logs.
                if !success {
                    tracing::warn!(
                        event = "mail_delivery_failed",
                        "Delivery failed; user can request a fresh link"
                    );
                }
            }
        });
        Ok((mailer, handle))
    }
}
