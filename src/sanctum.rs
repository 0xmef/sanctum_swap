use serde::{
    Deserialize, Serialize,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Signature, Signer},
    transaction::VersionedTransaction,
};
use solana_client::{
    rpc_config::RpcSendTransactionConfig,
    client_error::ClientError,
};
use crate::utils::{
    deserialize_option_f64,
    base64_deserialize,
    Account
};


#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    #[serde(deserialize_with = "deserialize_option_f64")]
    out_amount: Option<f64>,
    swap_src: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SwapRequest {
    input: String,
    output_lst_mint: String,
    signer: String,
    amount: String,
    quoted_amount: String,
    mode: String,
    swap_src: String,
}

#[derive(Deserialize, Debug)]
struct SwapResponse {
    #[serde(deserialize_with = "base64_deserialize")]
    tx: Vec<u8>,
}


impl Account {

    pub async fn get_inf_price(&self) -> anyhow::Result<f64> {
        let url: &str = "https://sanctum-extra-api.shuttleapp.rs/v1/sol-value/current?lst=INF";
        
        let resp: reqwest::Response = self.web2_client
            .get(url)
            .send()
            .await?;

        let body: String = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        
        let inf_price: f64 = json["solValues"]["INF"].as_str().unwrap().parse::<f64>()?;
        
        Ok(inf_price)
    }

    pub async fn check_profile(&self) -> anyhow::Result<u64> {
        let user_profile = self.web2_client.get("https://sanctum-wonderland-season1-api.shuttleapp.rs/s1/user/full")
            .query(&[("pk", &self.keypair.pubkey().to_string())])
            .send()
            .await?;

        match user_profile.status() {
            reqwest::StatusCode::OK => {
                let exp: u64 = user_profile.json::<serde_json::Value>().await?["totalExp"].to_string().parse::<u64>()?;
                println!("{} have {} exp", self.keypair.pubkey(), exp);
                return  Ok(exp);
            },
            _ => {
                return Err(anyhow::anyhow!("User not registered"));
            }            
        }
    }


    pub async fn sanctum_register(&self) -> anyhow::Result<()> {
        let user_exp: Result<u64, anyhow::Error> = self.check_profile().await;

        match user_exp {
            Ok(_) => {
                println!("Skip {} already registered", self.keypair.pubkey());
                return Ok(());
            },
            Err(_) => {
                println!("{} not registered, start registering", self.keypair.pubkey());
            }
        }

        let message: String = "WAT IS WONDERLAND".to_string();

        let signature: Signature = self.keypair.sign_message(message.as_bytes());
        let signature_base58: String = bs58::encode(signature).into_string();

        let user_register = self.web2_client.post("https://sanctum-wonderland-season1-api.shuttleapp.rs/s1/onboard")
            .json(&serde_json::json!({
                "pk": self.keypair.pubkey().to_string(),
                "sig": signature_base58,
                "msg": "V0FUIElTIFdPTkRFUkxBTkQ=",
            }))
            .send()
            .await?;
        
        println!("User registered with {:?}", user_register.text().await?);

        Ok(())
    }

    pub async fn sanctum_swap(&self) -> anyhow::Result<Signature> {
        for _ in 0..self.config.max_retries {

            let inf_price: f64 = self.get_inf_price().await.expect("failed to get INF price");

            let sol_lamports_in: u64 = (self.inf_amount as f64 * inf_price / 1000000000.0) as u64; 

            let quote_response: QuoteResponse = self.web2_client.get("https://sanctum-s-api.fly.dev/v1/swap/quote")
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

            let swap_request_data: SwapRequest = SwapRequest {
                input: "So11111111111111111111111111111111111111112".to_string(),
                output_lst_mint: "5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm".to_string(),
                signer: self.keypair.pubkey().to_string(),
                amount: sol_lamports_in.to_string(),
                quoted_amount: quote_response.out_amount.unwrap_or(0.0).to_string(),
                mode: "ExactIn".to_string(),
                swap_src: quote_response.swap_src.to_string(),
            };

            let swap_response: SwapResponse = self.web2_client.post("https://sanctum-s-api.fly.dev/v1/swap")
                .json(&swap_request_data)
                .send()
                .await?
                .json::<SwapResponse>()
                .await?;

            let mut versioned_transaction: VersionedTransaction =
                bincode::deserialize(&swap_response.tx)?;

            versioned_transaction
                .message
                .set_recent_blockhash(self.client.get_latest_blockhash().await?);

            let signed_versioned_transaction =
                VersionedTransaction::try_new(versioned_transaction.message, &[&self.keypair])?;

            let config: RpcSendTransactionConfig = RpcSendTransactionConfig {
                skip_preflight: true,
                max_retries: Some(2),
                ..Default::default()
            };

            let commitment_config: CommitmentConfig = CommitmentConfig::confirmed();

            let tx: Result<Signature, ClientError> = self.client
                .send_and_confirm_transaction_with_spinner_and_config(
                    &signed_versioned_transaction,
                    commitment_config,
                    config,
                )
                .await;

            match tx {
                Ok(signature) => {
                    println!("SWAP|{} SOL > {} INF\nHash:{:?}",
                    sol_lamports_in as f64 / 1000000000.0,
                    self.inf_amount as f64 / 1000000000.0, 
                    signature);
                    return Ok(signature);
                }
                Err(e) => {
                    eprintln!("SWAP|Error while sending transaction: {:?}", e);
                    let re: regex::Regex = regex::Regex::new(r"\b0x1\b")?;
                    if re.is_match(&e.to_string()) {
                        println!("SWAP|Stop, not enough balance for {:?}", self.keypair.pubkey());
                        return Err(Box::new(e).into());
                    } else {
                        println!("SWAP|Try again");
                        continue;
                    }
                }
            }
        }

        Err(anyhow::anyhow!("SWAP|Failed after {} retries", self.config.max_retries))
    }
}