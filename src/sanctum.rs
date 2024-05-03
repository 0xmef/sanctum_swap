use serde::{
    Deserialize, Serialize,
};
use std::sync::Arc;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    transaction::VersionedTransaction,
};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::RpcSendTransactionConfig,
    client_error::ClientError,
};
use crate::utils::{
    deserialize_option_f64,
    base64_deserialize
};


#[derive(Deserialize, Serialize, Debug)]
pub struct QuoteResponse {
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[serde(rename = "inAmount")]
    pub in_amount: Option<f64>,
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[serde(rename = "outAmount")]
    pub out_amount: Option<f64>,
    #[serde(rename = "swapSrc")]
    pub swap_src: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SwapRequest {
    pub input: String,
    #[serde(rename = "outputLstMint")]
    pub output_lst_mint: String,
    pub signer: String,
    pub amount: String,
    #[serde(rename = "quotedAmount")]
    pub quoted_amount: String,
    pub mode: String,
    #[serde(rename = "swapSrc")]
    pub swap_src: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SwapResponse {
    #[serde(deserialize_with = "base64_deserialize")]
    pub tx: Vec<u8>,
}

pub async fn get_inf_price() -> anyhow::Result<f64> {
    let url: &str = "https://sanctum-extra-api.shuttleapp.rs/v1/sol-value/current?lst=INF";

    let resp: reqwest::Response = reqwest::get(url).await?;
    let body: String = resp.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    
    let inf_price: f64 = json["solValues"]["INF"].as_str().unwrap().parse::<f64>()?;
    
    Ok(inf_price)
}


pub async fn sanctum_swap(
    sol_lamports_in: u64,
    keypair: Arc<Keypair>,
    client: Arc<RpcClient>,
    max_retries: u32,
) -> anyhow::Result<Signature> {
    let mut headers: reqwest::header::HeaderMap = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let web2_client: reqwest::Client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    for _ in 0..max_retries {
        let quote_response = web2_client.get("https://sanctum-s-api.fly.dev/v1/swap/quote")
            .query(&[
                ("input", "So11111111111111111111111111111111111111112"),
                ("outputLstMint", "5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm"),
                ("amount", &sol_lamports_in.to_string()),
                ("mode", "ExactIn"),
            ])
            .send()
            .await?
            .json::<QuoteResponse>()
            .await?;

        let swap_request: SwapRequest = SwapRequest {
            input: "So11111111111111111111111111111111111111112".to_string(),
            output_lst_mint: "5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm".to_string(),
            signer: keypair.pubkey().to_string(),
            amount: sol_lamports_in.to_string(),
            quoted_amount: quote_response.out_amount.unwrap_or(0.0).to_string(),
            mode: "ExactIn".to_string(),
            swap_src: quote_response.swap_src.to_string(),
        };

        let swap_response: SwapResponse = web2_client.post("https://sanctum-s-api.fly.dev/v1/swap")
            .json(&swap_request)
            .send()
            .await?
            .json::<SwapResponse>()
            .await?;

        let mut versioned_transaction: VersionedTransaction =
            bincode::deserialize(&swap_response.tx).unwrap();

        versioned_transaction
            .message
            .set_recent_blockhash(client.get_latest_blockhash().await?);

        let signed_versioned_transaction =
            VersionedTransaction::try_new(versioned_transaction.message, &[&keypair])?;

        let config: RpcSendTransactionConfig = RpcSendTransactionConfig {
            skip_preflight: true,
            max_retries: Some(2),
            ..Default::default()
        };

        let commitment_config: CommitmentConfig = CommitmentConfig::confirmed();

        let tx: Result<Signature, ClientError> = client
            .send_and_confirm_transaction_with_spinner_and_config(
                &signed_versioned_transaction,
                commitment_config,
                config,
            )
            .await;

        match tx {
            Ok(signature) => {
                println!("SWAP|{:?}", signature);
                return Ok(signature);
            }
            Err(e) => {
                eprintln!("SWAP|Error while sending transaction: {:?}", e);
                let re = regex::Regex::new(r"\b0x1\b").unwrap();
                if re.is_match(&e.to_string()) {
                    println!("SWAP|Stop, not enough balance for {:?}", keypair.pubkey());
                    return Err(Box::new(e).into());
                } else {
                    println!("SWAP|Try again");
                    continue;
                }
            }
        }
    }

    Err(anyhow::anyhow!("SWAP|Failed after {} retries", max_retries))
}