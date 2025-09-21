use super::templates::EmailTemplate;
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::email_sender::EmailSenderPort,
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{authentication::Credentials, client::Tls},
};
use std::str::FromStr;
use tracing::{error, info, instrument};

#[derive(Clone)]
pub struct SmtpEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    from_name: String,
    base_url: String,
}

#[derive(Clone)]
pub struct SmtpEmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub base_url: String,
    pub use_tls: bool,
}

impl SmtpEmailSender {
    pub fn new(config: SmtpEmailConfig) -> Result<Self, AppError> {
        let mut transport_builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port);

        if !config.username.is_empty() && !config.password.is_empty() {
            let creds = Credentials::new(config.username, config.password);
            transport_builder = transport_builder.credentials(creds);
        }

        let transport = if config.use_tls {
            transport_builder.build()
        } else {
            transport_builder.tls(Tls::None).build()
        };

        info!(
            smtp_host = %config.smtp_host,
            smtp_port = config.smtp_port,
            from_email = %config.from_email,
            use_tls = config.use_tls,
            "SMTP email sender initialized"
        );

        Ok(Self {
            transport,
            from_email: config.from_email,
            from_name: config.from_name,
            base_url: config.base_url,
        })
    }
}

#[async_trait::async_trait]
impl EmailSenderPort for SmtpEmailSender {
    #[instrument(skip(self, verification_token))]
    async fn send_verification_email(
        &self,
        recipient_email: &str,
        username: &str,
        verification_token: &str,
    ) -> AppResult<()> {
        let verification_link = format!(
            "{}/auth/verify-email?token={}",
            self.base_url, verification_token
        );

        let from_mailbox = Mailbox::from_str(&format!("{} <{}>", self.from_name, self.from_email))
            .map_err(|e| AppError::ExternalServiceError {
                message: format!("Invalid from email address: {}", e),
            })?;

        let to_mailbox =
            Mailbox::from_str(recipient_email).map_err(|e| AppError::ExternalServiceError {
                message: format!("Invalid recipient email address: {}", e),
            })?;

        let email_body = EmailTemplate::verification_email_html(username, &verification_link);

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject("Welcome to FediPlace - Verify Your Email")
            .header(ContentType::TEXT_HTML)
            .body(email_body)
            .map_err(|e| AppError::ExternalServiceError {
                message: format!("Failed to build email message: {}", e),
            })?;

        self.transport.send(email).await.map_err(|e| {
            error!(
                error = %e,
                recipient = recipient_email,
                "Failed to send verification email"
            );
            AppError::ExternalServiceError {
                message: format!("Failed to send email: {}", e),
            }
        })?;

        info!(
            recipient = recipient_email,
            username = username,
            "Verification email sent successfully"
        );

        Ok(())
    }
}
