use std::path::PathBuf;

use anyhow::Result;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey, spki::der::pem::LineEnding};
use libp2p::{
    Multiaddr, PeerId,
    identity::{self},
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = "chippy";
const CONFIG_FILE_NAME: &str = "Config.toml";
const KEY_FILE_NAME: &str = "key.pem";

#[derive(Serialize, Deserialize, Clone)]
pub struct RelayConfig {
    pub address: Multiaddr,
    pub peer_id: PeerId,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            address: "/ip4/0.0.0.0".parse().unwrap(),
            peer_id: PeerId::random(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentityConfig {
    pub key_file_path: PathBuf,
    pub pre_shared_key: String,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            key_file_path: dirs::config_dir()
                .unwrap()
                .join(CONFIG_DIR_NAME)
                .join(KEY_FILE_NAME),
            pre_shared_key: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub relay: RelayConfig,
    pub identity: IdentityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            relay: RelayConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn default_config_location() -> String {
        let home_dir = dirs::config_dir().expect("Could not find config directory");
        let config_dir = home_dir.join(CONFIG_DIR_NAME);
        std::fs::create_dir_all(&config_dir).expect("Could not create config directory");
        config_dir
            .join(CONFIG_FILE_NAME)
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn load() -> Result<Self> {
        let path = Self::default_config_location();
        Self::load_from_file(&path)
    }

    fn load_from_file(path: &str) -> Result<Self> {
        let config_data = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&config_data)?;
        Ok(config)
    }

    fn save_to_file(&self, path: &str) -> Result<()> {
        let config_data = toml::to_string(self)?;
        std::fs::write(path, config_data)?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::default_config_location();
        self.save_to_file(&path)
    }

    pub fn validate(&self) -> Result<()> {
        if self.identity.pre_shared_key.is_empty() {
            anyhow::bail!(
                "Failed loading config at {}: Pre-shared key cannot be empty",
                Self::default_config_location()
            );
        }

        if self.relay.address.iter().count() == 0 {
            anyhow::bail!(
                "Failed loading config at {}: Relay address cannot be empty",
                Self::default_config_location()
            );
        }

        if self.relay.peer_id.to_string().is_empty() {
            anyhow::bail!(
                "Failed loading config at {}: Relay peer ID cannot be empty",
                Self::default_config_location()
            );
        }

        Ok(())
    }

    fn generate_new_identity(&self) -> Result<()> {
        std::fs::create_dir_all(
            std::path::Path::new(&self.identity.key_file_path)
                .parent()
                .unwrap(),
        )?;

        let keypair = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let pem = keypair.to_pkcs8_pem(LineEnding::LF).unwrap();
        std::fs::write(&self.identity.key_file_path, pem).expect("Unable to write key file");
        Ok(())
    }

    pub fn load_keypair(&self) -> Result<identity::Keypair> {
        std::fs::create_dir_all(
            std::path::Path::new(&self.identity.key_file_path)
                .parent()
                .unwrap(),
        )?;

        if !self.identity.key_file_path.exists() {
            self.generate_new_identity()?;
            return self.load_keypair();
        }

        let pem = std::fs::read_to_string(&self.identity.key_file_path)?;
        let key = ed25519_dalek::SigningKey::from_pkcs8_pem(&pem)?;
        let key_bytes = key.as_bytes();
        Ok(identity::Keypair::ed25519_from_bytes(*key_bytes)?)
    }
}
