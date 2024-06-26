use tokio::{
    io::AsyncReadExt,
    fs::File
};
use toml;
use rand::Rng;
use serde::{
    Deserialize,
    de::{self, Deserializer, Visitor}
};
use solana_sdk::{
    signature::Keypair,
    pubkey::Pubkey
    };
use std::{
    sync::Arc,
    str::FromStr
};
use solana_client::nonblocking::rpc_client::RpcClient;
use base64::engine::Engine;


lazy_static::lazy_static! {
    pub static ref INF_TOKEN: Pubkey = Pubkey::from_str("5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm")
       .expect("failed to parse inf address");
}


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
    pub web2_client: reqwest::Client,
    pub config: Config
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
    let mut headers: reqwest::header::HeaderMap = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let keys: Vec<Arc<Keypair>> = tokio::fs::read_to_string("./data/keys.txt")
        .await?
        .trim()
        .split("\n")
        .map(Keypair::from_base58_string)
        .map(Arc::new)
        .collect::<Vec<_>>();

    let proxy: Vec<String> = tokio::fs::read_to_string("./data/proxy.txt")
        .await?
        .trim()
        .split("\n")
        .map(String::from)
        .collect::<Vec<_>>();

    let mut accounts: Vec<Account> = Vec::new();
    
    for (key, proxy) in keys.iter().zip(proxy) {
        let client: Arc<RpcClient> = Arc::new(RpcClient::new(config.http_node_url.clone()));

        let web2_client: reqwest::Client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy)?)
            .default_headers(headers.clone())
            .build()?;

        let inf_amount: u64 = if config.amount[0] == config.amount[1] {
            (config.amount[0] * 1_000_000_000f64) as u64
        } else {
            (rand::rngs::ThreadRng::default().gen_range(config.amount[0]..config.amount[1]) * 1_000_000_000f64) as u64
        }; // 10**9

        accounts.push(Account {
            keypair: key.clone(),
            client: client,
            inf_amount: inf_amount,
            web2_client: web2_client,
            config: config.clone()
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