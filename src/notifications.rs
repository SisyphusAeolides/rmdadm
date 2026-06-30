//! Email notification module for RAID array alerts
//! Supports SMTP email notifications for array state changes

use lettre::{
    Message, SmtpTransport, Transport,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
};
use tracing::{info, error, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub from_name: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            smtp_username: String::new(),
            smtp_password: String::new(),
            from_address: "rmdadm@localhost".to_string(),
            from_name: "rmdadm RAID Monitor".to_string(),
            to_addresses: vec![],
            use_tls: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

impl AlertLevel {
    fn emoji(&self) -> &str {
        match self {
            AlertLevel::Info => "ℹ️",
            AlertLevel::Warning => "⚠️",
            AlertLevel::Critical => "🚨",
        }
    }

    fn color(&self) -> &str {
        match self {
            AlertLevel::Info => "#3b82f6",
            AlertLevel::Warning => "#f59e0b",
            AlertLevel::Critical => "#ef4444",
        }
    }
}

pub struct EmailNotifier {
    config: EmailConfig,
}

impl EmailNotifier {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("RMDADM_EMAIL_ENABLED")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(false);

        if !enabled {
            return None;
        }

        let config = EmailConfig {
            enabled: true,
            smtp_host: std::env::var("RMDADM_SMTP_HOST")
                .unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port: std::env::var("RMDADM_SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_username: std::env::var("RMDADM_SMTP_USERNAME").unwrap_or_default(),
            smtp_password: std::env::var("RMDADM_SMTP_PASSWORD").unwrap_or_default(),
            from_address: std::env::var("RMDADM_EMAIL_FROM")
                .unwrap_or_else(|_| "rmdadm@localhost".to_string()),
            from_name: std::env::var("RMDADM_EMAIL_FROM_NAME")
                .unwrap_or_else(|_| "rmdadm RAID Monitor".to_string()),
            to_addresses: std::env::var("RMDADM_EMAIL_TO")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            use_tls: std::env::var("RMDADM_SMTP_TLS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        };

        if config.to_addresses.is_empty() {
            warn!("Email notifications enabled but no recipients configured");
            return None;
        }

        Some(Self::new(config))
    }

    pub async fn send_array_alert(
        &self,
        array_name: &str,
        state: &str,
        level: AlertLevel,
        details: Option<&str>,
    ) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let subject = format!(
            "{} rmdadm Alert: Array {} - {}",
            level.emoji(),
            array_name,
            state.to_uppercase()
        );

        let body = self.build_html_body(array_name, state, &level, details);

        self.send_email(&subject, &body).await
    }

    pub async fn send_disk_alert(
        &self,
        array_name: &str,
        disk_name: &str,
        event: &str,
        level: AlertLevel,
    ) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let subject = format!(
            "{} rmdadm Alert: Disk {} in Array {}",
            level.emoji(),
            disk_name,
            array_name
        );

        let body = format!(
            r#"
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .header {{ background: {}; color: white; padding: 20px; border-radius: 5px; }}
                    .content {{ background: #f9f9f9; padding: 20px; margin-top: 20px; border-radius: 5px; }}
                    .footer {{ margin-top: 20px; font-size: 12px; color: #666; }}
                    .detail {{ background: white; padding: 10px; margin: 10px 0; border-left: 3px solid {}; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h2>{} Disk Event Notification</h2>
                    </div>
                    <div class="content">
                        <p><strong>Array:</strong> {}</p>
                        <p><strong>Disk:</strong> {}</p>
                        <p><strong>Event:</strong> {}</p>
                        <p><strong>Time:</strong> {}</p>
                        <div class="detail">
                            <p><strong>Action Required:</strong></p>
                            <ul>
                                <li>Check array status: <code>rmdadm detail {}</code></li>
                                <li>Review system logs for additional information</li>
                                <li>Consider replacing the disk if it has failed</li>
                            </ul>
                        </div>
                    </div>
                    <div class="footer">
                        <p>This is an automated notification from rmdadm RAID monitoring system.</p>
                        <p>To disable these notifications, update your rmdadm configuration.</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            level.color(),
            level.color(),
            level.emoji(),
            array_name,
            disk_name,
            event,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            array_name
        );

        self.send_email(&subject, &body).await
    }

    fn build_html_body(
        &self,
        array_name: &str,
        state: &str,
        level: &AlertLevel,
        details: Option<&str>,
    ) -> String {
        format!(
            r#"
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .header {{ background: {}; color: white; padding: 20px; border-radius: 5px; }}
                    .content {{ background: #f9f9f9; padding: 20px; margin-top: 20px; border-radius: 5px; }}
                    .footer {{ margin-top: 20px; font-size: 12px; color: #666; }}
                    .alert {{ background: white; padding: 15px; margin: 15px 0; border-left: 4px solid {}; }}
                    .detail {{ background: white; padding: 10px; margin: 10px 0; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h2>{} RAID Array Alert</h2>
                    </div>
                    <div class="content">
                        <div class="alert">
                            <h3>Array Status Change Detected</h3>
                            <p><strong>Array Name:</strong> {}</p>
                            <p><strong>Current State:</strong> {}</p>
                            <p><strong>Alert Level:</strong> {:?}</p>
                            <p><strong>Time:</strong> {}</p>
                        </div>
                        {}
                        <div class="detail">
                            <p><strong>Recommended Actions:</strong></p>
                            <ul>
                                <li>Check array details: <code>rmdadm detail {}</code></li>
                                <li>Review system logs: <code>journalctl -u rmdadm</code></li>
                                <li>Access web dashboard: <a href="http://localhost:8080">http://localhost:8080</a></li>
                                {}
                            </ul>
                        </div>
                    </div>
                    <div class="footer">
                        <p>This is an automated notification from rmdadm RAID monitoring system.</p>
                        <p>Hostname: {}</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            level.color(),
            level.color(),
            level.emoji(),
            array_name,
            state,
            level,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            details.map(|d| format!("<div class='detail'><p><strong>Details:</strong></p><p>{}</p></div>", d)).unwrap_or_default(),
            array_name,
            if matches!(level, AlertLevel::Critical) {
                "<li><strong>URGENT:</strong> Immediate attention required to prevent data loss</li>"
            } else {
                ""
            },
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }

    async fn send_email(&self, subject: &str, body: &str) -> Result<(), String> {
        info!("Sending email notification: {}", subject);

        // Build email message
        let from_mailbox: Mailbox = format!("{} <{}>", self.config.from_name, self.config.from_address)
            .parse()
            .map_err(|e| format!("Invalid from address: {}", e))?;

        let mut email_builder = Message::builder()
            .from(from_mailbox)
            .subject(subject);

        // Add all recipients
        for to_addr in &self.config.to_addresses {
            let to_mailbox: Mailbox = to_addr
                .parse()
                .map_err(|e| format!("Invalid to address {}: {}", to_addr, e))?;
            email_builder = email_builder.to(to_mailbox);
        }

        let email = email_builder
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| format!("Failed to build email: {}", e))?;

        // Create SMTP transport
        let creds = Credentials::new(
            self.config.smtp_username.clone(),
            self.config.smtp_password.clone(),
        );

        let mailer = if self.config.use_tls {
            SmtpTransport::starttls_relay(&self.config.smtp_host)
                .map_err(|e| format!("Failed to create SMTP transport: {}", e))?
                .port(self.config.smtp_port)
                .credentials(creds)
                .build()
        } else {
            SmtpTransport::builder_dangerous(&self.config.smtp_host)
                .port(self.config.smtp_port)
                .credentials(creds)
                .build()
        };

        // Send email
        match mailer.send(&email) {
            Ok(_) => {
                info!("Email notification sent successfully to {} recipients", self.config.to_addresses.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to send email: {}", e);
                Err(format!("Failed to send email: {}", e))
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_default() {
        let config = EmailConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.smtp_port, 587);
        assert!(config.use_tls);
    }

    #[test]
    fn test_alert_level_emoji() {
        assert_eq!(AlertLevel::Info.emoji(), "ℹ️");
        assert_eq!(AlertLevel::Warning.emoji(), "⚠️");
        assert_eq!(AlertLevel::Critical.emoji(), "🚨");
    }
}
