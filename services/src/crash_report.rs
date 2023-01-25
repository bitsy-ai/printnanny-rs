use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

use log::error;
use printnanny_settings::error::PrintNannySettingsError;
use zip::write::FileOptions;

fn netstat_routes() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat").args(["--route"]).output()?;
    Ok(output.stdout)
}

fn netstat_statistics() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat").args(["--statistics"]).output()?;
    Ok(output.stdout)
}

fn netstat_groups() -> io::Result<Vec<u8>> {
    let output = Command::new("netstat").args(["--groups"]).output()?;
    Ok(output.stdout)
}

fn ifconfig() -> io::Result<Vec<u8>> {
    let output = Command::new("ifconfig").args(["-a", "-v"]).output()?;
    Ok(output.stdout)
}

fn disk_usage() -> io::Result<Vec<u8>> {
    let output = Command::new("df").args(["-hT", "--all"]).output()?;
    Ok(output.stdout)
}

fn systemd_networkd_logs() -> io::Result<Vec<u8>> {
    let output = Command::new("journalctl")
        .args(["-u", "systemd-networkd.service", "--no-pager"])
        .output()?;
    Ok(output.stdout)
}

fn systemd_avahi_daemon_logs() -> io::Result<Vec<u8>> {
    let output = Command::new("journalctl")
        .args(["-u", "avahi-daemon.service", "--no-pager"])
        .output()?;
    Ok(output.stdout)
}

fn list_failed_units() -> io::Result<Vec<u8>> {
    let output = Command::new("systemctl")
        .args(["list-units", "--state=failed"])
        .output()?;
    Ok(output.stdout)
}

pub fn machine_id() -> io::Result<String> {
    fs::read_to_string("machine-id")
}

pub fn write_crash_report_zip(
    file: &File,
    crash_report_paths: Vec<PathBuf>,
) -> Result<(), PrintNannySettingsError> {
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().unix_permissions(0o755);
    let mut buffer = Vec::new();

    // write disk usage to zip
    zip.start_file("disk_usage.txt", options)?;
    zip.write_all(&disk_usage()?)?;

    // list failed systemd units
    zip.start_file("failed_systemd_units.txt", options)?;
    zip.write_all(&list_failed_units()?)?;

    zip.start_file("netstat_routes.txt", options)?;
    zip.write_all(&netstat_routes()?)?;

    zip.start_file("netstat_groups.txt", options)?;
    zip.write_all(&netstat_groups()?)?;

    zip.start_file("netstat_statistics.txt", options)?;
    zip.write_all(&netstat_statistics()?)?;

    zip.start_file("ifconfig.txt", options)?;
    zip.write_all(&ifconfig()?)?;

    zip.start_file("systemd-networkd.service.log", options)?;
    zip.write_all(&systemd_networkd_logs()?)?;

    zip.start_file("avahi-daemon.service.log", options)?;
    zip.write_all(&systemd_avahi_daemon_logs()?)?;

    for path in crash_report_paths {
        // read all files in directory
        if path.is_dir() {
            for dir_file in fs::read_dir(&path)? {
                match &dir_file {
                    Ok(dir_file) => {
                        let dir_file_path = dir_file.path();
                        zip.start_file(dir_file_path.display().to_string(), options)?;
                        let mut f = File::open(dir_file_path)?;
                        f.read_to_end(&mut buffer)?;
                        zip.write_all(&buffer)?;
                        buffer.clear();
                    }
                    Err(e) => {
                        error!("Failed to read DirEntry={:#?} error={}", &dir_file, e);
                    }
                }
            }
        } else {
            match File::open(&path) {
                Ok(mut f) => {
                    f.read_to_end(&mut buffer)?;

                    zip.start_file(path.display().to_string(), options)?;

                    zip.write_all(&buffer)?;
                    buffer.clear();
                }
                Err(e) => {
                    error!("Failed to read file={} error={}", path.display(), e);
                }
            }
        }
    }

    zip.finish()?;

    Ok(())
}
