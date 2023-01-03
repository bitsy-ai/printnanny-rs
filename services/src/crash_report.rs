use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::process::Command;

use log::error;
use printnanny_settings::error::PrintNannySettingsError;
use zip::write::FileOptions;

use printnanny_settings::printnanny::PrintNannySettings;

fn disk_usage() -> io::Result<Vec<u8>> {
    let output = Command::new("df").args(["-hT", "--all"]).output()?;
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

pub fn write_crash_report_zip(file: &File) -> Result<(), PrintNannySettingsError> {
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::ZSTD)
        .unix_permissions(0o755);

    let settings = PrintNannySettings::new()?;
    let mut buffer = Vec::new();

    // write disk usage to zip
    zip.start_file("disk_usage.txt", options)?;
    zip.write_all(&disk_usage()?)?;

    // list failed systemd units
    zip.start_file("failed_systemd_units.txt", options)?;
    zip.write_all(&list_failed_units()?)?;

    for path in settings.paths.crash_report_paths() {
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
                        error!("Failed to read DirEntry={:#?} error={}", &dir_file, e)
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
                    error!("Failed to read file={} error={}", path.display(), e)
                }
            }
        }
    }

    Ok(())
}
