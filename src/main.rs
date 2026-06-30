mod cli;
mod config;
mod error;
mod ioctl;
mod sysfs;
mod metadata;
mod ops;
mod validation;
mod logging;
mod api;
mod daemon;
mod notifications;

use clap::Parser;
use cli::{Cli, Command};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), error::MdError> {
    let args = Cli::parse();
    
    // Initialize logging
    let log_dir = std::path::PathBuf::from("/var/log/rmdadm");
    let log_dir_exists = log_dir.exists();
    
    if let Err(e) = logging::init_logging(
        if log_dir_exists { Some(&log_dir) } else { None },
        args.json
    ) {
        eprintln!("Failed to initialize logging: {}", e);
    }
    
    info!("rmdadm starting with command: {:?}", args.command);

    let result = match args.command {
        Command::Assemble { md_device, components, auto } => {
            if let Some(dev) = auto {
                info!("Auto-assembling array using device {}", dev.display());
                // In a real implementation, we would examine the superblock of `dev` to find the array name and other components.
                // For now, this is a stub.
                Ok(())
            } else if let (Some(md_dev), Some(comps)) = (md_device, components) {
                info!("Assembling array from {} components", comps.len());
                ops::assemble::run(&md_dev, comps, args.dry_run)
            } else {
                Err(error::MdError::ConfigValidation("Must provide either --auto or md_device and components".to_string()))
            }
        }
        Command::Create { md_device, level, raid_devices, metadata, components } => {
            info!("Creating RAID{} array with {} devices", level, raid_devices);
            ops::create::run(&md_device, level, raid_devices, &metadata, components, None, args.dry_run)
        }
        Command::Detail { md_device } => {
            info!("Getting details for array: {}", md_device.display());
            ops::detail::run(&md_device, args.json)
        }
        Command::Stop { md_device, force } => {
            info!("Stopping array: {}", md_device.display());
            ops::manage::stop(&md_device, force, args.dry_run)
        }
        Command::Manage { md_device, add, remove, fail, force } => {
            info!("Managing array: {}", md_device.display());
            ops::manage::manage(&md_device, add, remove, fail, force, args.dry_run)
        }
        Command::Apply { config_file } => {
            info!("Applying configuration from: {}", config_file.display());
            ops::apply::run(&config_file, args.dry_run)
        }
        Command::Monitor => {
            info!("Starting interactive monitor");
            ops::monitor::run()
        }
        Command::Exporter => {
            info!("Starting Prometheus exporter");
            ops::exporter::run().await
        }
        Command::Daemon { addr } => {
            info!("Starting rmdadm daemon (API + Monitoring) on {}", addr);
            
            // Load configuration
            let config = config::Config::load()
                .map_err(|e| error::MdError::Api(format!("Failed to load configuration: {}", e)))?;
            
            let socket_addr: std::net::SocketAddr = addr.parse()
                .map_err(|e| error::MdError::Api(format!("Invalid address: {}", e)))?;
            
            api::start_server(socket_addr, config).await
        }
    };

    match result {
        Ok(_) => {
            info!("Operation completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Operation failed: {}", e);
            Err(e)
        }
    }
}
