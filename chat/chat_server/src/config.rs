use std::{env, fs::File, path::PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub kafka: KafkaConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub sk: String,
    pub pk: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_kafka_brokers")]
    pub brokers: String,
    #[serde(default = "default_kafka_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default = "default_kafka_client_id")]
    pub client_id: String,
    #[serde(default = "default_kafka_group_id")]
    pub group_id: String,
    #[serde(default)]
    pub consume_enabled: bool,
    #[serde(default)]
    pub consume_topics: Vec<String>,
    #[serde(default = "default_kafka_timeout_ms")]
    pub producer_timeout_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            brokers: default_kafka_brokers(),
            topic_prefix: default_kafka_topic_prefix(),
            client_id: default_kafka_client_id(),
            group_id: default_kafka_group_id(),
            consume_enabled: false,
            consume_topics: Vec::new(),
            producer_timeout_ms: default_kafka_timeout_ms(),
        }
    }
}

fn default_kafka_brokers() -> String {
    "127.0.0.1:9092".to_string()
}

fn default_kafka_topic_prefix() -> String {
    "aicomm".to_string()
}

fn default_kafka_client_id() -> String {
    "chat-server".to_string()
}

fn default_kafka_group_id() -> String {
    "chat-server-bootstrap".to_string()
}

fn default_kafka_timeout_ms() -> u64 {
    1500
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // read from  ./app.yml, or /etc/config/app.yml, or from env CHAT_CONFIG
        let ret = match (
            File::open("chat.yml"),
            File::open("/etc/config/chat.yml"),
            env::var("CHAT_CONFIG"),
        ) {
            (Ok(reader), _, _) => serde_yaml::from_reader(reader),
            (_, Ok(reader), _) => serde_yaml::from_reader(reader),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => bail!("Config file not found"),
        };
        Ok(ret?)
    }
}
