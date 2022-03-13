use std::collections::HashMap;
use std::io;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CMD_SERVICE: &str = "printnanny-cmd";
pub const DASH_SERVICE: &str = "printnanny-dash";
pub const FIRSTBOOT_SERVICE: &str = "printnanny-firstboot";
pub const METADATA_SERVICE: &str = "printnanny-metadata";
pub const MONITOR_SERVICE: &str = "printnanny-monitor";
pub const MQTT_SERVICE: &str = "printnanny-mqtt";
pub const NGINX_SERVICE: &str = "printnanny-ngnx";
pub const OCTOPRINT_SERBICE: &str = "printnanny-octoprint";

pub const SERVICES: &'static [&str] = &[
    CMD_SERVICE,
    DASH_SERVICE,
    FIRSTBOOT_SERVICE,
    METADATA_SERVICE,
    MONITOR_SERVICE,
    MQTT_SERVICE,
];

#[derive(Error, Debug)]
pub enum HealthCheckError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

pub struct SystemctlStatus {
    result: Option<String>,       // Result
    start_time: Option<String>,   // ExecMainStartTimestamp
    exit_time: Option<String>,    // ExecMainExitTimestamp
    start_ts: Option<i32>,        // ExecMainStartTimestampMonotonic
    exit_ts: Option<i32>,         // ExecMainExitTimestampMonotonic
    pid: Option<i32>,             // ExecMainPID
    code: Option<i32>,            // ExecMainCode
    status: Option<i32>,          // ExecMainStatus
    active_state: Option<String>, // ActiveState
    log: Vec<String>,             // journalctl -b -u <unit> -o short-iso
}
pub struct ServiceCheck {
    name: String,
    status: SystemctlStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitState {
    unit: String,
    load: String,
    active: String,
    sub: String,
    description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCheck {
    firstboot_running: bool,
    list_units: Vec<UnitState>,
    boot_history: Vec<String>,
    systemctl_status: String,
}

impl HealthCheck {
    pub fn new() -> Result<Self, HealthCheckError> {
        let systemctl_status = Self::systemctl_status()?;
        let boot_history = Self::boot_history()?;
        let list_units = Self::list_units()?;
        let firstboot_running = Self::firstboot_running()?;
        Ok(Self {
            boot_history,
            firstboot_running,
            list_units,
            systemctl_status,
        })
    }

    pub fn firstboot_running() -> Result<bool, HealthCheckError> {
        let output = Command::new("systemctl")
            .args(&["show", "-p", "SubState", "--value", &FIRSTBOOT_SERVICE])
            .output()?;
        let result = String::from_utf8_lossy(&output.stdout) == "running";
        Ok(result)
    }

    pub fn list_units() -> Result<Vec<UnitState>, HealthCheckError> {
        let output = Command::new("systemctl")
            .args(&[
                "list-units",
                "printnanny*",
                "--no-pager",
                "--all", // list units in both active / inactive states
                "-o",
                "json",
            ])
            .output()?;
        let result: Vec<UnitState> = serde_json::from_slice(output.stdout.as_slice())?;
        Ok(result)
    }

    pub fn systemctl_status() -> Result<String, HealthCheckError> {
        let output = Command::new("systemctl")
            .args(&[
                "status",
                "printnanny*",
                "--no-pager",
                "-l", // show full untruncated output
                "-o",
                "short-iso", // show dates in iso format
            ])
            .output()?;
        let result = String::from_utf8(output.stdout)?;
        Ok(result)
    }

    pub fn boot_history() -> Result<Vec<String>, HealthCheckError> {
        let output = Command::new("journalctl")
            .args(&["--list-boots"])
            .output()?;
        let result = String::from_utf8_lossy(output.stdout.as_slice())
            .split("\n")
            .map(String::from)
            .collect::<Vec<String>>();
        Ok(result)
    }
}

// pub fn parse_systemctl_status(stdout: Vec<u8>, log: Vec<String>) -> SystemctlStatus {
//     let map: HashMap<String, String> = stdout
//         .iter()
//         .map(|kv| from_utf8_lossy(kv).unwrap().split("="))
//         .map(|mut kv| kv.to_tuple2::<String>().unwrap())
//         .collect::<HashMap<_, _>>();
//     SystemctlStatus {
//         result: map.get("Result").map(String::from),
//         start_time: map.get("ExecMainStartTimestamp").map(String::from),
//         start_ts: map.get("ExecMainStartTimestampMonotonic").map(|s| s.parse::<i32>().unwrap()),
//         exit_time: map.get("ExecMainExitTimestamp").map(String::from),
//         exit_ts: map.get("ExecMainExitTimestampMonotonic").map(|s| s.parse::<i32>().unwrap()),
//         pid: map.get("ExecMainPID").map(|s| s.parse::<i32>().unwrap()),
//         code: map.get("ExecMainCode").map(|s| s.parse::<i32>().unwrap()),
//         status: map.get("ExecMainStatus").map(|s| s.parse::<i32>().unwrap()),
//         active_state: map.get("ActiveState").map(String::from),
//         log: log
//     }
// }

// pub fn service_check(name: &str) -> Result<ServiceCheck, io::Error> {
//     let output = Command::new("systemctl")
//         .args(&["show", name, "--no-pager"])
//         .output()?;
//     let log = Command::new("journalctl")
//     .args(&["-b", "-u", name, "-o", "short-iso","--no-pager"])
//     .output()?;
//     let status = parse_systemctl_status(output.stdout, log);
//     Ok(ServiceCheck {
//         status,
//         name: name.into(),
//     })
// }

// pub fn health_check() -> Result<HealthCheck, io::Error> {
//     let mut services: Vec<ServiceCheck> = vec![];
//     for name in SERVICES {
//         let check = service_check(name)?;
//         services.push(check);
//     };
//     HealthCheck {
//         services,
//     }
// }

// #[test]
// fn test_systemctl_status_parser() {
//     let log: Vec<String> = vec![
//         "-- Journal begins at Thu 2022-01-27 19:21:52 PST, ends at Sun 2022-03-13 13:12:54 PDT. --".into()
//     ];
//     let input: Vec<u8> = r#"
//         Type=oneshot
//         Restart=no
//         NotifyAccess=none
//         RestartUSec=100ms
//         TimeoutStartUSec=infinity
//         TimeoutStopUSec=1min 30s
//         TimeoutAbortUSec=1min 30s
//         TimeoutStartFailureMode=terminate
//         TimeoutStopFailureMode=terminate
//         RuntimeMaxUSec=infinity
//         WatchdogUSec=infinity
//         WatchdogTimestampMonotonic=0
//         RootDirectoryStartOnly=no
//         RemainAfterExit=no
//         GuessMainPID=yes
//         MainPID=0
//         ControlPID=0
//         FileDescriptorStoreMax=0
//         NFileDescriptorStore=0
//         StatusErrno=0
//         Result=success
//         ReloadResult=success
//         CleanResult=success
//         UID=[not set]
//         GID=[not set]
//         NRestarts=0
//         OOMPolicy=stop
//         ExecMainStartTimestampMonotonic=0
//         ExecMainExitTimestampMonotonic=0
//         ExecMainPID=0
//         ExecMainCode=0
//         ExecMainStatus=0
//         ExecStart={ path=/usr/bin/flock ; argv[]=/usr/bin/flock --verbose -n /var/run/printnanny/ansible.lock /opt/printnanny/ansible/venv/bin/ansible-playbook -vv --skip-tags packer bitsyai.printnanny.images.octoprint_desktop ; ignore_errors=no ; start_time=[n/a] ; stop_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }
//         ExecStartEx={ path=/usr/bin/flock ; argv[]=/usr/bin/flock --verbose -n /var/run/printnanny/ansible.lock /opt/printnanny/ansible/venv/bin/ansible-playbook -vv --skip-tags packer bitsyai.printnanny.images.octoprint_desktop ; flags= ; start_time=[n/a] ; stop_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }
//         Slice=system.slice
//         MemoryCurrent=[not set]
//         CPUUsageNSec=[not set]
//         EffectiveCPUs=
//         EffectiveMemoryNodes=
//         TasksCurrent=[not set]
//         IPIngressBytes=[no data]
//         IPIngressPackets=[no data]
//         IPEgressBytes=[no data]
//         IPEgressPackets=[no data]
//         IOReadBytes=18446744073709551615
//         IOReadOperations=18446744073709551615
//         IOWriteBytes=18446744073709551615
//         IOWriteOperations=18446744073709551615
//         Delegate=no
//         CPUAccounting=yes
//         CPUWeight=[not set]
//         StartupCPUWeight=[not set]
//         CPUShares=[not set]
//         StartupCPUShares=[not set]
//         CPUQuotaPerSecUSec=infinity
//         CPUQuotaPeriodUSec=infinity
//         AllowedCPUs=
//         AllowedMemoryNodes=
//         IOAccounting=no
//         IOWeight=[not set]
//         StartupIOWeight=[not set]
//         BlockIOAccounting=no
//         BlockIOWeight=[not set]
//         StartupBlockIOWeight=[not set]
//         MemoryAccounting=yes
//         DefaultMemoryLow=0
//         DefaultMemoryMin=0
//         MemoryMin=0
//         MemoryLow=0
//         MemoryHigh=infinity
//         MemoryMax=infinity
//         MemorySwapMax=infinity
//         MemoryLimit=infinity
//         DevicePolicy=auto
//         TasksAccounting=yes
//         TasksMax=4103
//         IPAccounting=no
//         ManagedOOMSwap=auto
//         ManagedOOMMemoryPressure=auto
//         ManagedOOMMemoryPressureLimitPercent=0%
//         UMask=0022
//         LimitCPU=infinity
//         LimitCPUSoft=infinity
//         LimitFSIZE=infinity
//         LimitFSIZESoft=infinity
//         LimitDATA=infinity
//         LimitDATASoft=infinity
//         LimitSTACK=infinity
//         LimitSTACKSoft=8388608
//         LimitCORE=infinity
//         LimitCORESoft=0
//         LimitRSS=infinity
//         LimitRSSSoft=infinity
//         LimitNOFILE=524288
//         LimitNOFILESoft=1024
//         LimitAS=infinity
//         LimitASSoft=infinity
//         LimitNPROC=13677
//         LimitNPROCSoft=13677
//         LimitMEMLOCK=65536
//         LimitMEMLOCKSoft=65536
//         LimitLOCKS=infinity
//         LimitLOCKSSoft=infinity
//         LimitSIGPENDING=13677
//         LimitSIGPENDINGSoft=13677
//         LimitMSGQUEUE=819200
//         LimitMSGQUEUESoft=819200
//         LimitNICE=0
//         LimitNICESoft=0
//         LimitRTPRIO=0
//         LimitRTPRIOSoft=0
//         LimitRTTIME=infinity
//         LimitRTTIMESoft=infinity
//         RootHashSignature=
//         OOMScoreAdjust=0
//         CoredumpFilter=0x33
//         Nice=0
//         IOSchedulingClass=0
//         IOSchedulingPriority=0
//         CPUSchedulingPolicy=0
//         CPUSchedulingPriority=0
//         CPUAffinity=
//         CPUAffinityFromNUMA=no
//         NUMAPolicy=n/a
//         NUMAMask=
//         TimerSlackNSec=50000
//         CPUSchedulingResetOnFork=no
//         NonBlocking=no
//         StandardInput=null
//         StandardInputData=
//         StandardOutput=journal
//         StandardError=inherit
//         TTYReset=no
//         TTYVHangup=no
//         TTYVTDisallocate=no
//         SyslogPriority=30
//         SyslogIdentifier=printnanny-update
//         SyslogLevelPrefix=yes
//         SyslogLevel=6
//         SyslogFacility=3
//         LogLevelMax=-1
//         LogRateLimitIntervalUSec=0
//         LogRateLimitBurst=0
//         SecureBits=0
//         CapabilityBoundingSet=cap_chown cap_dac_override cap_dac_read_search cap_fowner cap_fsetid cap_kill cap_setgid cap_setuid cap_setpcap cap_linux_immutable cap_net_bind_service cap_net_broadcast cap_net_admin cap_net_raw cap_ipc_lock cap_ipc_owner cap_sys_module cap_sys_rawio cap_sys_chroot cap_sys_ptrace cap_sys_pacct cap_sys_admin cap_sys_boot cap_sys_nice cap_sys_resource cap_sys_time cap_sys_tty_config cap_mknod cap_lease cap_audit_write cap_audit_control cap_setfcap cap_mac_override cap_mac_admin cap_syslog cap_wake_alarm cap_block_suspend cap_audit_read cap_perfmon cap_bpf cap_checkpoint_restore
//         AmbientCapabilities=
//         DynamicUser=no
//         RemoveIPC=no
//         MountFlags=
//         PrivateTmp=no
//         PrivateDevices=no
//         ProtectClock=no
//         ProtectKernelTunables=no
//         ProtectKernelModules=no
//         ProtectKernelLogs=no
//         ProtectControlGroups=no
//         PrivateNetwork=no
//         PrivateUsers=no
//         PrivateMounts=no
//         ProtectHome=no
//         ProtectSystem=no
//         SameProcessGroup=no
//         UtmpMode=init
//         IgnoreSIGPIPE=yes
//         NoNewPrivileges=no
//         SystemCallErrorNumber=2147483646
//         LockPersonality=no
//         RuntimeDirectoryPreserve=no
//         RuntimeDirectoryMode=0755
//         StateDirectoryMode=0755
//         CacheDirectoryMode=0755
//         LogsDirectoryMode=0755
//         ConfigurationDirectoryMode=0755
//         TimeoutCleanUSec=infinity
//         MemoryDenyWriteExecute=no
//         RestrictRealtime=no
//         RestrictSUIDSGID=no
//         RestrictNamespaces=no
//         MountAPIVFS=no
//         KeyringMode=private
//         ProtectProc=default
//         ProcSubset=all
//         ProtectHostname=no
//         KillMode=control-group
//         KillSignal=15
//         RestartKillSignal=15
//         FinalKillSignal=9
//         SendSIGKILL=yes
//         SendSIGHUP=no
//         WatchdogSignal=6
//         Id=printnanny-update.service
//         Names=printnanny-update.service
//         Requires=sysinit.target system.slice
//         Wants=network.target printnanny-ansible-collection.service printnanny-metadata.target network-online.target
//         Conflicts=shutdown.target
//         Before=shutdown.target
//         After=sysinit.target printnanny-ansible-collection.service network-online.target basic.target system.slice printnanny-metadata.target systemd-journald.socket network.target
//         Description=Print Nanny Update Service
//         LoadState=loaded
//         ActiveState=inactive
//         FreezerState=running
//         SubState=dead
//         FragmentPath=/etc/systemd/system/printnanny-update.service
//         UnitFileState=disabled
//         UnitFilePreset=enabled
//         StateChangeTimestampMonotonic=0
//         InactiveExitTimestampMonotonic=0
//         ActiveEnterTimestampMonotonic=0
//         ActiveExitTimestampMonotonic=0
//         InactiveEnterTimestampMonotonic=0
//         CanStart=yes
//         CanStop=yes
//         CanReload=no
//         CanIsolate=no
//         CanFreeze=yes
//         StopWhenUnneeded=no
//         RefuseManualStart=no
//         RefuseManualStop=no
//         AllowIsolate=no
//         DefaultDependencies=yes
//         OnFailureJobMode=replace
//         IgnoreOnIsolate=no
//         NeedDaemonReload=no
//         JobTimeoutUSec=infinity
//         JobRunningTimeoutUSec=infinity
//         JobTimeoutAction=none
//         ConditionResult=no
//         AssertResult=no
//         ConditionTimestampMonotonic=0
//         AssertTimestampMonotonic=0
//         Transient=no
//         Perpetual=no
//         StartLimitIntervalUSec=10s
//         StartLimitBurst=5
//         StartLimitAction=none
//         FailureAction=none
//         SuccessAction=none
//         CollectMode=inactive
//         "#.split("\n").collect::<Vec<String>>().map(|s| s.as_bytes());

//     let output = parse_systemctl_status(inpout, log)?;
//     let expected = SystemctlStatus{
//         result: "success".into(),
//         start_time: ""
//     }
// }
