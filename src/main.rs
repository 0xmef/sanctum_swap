use std::{
    sync::Arc,
    io::BufRead,
    future::Future,
    pin::Pin,
};
use tokio::sync::Semaphore;
use futures_util::{
    TryFutureExt, 
    FutureExt
};
pub mod utils;
pub mod sanctum;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    print!("\x1B[2J\x1B[1;1H"); // clear terminal

    let config: utils::Config = utils::read_config().await.expect("failed to read config file");
    let accounts: Vec<utils::Account> = utils::prepapre_accounts(config.clone()).await.expect("failed to prepare accounts");

    println!("Upload {} accounts", accounts.len());
    println!("Menu:\n1 - Register sanctum accounts\n2 - Swap SOL to INF\n3 - Check EXP count");

    let stdin: std::io::Stdin = std::io::stdin();
    let mut buffer = String::new();
    stdin.lock().read_line(&mut buffer)?;

    let choice: u64 = buffer.trim().parse::<u64>()?;

    print!("\x1B[2J\x1B[1;1H"); // clear terminal

    let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(config.threads as usize));

    let mut tasks: Vec<tokio::task::JoinHandle<Result<(), anyhow::Error>>> = Vec::new();

    for account in accounts {
        let account_clone = Arc::new(account);
        let future: Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> = match choice {
            1 => {
                Box::pin(async move {
                    Arc::clone(&account_clone).sanctum_register().map_ok(|_| ()).map_err(|e| e.into()).await
                })
            },
            2 => {
                Box::pin(async move {
                    Arc::clone(&account_clone).sanctum_swap().map_ok(|_| ()).map_err(|e| e.into()).await
                })
            },
            3 => {
                Box::pin(async move {
                    Arc::clone(&account_clone).check_profile().map_ok(|_| ()).map_err(|e| e.into()).await
                })
            },
            _ => {
                println!("Invalid choice");
                continue;
            }
        };
        let task = tokio::spawn(process_account(Arc::clone(&semaphore), future).map(|_| Ok(())));
        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

    Ok(())
}


async fn process_account<T>(semaphore: Arc<Semaphore>, action: Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send>>) -> Result<T, anyhow::Error>
where
    T: Send + 'static,
{
    let permit: tokio::sync::OwnedSemaphorePermit = Arc::clone(&semaphore).acquire_owned().await.expect("Failed to acquire permit");
    let _permit: tokio::sync::OwnedSemaphorePermit = permit;
    let task = action.await?;
    Ok(task)
}