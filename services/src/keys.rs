use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostKeyError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    OpenSSHKeyError(#[from] openssh_keys::errors::OpenSSHKeyError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostKeys {
    path: PathBuf,
}

impl Default for HostKeys {
    fn default() -> Self {
        let path = "/etc/ssh".into();
        Self { path }
    }
}

impl HostKeys {
    pub fn ecdsa_private_key_file(&self) -> PathBuf {
        self.path.join("ssh_host_ecdsa_key")
    }
    // ecdsa
    pub fn ecdsa_public_key_file(&self) -> PathBuf {
        self.path.join("ssh_host_ecdsa_key.pub")
    }

    pub fn ecdsa_public_key_content(&self) -> Result<String, HostKeyError> {
        Ok(String::from_utf8_lossy(&fs::read(self.ecdsa_public_key_file())?).to_string())
    }

    pub fn ecdsa_public_key(&self) -> Result<openssh_keys::PublicKey, HostKeyError> {
        let file = fs::File::open(self.ecdsa_public_key_file())?;
        let key = openssh_keys::PublicKey::read_keys(file)?[0].clone();
        Ok(key)
    }

    pub fn ecdsa_public_key_fingerprint(&self) -> Result<String, HostKeyError> {
        Ok(self.ecdsa_public_key()?.fingerprint_md5())
    }

    // rsa
    pub fn rsa_private_key_file(&self) -> PathBuf {
        self.path.join("ssh_host_rsa_key")
    }

    pub fn rsa_public_key_file(&self) -> PathBuf {
        self.path.join("ssh_host_rsa_key.pub")
    }

    pub fn rsa_public_key_content(&self) -> Result<String, HostKeyError> {
        Ok(String::from_utf8_lossy(&fs::read(self.rsa_public_key_file())?).to_string())
    }

    pub fn rsa_public_key(&self) -> Result<openssh_keys::PublicKey, HostKeyError> {
        let file = fs::File::open(self.rsa_public_key_file())?;
        let key = openssh_keys::PublicKey::read_keys(file)?[0].clone();
        Ok(key)
    }

    pub fn rsa_public_key_fingerprint(&self) -> Result<String, HostKeyError> {
        Ok(self.rsa_public_key()?.fingerprint_md5())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_ecdsa() {
        let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test");
        let expected_fingerprint = "41:e1:86:05:27:f4:cf:63:5a:4b:1c:60:35:37:08:7d";
        let expected_public_key_content= "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBNOZyjeo+NN5uG5swsrsEDW+Ah8UeTHKAmrCaoTJ2jGeE9mzNfs5noqkwlsxIt0AUsTDX1V5PqAdxDWN0mSqPak= test@test\n";
        let keys = HostKeys { path: test_path };
        assert_eq!(
            keys.ecdsa_public_key_fingerprint().unwrap(),
            expected_fingerprint.to_string()
        );
        assert_eq!(
            keys.ecdsa_public_key_content().unwrap(),
            expected_public_key_content.to_string()
        );
        assert_ne!(keys.ecdsa_private_key_file(), keys.ecdsa_public_key_file());
    }

    fn test_rsa() {
        let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test");
        let expected_fingerprint = "41:e1:86:05:27:f4:cf:63:5a:4b:1c:60:35:37:08:7d";
        let expected_public_key_content= "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7meTeYn54vE6JgdU/o7ApsaJdJHq+9ASGi+B2KTeimO7xfac2AMEwUjfgxBmQ2BXnBTD/LkLVVg5gRoudzYVxWxD5lmJvMIJ5xx/bj7sKfrwdeurOldjQg+ejtCJyu5j27DNFo5BJrFvLLEq864gWIF16Hx38gqocvtVewXCJ+klmZPnLG/G4ElFffyTcMSVFp06MmIVesClf/X2GSX1QXbPrcVA9FcJ35Q33SBP8FaIEhIyvu5uyJ499a3yyTL/NNLEnsS/6MKX1ycTRXg0XK0urIleIdQEef7LuDuk/+tgZWLViIxW7PgCbUUhHsyVw5E/s0Uq7mrgFK0pPCQA1d7qN6ExHsGFDOlJeNZ46+WpM2A8by9U0KZLEE4t57wF7T/VSokEjjx6NdkKAUOVfW4s0M85ymVL9qCSSaDq9yPc+KMWV8F8TY+MiOXvXVsM9qYHlXLhpGUhZm2UdWeSiH2PioGNofYGadmh0ulZwd9M2bOI5JvrI5ozxOJTHCcc= test@test \n";
        let keys = HostKeys { path: test_path };
        assert_eq!(
            keys.rsa_public_key_fingerprint().unwrap(),
            expected_fingerprint.to_string()
        );
        assert_eq!(
            keys.rsa_public_key_content().unwrap(),
            expected_public_key_content.to_string()
        );
        assert_ne!(keys.rsa_private_key_file(), keys.rsa_public_key_file());
    }
}
