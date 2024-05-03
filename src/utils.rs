use tokio::{
    io::AsyncReadExt,
    fs::File
};
use toml;
use rand::Rng;
use serde::{
    Deserialize,
    de::{self, Deserializer, Visitor}
};use solana_sdk::signature::Keypair;
use std::{
    sync::Arc,
    str::FromStr
};
use solana_client::nonblocking::rpc_client::RpcClient;
use base64::engine::Engine;


#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub threads: u32,
    pub amount: [f64; 2],
    pub http_node_url: String,
    pub max_retries: u32
}

#[derive(Clone)]
pub struct Account {
    pub keypair: Arc<Keypair>,
    pub client: Arc<RpcClient>,
    pub inf_amount: u64,
}

pub async fn read_config(
) -> anyhow::Result<Config> {

    let mut file: File = File::open("config.toml").await.expect("config file not found");
    let mut contents: String = String::new();
    file.read_to_string(&mut contents).await.expect("something went wrong reading the file");

    Ok(toml::from_str(&contents)?)
}

pub async fn prepapre_accounts(
    config: Config
) -> anyhow::Result<Vec<Account>> {
    let keys: Vec<Arc<Keypair>> = tokio::fs::read_to_string("./keys.txt")
        .await?
        .trim()
        .split("\n")
        .map(Keypair::from_base58_string)
        .map(Arc::new)
        .collect::<Vec<_>>();

    let mut accounts: Vec<Account> = Vec::new();
    
    for key in keys {
        let client: Arc<RpcClient> = Arc::new(RpcClient::new(config.http_node_url.clone()));

        let inf_amount: u64 = if config.amount[0] == config.amount[1] {
            (config.amount[0] * 1_000_000_000f64) as u64
        } else {
            (rand::rngs::ThreadRng::default().gen_range(config.amount[0]..config.amount[1]) * 1_000_000_000f64) as u64
        }; // 10**9

        accounts.push(Account {
            keypair: key,
            client: client,
            inf_amount: inf_amount,
        });
    }

    Ok(accounts)
}



pub struct OptionF64Visitor;

impl<'de> Visitor<'de> for OptionF64Visitor {
    type Value = Option<f64>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a stringified float or null")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        f64::from_str(&s).map(Some).map_err(de::Error::custom)
    }
}


pub fn deserialize_option_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_option(OptionF64Visitor)
}


pub fn base64_deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    base64::prelude::BASE64_STANDARD.decode(s.as_bytes()).map_err(serde::de::Error::custom)
}