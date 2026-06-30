use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::{MdError, MdResult};
use crate::ioctl::{self, MduArrayInfo, MduDiskInfo, MD_DISK_ACTIVE, MD_DISK_SYNC};
use crate::metadata::{Superblock, v1::{SuperblockV1, MD_SB_MAGIC}};
use crate::validation;
use tracing::{info, warn, debug, instrument};
use std::os::linux::fs::MetadataExt;

const DEFAULT_CHUNK_SIZE: i32 = 512 * 1024; // 512K default chunk size

#[instrument(skip(components))]
pub fn run(md_device: &PathBuf, level: u8, raid_devices: u32, metadata_str: &str, components: Vec<PathBuf>, chunk_size: Option<i32>, dry_run: bool) -> MdResult<()> {
    info!(
        "Creating RAID{} array {} with {} devices (dry_run: {})",
        level,
        md_device.display(),
        raid_devices,
        dry_run
    );
    
    if components.len() as u32 != raid_devices {
        return Err(MdError::InsufficientDevices {
            level,
            needed: raid_devices,
            actual: components.len() as u32,
        });
    }

    // Validate all devices comprehensively
    info!("Validating {} component devices", components.len());
    let device_infos = validation::validate_devices_for_array(&components, level)?;
    
    // Warn about unhealthy devices
    for info in &device_infos {
        if !info.smart_healthy {
            warn!("Device {} has SMART health issues - proceed with caution", info.path.display());
        }
    }

    let minor_version: u32 = match metadata_str {
        "1.0" => 0,
        "1.1" => 1,
        "1.2" => 2,
        _ => {
            warn!("Unknown metadata version '{}', defaulting to 1.2", metadata_str);
            2
        }
    };
    
    debug!("Using metadata version 1.{}", minor_version);

    // Create the MD device via sysfs if it doesn't exist
    if !md_device.exists() {
        info!("Creating MD device via sysfs: {}", md_device.display());
        let md_name = md_device.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MdError::InvalidMetadata("Invalid MD device name".to_string()))?;
        
        // Try to create via /sys/module/md_mod/parameters/new_array
        let new_array_path = "/sys/module/md_mod/parameters/new_array";
        if let Err(e) = fs::write(new_array_path, md_name) {
            debug!("Failed to create via sysfs ({}), device node should exist or will be created by kernel", e);
        } else {
            info!("MD device created via sysfs");
        }
    }

    // Generate a random UUID
    let uuid = uuid::Uuid::new_v4();
    let uuid_bytes = *uuid.as_bytes();
    debug!("Generated UUID for array: {}", uuid);
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| MdError::InvalidMetadata(format!("System time error: {}", e)))?
        .as_secs();
    
    info!("Creating superblocks on {} devices", components.len());

    for (i, comp) in components.iter().enumerate() {
        debug!("Processing device {}/{}: {}", i + 1, components.len(), comp.display());
        
        // Get device size
        let comp_file = OpenOptions::new().read(true).open(comp)?;
        let device_size = {
            let mut size = 0u64;
            use std::os::fd::AsRawFd;
            if unsafe { ioctl::blkgetsize64(comp_file.as_raw_fd(), &mut size) }.is_ok() {
                size
            } else {
                comp_file.metadata()?.len()
            }
        };
        
        // Calculate offsets based on metadata version
        let (data_offset, super_offset, data_size) = match minor_version {
            0 => {
                // 1.0: superblock at end, data at start
                let sb_offset = (device_size & !0x1FFF).saturating_sub(8192) / 512;
                (0, sb_offset, sb_offset)
            },
            1 => {
                // 1.1: superblock at start, data after
                let data_off = 8192 / 512; // 8K in sectors
                (data_off, 0, (device_size / 512).saturating_sub(data_off))
            },
            2 | _ => {
                // 1.2: superblock at 4K, data after
                let data_off = 8192 / 512; // 8K in sectors  
                (data_off, 8, (device_size / 512).saturating_sub(data_off))
            },
        };
        
        // Create device roles array - all devices get the same complete array layout
        let dev_roles: Vec<u16> = (0..raid_devices).map(|idx| idx as u16).collect();
        
        // Write superblock
        let mut sb = SuperblockV1 {
            magic: MD_SB_MAGIC,
            major_version: 1,
            feature_map: 0,
            pad0: 0,
            set_uuid: uuid_bytes,
            set_name: [0; 32],
            ctime: now,
            utime: now,
            level: level as i32,
            layout: 2, // RAID5 left-symmetric
            size: data_size / raid_devices as u64,
            chunksize: chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
            raid_disks: raid_devices as i32,
            bitmap_offset: 0,
            new_level: level as i32,
            reshape_position: u64::MAX, // Not reshaping
            delta_disks: 0,
            new_layout: 0,
            new_chunk: chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
            new_offset: 0,
            data_offset,
            data_size,
            super_offset,
            recovery_offset: u64::MAX, // Fully recovered
            dev_number: i as u32,
            cnt_corrected_read: 0,
            dev_roles,
            sb_csum: 0, // Will be calculated
            events: 0,
            resync_offset: u64::MAX, // Mark as clean/in-sync
            bblog_shift: 0,
            bblog_size: 0,
            bblog_offset: 0,
            max_dev: raid_devices,
            minor_version,
            pad_bytes: Vec::new(),
        };
        
        // Note: Checksum will be calculated by write_to_disk if needed

        // Find major/minor for component
        let comp_meta = std::fs::metadata(comp)
            .map_err(|e| MdError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to get metadata for {}: {}", comp.display(), e)
            )))?;
        let rdev = comp_meta.st_rdev();
        
        let mut disk_info = MduDiskInfo::default();
        disk_info.number = i as i32;
        disk_info.major = ((rdev >> 8) & 0xff) as i32;
        disk_info.minor = (rdev & 0xff) as i32;
        disk_info.raid_disk = i as i32;
        disk_info.state = (1 << MD_DISK_ACTIVE) | (1 << MD_DISK_SYNC);

        if dry_run {
            info!("[DRY RUN] Would write superblock to {}", comp.display());
        } else {
            debug!("Writing superblock to {}", comp.display());
            sb.write_to_disk(comp)
                .map_err(|e| e.context(format!("Failed to write superblock to {}", comp.display())))?;
            info!("Superblock written to {}", comp.display());
        }
    }

    if dry_run {
        info!("[DRY RUN] Would trigger kernel auto-assembly");
        info!("[DRY RUN] Array creation simulation completed successfully");
    } else {
        // For v1.x metadata, trigger kernel auto-assembly
        info!("Triggering kernel auto-assembly");
        
        // Trigger uevents to make kernel scan for MD superblocks
        for comp in &components {
            let dev_name = comp.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let uevent_path = format!("/sys/block/{}/uevent", dev_name);
            if let Err(e) = fs::write(&uevent_path, "change") {
                debug!("Failed to trigger uevent for {}: {}", dev_name, e);
            }
        }
        
        // Give kernel time to auto-assemble
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        info!("Array {} created successfully with UUID {}", md_device.display(), uuid);
        info!("Note: Kernel will auto-assemble the array. Check /proc/mdstat for status.");
    }
    
    Ok(())
}
