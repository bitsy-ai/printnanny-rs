use std::io;
use std::process::Command;

use tempfile::tempfile;
use zip::write::FileOptions;

use printnanny_settings::printnanny::PrintNannySettings;

pub fn disk_usage() -> io::Result<Vec<u8>> {
    let output = Command::new("df").args(["-hT", "--all"]).output()?;
    Ok(output.stdout)
}

pub fn crash_report_zip() -> Result<()> {
    let mut file = tempfile()?;

    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::ZSTD)
        .unix_permissions(0o755);

    let settings = PrintNannySettings::new()?;

    Ok(())
}
