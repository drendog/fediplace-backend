use super::templates::EmailTemplate;
use fedi_wplace_application::{error::AppResult, ports::outgoing::email_sender::EmailSenderPort};
use tracing::{info, instrument};

pub struct ConsoleEmailSender {
    base_url: String,
}

impl ConsoleEmailSender {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait::async_trait]
impl EmailSenderPort for ConsoleEmailSender {
    #[instrument(skip(self, verification_token))]
    async fn send_verification_email(
        &self,
        recipient_email: &str,
        username: &str,
        verification_token: &str,
    ) -> AppResult<()> {
        let verification_link =
            format!("{}/auth/verify?token={}", self.base_url, verification_token);

        let email_content = EmailTemplate::verification_email_console(
            recipient_email,
            username,
            &verification_link,
        );

        info!(
            recipient = recipient_email,
            username = username,
            link = verification_link,
            "📧 EMAIL VERIFICATION LINK (Console Email Sender)"
        );

        info!("{}", email_content);

        Ok(())
    }
}
