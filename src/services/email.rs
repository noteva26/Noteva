//! Email service for sending verification codes

use anyhow::{anyhow, Result};
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::Arc;
use crate::db::repositories::SettingsRepository;

/// Email service for sending emails
pub struct EmailService {
    settings_repo: Arc<dyn SettingsRepository>,
}

impl EmailService {
    pub fn new(settings_repo: Arc<dyn SettingsRepository>) -> Self {
        Self { settings_repo }
    }

    /// Check if email verification is enabled
    pub async fn is_verification_enabled(&self) -> bool {
        if let Ok(Some(setting)) = self.settings_repo.get("email_verification_enabled").await {
            return setting.value == "true";
        }
        false
    }

    /// Send verification code email
    pub async fn send_verification_code(&self, to_email: &str, code: &str) -> Result<()> {
        // Get SMTP settings
        let smtp_host = self.get_setting("smtp_host").await
            .map_err(|_| anyhow!("SMTP host not configured. Please configure SMTP settings first."))?;
        
        if smtp_host.is_empty() {
            return Err(anyhow!("SMTP host not configured. Please configure SMTP settings first."));
        }
        
        let smtp_port: u16 = self.get_setting("smtp_port").await
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);
        let smtp_username = self.get_setting("smtp_username").await
            .map_err(|_| anyhow!("SMTP username not configured"))?;
        let smtp_password = self.get_setting("smtp_password").await
            .map_err(|_| anyhow!("SMTP password not configured"))?;
        let smtp_from = self.get_setting("smtp_from").await
            .map_err(|_| anyhow!("SMTP from address not configured"))?;
        let smtp_from_name = self.get_setting("smtp_from_name").await
            .unwrap_or_else(|_| "Noteva".to_string());
        let site_name = self.get_setting("site_name").await
            .unwrap_or_else(|_| "Noteva".to_string());

        // Build email
        let from = format!("{} <{}>", smtp_from_name, smtp_from);
        let subject = format!("[{}] 邮箱验证码", site_name);
        let body = format!(
            "您好！\n\n您的验证码是：{}\n\n验证码有效期为10分钟，请尽快完成验证。\n\n如果这不是您的操作，请忽略此邮件。\n\n{} 团队",
            code, site_name
        );

        let email = Message::builder()
            .from(from.parse().map_err(|e| anyhow!("Invalid from address: {}", e))?)
            .to(to_email.parse().map_err(|e| anyhow!("Invalid to address: {}", e))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| anyhow!("Failed to build email: {}", e))?;

        // Build SMTP transport
        let creds = Credentials::new(smtp_username, smtp_password);
        
        let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
            .map_err(|e| anyhow!("Failed to create SMTP transport: {}", e))?
            .credentials(creds)
            .port(smtp_port)
            .build();

        // Send email
        mailer.send(email).await
            .map_err(|e| anyhow!("Failed to send email: {}", e))?;

        Ok(())
    }

    /// Send test email
    pub async fn send_test_email(&self, to_email: &str) -> Result<()> {
        let site_name = self.get_setting("site_name").await
            .unwrap_or_else(|_| "Noteva".to_string());
        
        self.send_verification_code(to_email, &format!("TEST-{}", site_name)).await
    }

    async fn get_setting(&self, key: &str) -> Result<String> {
        self.settings_repo
            .get(key)
            .await?
            .map(|s| s.value)
            .ok_or_else(|| anyhow!("Setting '{}' not configured", key))
    }
}

/// Generate a random 6-digit verification code
pub fn generate_verification_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:06}", (seed % 1000000) as u32)
}
