use std::io::Read;
use std::path::PathBuf;

use printnanny_settings::error::PrintNannySettingsError;

use super::error::ServiceError;
use super::file::open;

/// Represents the data from `/proc/cpuinfo`.
///
/// The `fields` field stores the fields that are common among all CPUs.  The `cpus` field stores
/// CPU-specific info.
///
/// For common fields, there are methods that will return the data, converted to a more appropriate
/// data type.  These methods will all return `None` if the field doesn't exist, or is in some
/// unexpected format (in that case, you'll have to access the string data directly).
#[derive(Debug, Clone)]
pub struct RpiCpuInfo {
    /// This stores fields that are common among all CPUs
    pub model: Option<String>,
    pub revision: Option<String>,
    pub hardware: Option<String>,
    pub serial: Option<String>,
}

impl RpiCpuInfo {
    /// Get CpuInfo from a custom Read instead of the default `/proc/cpuinfo`.
    pub fn from_reader<R: Read>(r: R) -> Self {
        use std::io::{BufRead, BufReader};

        let reader = BufReader::new(r);

        let mut model: Option<String> = None;
        let mut revision: Option<String> = None;
        let mut hardware: Option<String> = None;
        let mut serial: Option<String> = None;

        for line in reader.lines().flatten() {
            if !line.is_empty() {
                let mut s = line.split(':');
                let key = s.next().unwrap();
                if let Some(value) = s.next() {
                    let key = key.trim();
                    let value = value.trim();
                    match key {
                        "Model" => model = Some(value.to_string()),
                        "Hardware" => hardware = Some(value.to_string()),
                        "Revision" => revision = Some(value.to_string()),
                        "Serial" => serial = Some(value.to_string()),
                        _ => (),
                    };
                }
            }
        }
        RpiCpuInfo {
            model,
            hardware,
            revision,
            serial,
        }
    }
    pub fn new() -> Result<Self, ServiceError> {
        let path = "/proc/cpuinfo";
        let file = match open(&path) {
            Ok(f) => Ok(f),
            Err(error) => Err(PrintNannySettingsError::ReadIOError {
                path: PathBuf::from(path),
                error,
            }),
        }?;
        Ok(RpiCpuInfo::from_reader(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cpuinfo_rpi() {
        // My rpi system includes some stuff at the end of /proc/cpuinfo that we shouldn't parse
        let data = r#"processor       : 0
model name      : ARMv7 Processor rev 4 (v7l)
BogoMIPS        : 38.40
Features        : half thumb fastmult vfp edsp neon vfpv3 tls vfpv4 idiva idivt vfpd32 lpae evtstrm crc32
CPU implementer : 0x41
CPU architecture: 7
CPU variant     : 0x0
CPU part        : 0xd03
CPU revision    : 4

processor       : 1
model name      : ARMv7 Processor rev 4 (v7l)
BogoMIPS        : 38.40
Features        : half thumb fastmult vfp edsp neon vfpv3 tls vfpv4 idiva idivt vfpd32 lpae evtstrm crc32
CPU implementer : 0x41
CPU architecture: 7
CPU variant     : 0x0
CPU part        : 0xd03
CPU revision    : 4

processor       : 2
model name      : ARMv7 Processor rev 4 (v7l)
BogoMIPS        : 38.40
Features        : half thumb fastmult vfp edsp neon vfpv3 tls vfpv4 idiva idivt vfpd32 lpae evtstrm crc32
CPU implementer : 0x41
CPU architecture: 7
CPU variant     : 0x0
CPU part        : 0xd03
CPU revision    : 4

processor       : 3
model name      : ARMv7 Processor rev 4 (v7l)
BogoMIPS        : 38.40
Features        : half thumb fastmult vfp edsp neon vfpv3 tls vfpv4 idiva idivt vfpd32 lpae evtstrm crc32
CPU implementer : 0x41
CPU architecture: 7
CPU variant     : 0x0
CPU part        : 0xd03
CPU revision    : 4

Hardware        : BCM2835
Revision        : a020d3
Serial          : 0000000012345678
Model           : Raspberry Pi 3 Model B Plus Rev 1.3
"#;

        let r = std::io::Cursor::new(data.as_bytes());

        let info = RpiCpuInfo::from_reader(r);
        assert_eq!(info.hardware, Some("BCM2835".to_string()));
        assert_eq!(info.revision, Some("a020d3".to_string()));
        assert_eq!(info.serial, Some("0000000012345678".to_string()));
        assert_eq!(
            info.model,
            Some("Raspberry Pi 3 Model B Plus Rev 1.3".to_string())
        );
    }
}
