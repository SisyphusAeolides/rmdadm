use std::time::Duration;
use tokio::time;
use tracing::{info, debug, warn, error};
use std::fs;
use crate::sysfs::MdSysfs;
use crate::notifications::{EmailNotifier, AlertLevel};

pub async fn run_monitor_loop() {
    info!("Background monitoring task started");
    
    // Initialize email notifier if configured
    let email_notifier = EmailNotifier::from_env();
    if let Some(ref notifier) = email_notifier {
        if notifier.is_enabled() {
            info!("📧 Email notifications enabled");
        }
    }
    
    let mut interval = time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        debug!("Running background array health check");
        
        match check_all_arrays(email_notifier.as_ref()).await {
            Ok(_) => debug!("Array health check completed successfully"),
            Err(e) => error!("Failed to check array health: {}", e),
        }
    }
}

async fn check_all_arrays(email_notifier: Option<&EmailNotifier>) -> Result<(), crate::error::MdError> {
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if name_str.starts_with("md") {
                let sys = MdSysfs::new(&name_str);
                
                match sys.get_array_state() {
                    Ok(state) => {
                        let state_str = state.to_string();
                        if state_str == "degraded" || state_str == "clean" || state_str == "active" {
                            debug!("Array {} is in state {}", name_str, state_str);
                            
                            if state_str == "degraded" {
                                warn!("ARRAY {} IS DEGRADED! Initiating alerts...", name_str);
                                
                                // Send webhook alert
                                send_webhook_alert(&name_str, "degraded").await;
                                
                                // Send email alert if configured
                                if let Some(notifier) = email_notifier {
                                    if let Err(e) = notifier.send_array_alert(
                                        &name_str,
                                        "degraded",
                                        AlertLevel::Critical,
                                        Some("Array has entered degraded state. One or more disks may have failed. Immediate attention required to prevent data loss.")
                                    ).await {
                                        error!("Failed to send email alert: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Could not read state for {}: {}", name_str, e);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn send_webhook_alert(array_name: &str, state: &str) {
    // Attempt to read a webhook URL from an environment variable
    let webhook_url = std::env::var("RMDADM_WEBHOOK_URL").unwrap_or_default();
    
    if webhook_url.is_empty() {
        warn!("Array {} is {}, but RMDADM_WEBHOOK_URL is not set. Cannot send alert.", array_name, state);
        return;
    }
    
    let message = format!("🚨 **rmdadm ALERT**: Array `{}` has entered `{}` state! Immediate attention required.", array_name, state);
    
    let payload = serde_json::json!({
        "text": message
    });
    
    let client = reqwest::Client::new();
    match client.post(&webhook_url)
        .json(&payload)
        .send()
        .await 
    {
        Ok(res) if res.status().is_success() => info!("Webhook alert sent successfully for {}", array_name),
        Ok(res) => error!("Webhook alert failed with status: {}", res.status()),
        Err(e) => error!("Failed to send webhook alert: {}", e),
    }
}
