use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use log::{debug, error, warn};

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

fn write_to_zipfile(
    fname: &str,
    content: &[u8],
    writer: &mut zip::ZipWriter<&File>,
    options: FileOptions,
) {
    match writer.start_file(fname, options) {
        Ok(_) => match writer.write_all(content) {
            Ok(_) => debug!("Wrote {} to crash report", fname),
            Err(e) => error!(
                "Failed to write file={} to crash report zip with error={}",
                fname, e
            ),
        },
        Err(e) => {
            error!(
                "Failed to start file={} in crash report zip error={}",
                fname, e
            );
        }
    }
}

pub async fn write_crash_report_zip(
    file: &File,
    crash_report_paths: Vec<PathBuf>,
) -> Result<(), PrintNannySettingsError> {
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().unix_permissions(0o755);

    for path in crash_report_paths {
        // handle path does not exist
        if !path.exists() {
            warn!(
                "Path {} does not exist and will not be included in crash report",
                path.display()
            );
        }
        // read all files in directory
        else if path.is_dir() {
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

    // write disk usage to zip
    let fname = "disk_usage.txt";
    match &disk_usage().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to add disk usage to crash report error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    // list failed systemd units
    let fname = "failed_systemd_units.txt";
    match &list_failed_units().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to list failed systemd units error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    // write netstat routes to zip
    let fname = "netstat_routes.txt";
    match &netstat_routes().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get netstat routes error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    let fname = "netstat_groups.txt";
    match &netstat_groups().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get netstat groups error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    let fname = "netstat_statistics.txt";
    match &netstat_statistics().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get netstat statistics error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    let fname = "ifconfig.txt";
    match &ifconfig().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get ifconfig error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    let fname = "systemd-networkd.service.log";
    match &systemd_networkd_logs().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get systemd-networkd.service logs error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    };

    let fname = "avahi-daemon.service.log";
    match &systemd_avahi_daemon_logs().await {
        Ok(content) => {
            write_to_zipfile(fname, content, &mut zip, options);
        }
        Err(e) => {
            error!("Failed to get avahi-daemon.service logs error={}", e);
            write_to_zipfile(fname, e.to_string().as_bytes(), &mut zip, options);
        }
    }
    zip.finish()?;

    Ok(())
}
