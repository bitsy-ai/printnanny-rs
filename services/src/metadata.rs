use std::fs::read_to_string;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sysinfo::{DiskExt, System, SystemExt};

use super::cpuinfo::RpiCpuInfo;
use super::error::ServiceError;
use super::os_release::OsRelease;

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Populated from /etc/machine-id
    #[serde(rename = "machine_id")]
    pub machine_id: String,
    /// Populated from /proc/cpuinfo REVISION
    #[serde(rename = "revision")]
    pub revision: String,
    /// Populated from /proc/cpuinfo MODEL
    #[serde(rename = "model")]
    pub model: String,
    /// Populated from /proc/cpuinfo SERIAL
    #[serde(rename = "serial")]
    pub serial: String,
    #[serde(rename = "cores")]
    pub cores: i32,
    #[serde(rename = "ram")]
    pub ram: i64,
    pub os_release: OsRelease,
    /// system uptime (in seconds)
    #[serde(rename = "uptime")]
    pub uptime: i64,
    /// Size of /dev/root filesystem in bytes
    #[serde(rename = "rootfs_size")]
    pub rootfs_size: i64,
    /// Space used in /dev/root filesystem in bytes
    #[serde(rename = "rootfs_used")]
    pub rootfs_used: i64,
    /// Size of /dev/mmcblk0p1 filesystem in bytes
    #[serde(rename = "bootfs_size")]
    pub bootfs_size: i64,
    /// Space used in /dev/mmcblk0p1 filesystem in bytes
    #[serde(rename = "bootfs_used")]
    pub bootfs_used: i64,
    /// Size of /dev/mmcblk0p4 filesystem in bytes
    #[serde(rename = "datafs_size")]
    pub datafs_size: i64,
    /// Space used in /dev/mmcblk0p4 filesystem in bytes
    #[serde(rename = "datafs_used")]
    pub datafs_used: i64,
}

pub fn system_info() -> Result<SystemInfo, ServiceError> {
    let machine_id: String = read_to_string("/etc/machine-id")?;

    let mut sys = System::new_all();
    sys.refresh_all();

    // hacky parsing of rpi-specific /proc/cpuinfo
    let rpi_cpuinfo = RpiCpuInfo::new()?;
    let model = rpi_cpuinfo.model.unwrap_or_else(|| "unknown".to_string());
    let serial = rpi_cpuinfo.serial.unwrap_or_else(|| "unknown".to_string());
    let revision = rpi_cpuinfo
        .revision
        .unwrap_or_else(|| "unknown".to_string());

    let cpuinfo = procfs::CpuInfo::new()?;
    let cores: i32 = cpuinfo.num_cores().try_into().unwrap();
    let meminfo = procfs::Meminfo::new()?;
    let ram = meminfo.mem_total.try_into().unwrap();

    let os_release = OsRelease::new_from("/etc/os-release")?;

    let mut bootfs_used: i64 = 0;
    let mut bootfs_size: i64 = 0;
    let bootfs_mountpoint = PathBuf::from("/dev/mmcblk0p1");

    let mut datafs_used: i64 = 0;
    let mut datafs_size: i64 = 0;
    let datafs_mountpoint = PathBuf::from("/dev/mmcblk0p4");

    let mut rootfs_used: i64 = 0;
    let mut rootfs_size: i64 = 0;
    let rootfs_mountpoint = PathBuf::from("/");

    for disk in sys.disks() {
        if disk.mount_point() == rootfs_mountpoint {
            rootfs_size = disk.total_space() as i64;
            rootfs_used = rootfs_size - disk.available_space() as i64;
        } else if disk.mount_point() == bootfs_mountpoint {
            bootfs_size = disk.total_space() as i64;
            bootfs_used = bootfs_size - disk.available_space() as i64;
        } else if disk.mount_point() == datafs_mountpoint {
            datafs_size = disk.total_space() as i64;
            datafs_used = datafs_size - disk.available_space() as i64;
        }
    }
    let uptime = sys.uptime() as i64;

    let info = SystemInfo {
        machine_id,
        serial,
        revision,
        model,
        cores,
        ram,
        bootfs_size,
        bootfs_used,
        datafs_size,
        datafs_used,
        rootfs_size,
        rootfs_used,
        uptime,
        os_release,
    };
    Ok(info)
}
