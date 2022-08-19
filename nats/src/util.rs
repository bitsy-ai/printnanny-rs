use std::collections::HashMap;

use crate::error::CommandError;

// subscribe to commands with any subject prefix
pub fn to_nats_command_subscribe_subject(pi_id: &i32) -> String {
    return format!("pi.{}.command.>", pi_id);
}

// parses output of `systemctl show` into a hashmap
pub fn systemctl_show_payload(stdout: &[u8]) -> Result<HashMap<String, String>, CommandError> {
    let mut result: HashMap<String, String> = HashMap::new();
    let mapping = std::str::from_utf8(&stdout)?.trim().split("\n");

    for line in mapping {
        let split = line.split_once("=");
        if split.is_none() {
            return Err(CommandError::SystemctlParse {
                output: line.to_string(),
            });
        }
        let (key, value) = split.unwrap();
        result.insert(key.to_string(), value.to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn test_systemctl_show_payload() {
        let stdout = r#"
Version=251.1+
Features=-PAM -AUDIT -SELINUX -APPARMOR +IMA -SMACK +SECCOMP -GCRYPT -GNUTLS +OPENSSL +ACL +BLKID -CURL -ELFUTILS -FIDO2 -IDN2 -IDN -IPTC +KMOD -LIBCRYPTSETUP +LIBFDISK -PCRE2 -PWQUALITY -P11KIT -QRENCODE -TPM2 -BZIP2 -LZ4 -XZ -ZLIB +ZSTD -BPF_FRAMEWORK +XKBCOMMON +UTMP +SYSVINIT default-hierarchy=hybrid
Architecture=arm64
Tainted=unmerged-usr:cgroupsv1
FirmwareTimestampMonotonic=0
LoaderTimestampMonotonic=0
KernelTimestamp=Wed 1969-12-31 16:00:00 PST
KernelTimestampMonotonic=0
InitRDTimestampMonotonic=0
UserspaceTimestamp=Wed 1969-12-31 16:00:02 PST
UserspaceTimestampMonotonic=2000182
FinishTimestampMonotonic=0
SecurityStartTimestamp=Wed 1969-12-31 16:00:02 PST
SecurityStartTimestampMonotonic=2042364
SecurityFinishTimestamp=Wed 1969-12-31 16:00:02 PST
SecurityFinishTimestampMonotonic=2048733
GeneratorsStartTimestamp=Tue 2022-05-24 14:32:34 PDT
GeneratorsStartTimestampMonotonic=2239316
GeneratorsFinishTimestamp=Tue 2022-05-24 14:32:34 PDT
GeneratorsFinishTimestampMonotonic=2812515
UnitsLoadStartTimestamp=Tue 2022-05-24 14:32:34 PDT
UnitsLoadStartTimestampMonotonic=2812525
UnitsLoadFinishTimestamp=Tue 2022-05-24 14:32:35 PDT
UnitsLoadFinishTimestampMonotonic=3329561
UnitsLoadTimestamp=Tue 2022-05-24 14:33:08 PDT
UnitsLoadTimestampMonotonic=36319852
InitRDSecurityStartTimestampMonotonic=0
InitRDSecurityFinishTimestampMonotonic=0
InitRDGeneratorsStartTimestampMonotonic=0
InitRDGeneratorsFinishTimestampMonotonic=0
InitRDUnitsLoadStartTimestampMonotonic=0
InitRDUnitsLoadFinishTimestampMonotonic=0
LogLevel=info
LogTarget=journal-or-kmsg
NNames=298
NFailedUnits=1
NJobs=1
NInstalledJobs=20069
NFailedJobs=6
Progress=0.99995
Environment=LANG=en_US.UTF-8 PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ConfirmSpawn=no
ShowStatus=yes
UnitPath=/etc/systemd/system.control /run/systemd/system.control /run/systemd/transient /run/systemd/generator.early /etc/systemd/system /etc/systemd/system.attached /run/systemd/system /run/systemd/system.attached /run/systemd/generator /usr/local/lib/systemd/system /lib/systemd/system /usr/lib/systemd/system /run/systemd/generator.late
DefaultStandardOutput=journal
DefaultStandardError=inherit
RuntimeWatchdogUSec=0
RuntimeWatchdogPreUSec=0
RebootWatchdogUSec=10min
KExecWatchdogUSec=0
ServiceWatchdogs=yes
SystemState=starting
DefaultTimerAccuracyUSec=1min
DefaultTimeoutStartUSec=1min 30s
DefaultTimeoutStopUSec=1min 30s
DefaultTimeoutAbortUSec=1min 30s
DefaultRestartUSec=100ms
DefaultStartLimitIntervalUSec=10s
DefaultStartLimitBurst=5
DefaultCPUAccounting=no
DefaultBlockIOAccounting=no
DefaultMemoryAccounting=yes
DefaultTasksAccounting=yes
DefaultLimitCPU=infinity
DefaultLimitCPUSoft=infinity
DefaultLimitFSIZE=infinity
DefaultLimitFSIZESoft=infinity
DefaultLimitDATA=infinity
DefaultLimitDATASoft=infinity
DefaultLimitSTACK=infinity
DefaultLimitSTACKSoft=8388608
DefaultLimitCORE=infinity
DefaultLimitCORESoft=0
DefaultLimitRSS=infinity
DefaultLimitRSSSoft=infinity
DefaultLimitNOFILE=524288
DefaultLimitNOFILESoft=1024
DefaultLimitAS=infinity
DefaultLimitASSoft=infinity
DefaultLimitNPROC=30010
DefaultLimitNPROCSoft=30010
DefaultLimitMEMLOCK=8388608
DefaultLimitMEMLOCKSoft=8388608
DefaultLimitLOCKS=infinity
DefaultLimitLOCKSSoft=infinity
DefaultLimitSIGPENDING=30010
DefaultLimitSIGPENDINGSoft=30010
DefaultLimitMSGQUEUE=819200
DefaultLimitMSGQUEUESoft=819200
DefaultLimitNICE=0
DefaultLimitNICESoft=0
DefaultLimitRTPRIO=0
DefaultLimitRTPRIOSoft=0
DefaultLimitRTTIME=infinity
DefaultLimitRTTIMESoft=infinity
DefaultTasksMax=9003
TimerSlackNSec=50000
DefaultOOMPolicy=stop
DefaultOOMScoreAdjust=0
CtrlAltDelBurstAction=reboot-force
        "#;
        let result = systemctl_show_payload(stdout.as_bytes()).unwrap();
        println!("{:?}", result);
        assert_eq!(result.get("LogLevel"), Some(&"info".to_string()));
        assert_eq!(
            result.get("Environment"), 
            Some(&"LANG=en_US.UTF-8 PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string())
        );
    }

    #[test_log::test]
    fn test_to_nats_command_subscribe_subject() {
        assert_eq!(to_nats_command_subscribe_subject(&3), "pi.3.command.>");
    }
}
