pub struct EmailTemplate;

impl EmailTemplate {
    pub fn verification_email_console(
        recipient_email: &str,
        username: &str,
        verification_link: &str,
    ) -> String {
        format!(
            r"=== EMAIL VERIFICATION ===
To: {}
Subject: Welcome to FediPlace - Verify Your Email

Hi {},

Welcome to FediPlace! Please click the link below to verify your email:
{}

If you didn't create this account, please ignore this email.

Thanks,
The FediPlace Team
=== END EMAIL ===",
            recipient_email, username, verification_link
        )
    }

    pub fn verification_email_html(username: &str, verification_link: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Welcome to FediPlace - Verify Your Email</title>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #f4f4f4; padding: 20px; text-align: center; border-radius: 5px; }}
        .content {{ padding: 20px 0; }}
        .button {{ display: inline-block; padding: 12px 24px; background-color: #007cba; color: white; text-decoration: none; border-radius: 5px; font-weight: bold; }}
        .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; font-size: 0.9em; color: #666; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Welcome to FediPlace!</h1>
    </div>

    <div class="content">
        <p>Hi {},</p>

        <p>Thank you for joining FediPlace! To get started, please verify your email address by clicking the button below:</p>

        <p style="text-align: center; margin: 30px 0;">
            <a href="{}" class="button">Verify Email Address</a>
        </p>

        <p>Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #007cba;">{}</p>

        <p>If you didn't create this account, please ignore this email.</p>
    </div>

    <div class="footer">
        <p>Thanks,<br>The FediPlace Team</p>
    </div>
</body>
</html>"#,
            username, verification_link, verification_link
        )
    }
}
