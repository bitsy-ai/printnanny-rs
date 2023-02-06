use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use log::{debug, error};

use tokio::fs;
use tokio::process::Command;

use printnanny_settings::error::PrintNannySettingsError;
use zip::write::FileOptions;

async fn netstat_routes() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat").args(["--route"]).output().await?;
    Ok(output.stdout)
}

async fn netstat_statistics() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat")
        .args(["--statistics"])
        .output()
        .await?;
    Ok(output.stdout)
}

async fn netstat_groups() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat").args(["--groups"]).output().await?;
    Ok(output.stdout)
}

async fn ifconfig() -> io::Result<Vec<u8>> {
    let output = Command::new("ifconfig").args(["-a", "-v"]).output().await?;
    Ok(output.stdout)
}

async fn disk_usage() -> io::Result<Vec<u8>> {
    let output = Command::new("df").args(["-hT", "--all"]).output().await?;
    Ok(output.stdout)
}

async fn systemd_networkd_logs() -> io::Result<Vec<u8>> {
    let output = Command::new("journalctl")
        .args(["-u", "systemd-networkd.service", "--no-pager"])
        .output()
        .await?;
    Ok(output.stdout)
}

async fn systemd_avahi_daemon_logs() -> io::Result<Vec<u8>> {
    let output = Command::new("journalctl")
        .args(["-u", "avahi-daemon.service", "--no-pager"])
        .output()
        .await?;
    Ok(output.stdout)
}

async fn list_failed_units() -> io::Result<Vec<u8>> {
    let output = Command::new("systemctl")
        .args(["list-units", "--state=failed"])
        .output()
        .await?;
    Ok(output.stdout)
}

pub async fn machine_id() -> io::Result<String> {
    fs::read_to_string("machine-id").await
}

pub async fn write_crash_report_zip(
    file: &File,
    crash_report_paths: Vec<PathBuf>,
) -> Result<(), PrintNannySettingsError> {
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().unix_permissions(0o755);

    // write disk usage to zip
    zip.start_file("disk_usage.txt", options)?;
    zip.write_all(&disk_usage().await?)?;

    // list failed systemd units
    zip.start_file("failed_systemd_units.txt", options)?;
    zip.write_all(&list_failed_units().await?)?;

    zip.start_file("netstat_routes.txt", options)?;
    zip.write_all(&netstat_routes().await?)?;

    zip.start_file("netstat_groups.txt", options)?;
    zip.write_all(&netstat_groups().await?)?;

    zip.start_file("netstat_statistics.txt", options)?;
    zip.write_all(&netstat_statistics().await?)?;

    zip.start_file("ifconfig.txt", options)?;
    zip.write_all(&ifconfig().await?)?;

    zip.start_file("systemd-networkd.service.log", options)?;
    zip.write_all(&systemd_networkd_logs().await?)?;

    zip.start_file("avahi-daemon.service.log", options)?;
    zip.write_all(&systemd_avahi_daemon_logs().await?)?;

    for path in crash_report_paths {
        // read all files in directory
        if path.is_dir() {
            match fs::read_dir(&path).await {
                Ok(mut dir_entries) => {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                        let dir_file_path = entry.path();
                        match zip.start_file(dir_file_path.display().to_string(), options) {
                            Ok(_) => match fs::read(&dir_file_path).await {
                                Ok(contents) => match zip.write_all(&contents) {
                                    Ok(_) => debug!(
                                        "Added file={} to zip={:?}",
                                        dir_file_path.display(),
                                        file
                                    ),
                                    Err(e) => {
                                        error!("Failed to write file={} error={}, contents will be empty in crash report zip", dir_file_path.display(), e);
                                    }
                                },
                                Err(e) => {
                                    error!(
                                        "Failed to read file={} error={}, unable to copy file to crash report zip",
                                        dir_file_path.display(),
                                        e
                                    );
                                }
                            },
                            Err(e) => {
                                error!(
                                    "Failed to start file={} in crash report zip error={}",
                                    dir_file_path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to read directory {} while building crash report, error={}",
                        path.display(),
                        e
                    );
                }
            }
        } else {
            match fs::read(&path).await {
                Ok(content) => match zip.start_file(path.display().to_string(), options) {
                    Ok(_) => match zip.write_all(&content) {
                        Ok(_) => {
                            debug!("Added file={} to zip={:?}", path.display(), file)
                        }
                        Err(e) => {}
                    },
                    Err(e) => {
                        error!(
                            "Failed to start file={} in crash report zip error={}",
                            path.display(),
                            e
                        );
                    }
                },
                Err(e) => {
                    error!(
                        "Failed to read file={} error={}, unable to copy file to crash report zip",
                        path.display(),
                        e
                    );
                }
            };
        }
    }

    zip.finish()?;

    Ok(())
}
