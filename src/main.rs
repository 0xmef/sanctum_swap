use std::sync::Arc;
use tokio::sync::Semaphore;
pub mod utils;
pub mod sanctum;



#[tokio::main]
async fn main() {
    let config: utils::Config = utils::read_config().await.expect("failed to read config file");
    let accounts: Vec<utils::Account> = utils::prepapre_accounts(config.clone()).await.expect("failed to prepare accounts");

    println!("Upload {} accounts", accounts.len());

    let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(config.threads as usize));

    let mut tasks = Vec::new();

    for account in accounts {
        let permit = Arc::clone(&semaphore).acquire_owned().await.expect("Failed to acquire permit");
        let task = tokio::spawn(async move {
            let _permit = permit;

            let inf_price: f64 = sanctum::get_inf_price().await.expect("failed to get INF price");

            let sol_amount: u64 = (account.inf_amount as f64 * inf_price / 1000000000.0) as u64; 
            
            sanctum::sanctum_swap(
                sol_amount,
                account.keypair, 
                account.client,
                config.max_retries
                ).await
        });
        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

}