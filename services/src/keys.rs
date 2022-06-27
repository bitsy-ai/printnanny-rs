use log::info;
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::error::PrintNannyConfigError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintNannyKeys {
    pub force_create: bool,
    pub path: PathBuf,
}

impl Default for PrintNannyKeys {
    fn default() -> Self {
        let path = "/etc/printnanny/keys".into();
        let force_create = false;
        Self { force_create, path }
    }
}

impl PrintNannyKeys {
    pub fn ec_private_key_file(&self) -> PathBuf {
        self.path.join("ec_private.pem")
    }
    pub fn ec_public_key_file(&self) -> PathBuf {
        self.path.join("ec_public.pem")
    }
    pub fn ec_public_key_fingerprint(&self) -> PathBuf {
        self.path.join("ec_public.sha256")
    }
    fn keypair_exists(&self) -> bool {
        self.ec_private_key_file().exists() && self.ec_public_key_file().exists()
    }

    pub fn read_fingerprint(&self) -> Result<String, PrintNannyConfigError> {
        let contents = fs::read_to_string(self.ec_public_key_fingerprint())?;
        Ok(contents)
    }

    fn _try_generate(&self) -> Result<(), PrintNannyConfigError> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let private_key = EcKey::generate(&group)?;
        let public_key = EcKey::from_public_key(&group, private_key.public_key())?;
        let pcks8_key = PKey::try_from(private_key)?.private_key_to_pem_pkcs8()?;
        // create directory
        if !self.path.exists() {
            fs::create_dir_all(&self.path)?;
        }
        fs::write(self.ec_private_key_file(), pcks8_key)?;
        fs::write(self.ec_public_key_file(), public_key.public_key_to_pem()?)?;

        let public_der = public_key.public_key_to_der()?;
        let fingerprint = hex::encode(sha256(&public_der));
        fs::write(self.ec_public_key_fingerprint(), &fingerprint)?;
        info!(
            "Generated new keypair {:?} and wrote PEM-encoded key parts to {:?}",
            &fingerprint, self.path
        );
        Ok(())
    }
    pub fn try_generate(&self) -> Result<(), PrintNannyConfigError> {
        // check for existence of keys
        match self.keypair_exists() {
            true => match self.force_create {
                true => self._try_generate(),
                false => Err(PrintNannyConfigError::KeypairExists {
                    path: self.path.clone(),
                }),
            },
            false => self._try_generate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_generate_keys() {
        figment::Jail::expect_with(|jail| {
            let keys = PrintNannyKeys {
                path: jail.directory().to_path_buf(),
                force_create: false,
            };
            keys.try_generate().unwrap();
            let result = keys.try_generate();
            assert!(result.is_err());
            Ok(())
        });
    }
    #[test_log::test]
    fn test_generate_overwrite_keys() {
        figment::Jail::expect_with(|jail| {
            let keys = PrintNannyKeys {
                path: jail.directory().to_path_buf(),
                force_create: true,
            };
            keys.try_generate().unwrap();
            let fingerprint1 = keys.read_fingerprint().unwrap();
            keys.try_generate().unwrap();
            let fingerprint2 = keys.read_fingerprint().unwrap();
            assert_ne!(fingerprint1, fingerprint2);
            Ok(())
        });
    }
}
