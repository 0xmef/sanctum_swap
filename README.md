# Sanctum Bot

## 🤖 | Features:
- **Auto registration accounts**
- **Swap SOL to INF**
- **Check EXP/INF balance**


## 📝 | Description:
To use the bot, you need to have a **private key**. 

Before you start swap, you need to **register account**.

To **get points and pet from Sanctum**, you need to swap **min 0.1 SOL**.


## ⚙️ Config (config.toml):

```
Accounts: data > keys.txt (SOL private keys)
Proxies: data > proxies.txt (http://user:pass@ip:port)
```


| Name    | Description                                                                        |
|---------|------------------------------------------------------------------------------------|
| threads | Number of accounts that will work simultaneously                                   |
| amount | range of amount FROM and TO. A random number will be selected. Example: [0.1, 0.2] |
| http_node_url | SOL NODE URL (if you dont have - leave the default value)                          |
| max_retries | maximum number of attempts for failed transactions                         |


## 🚀 | How to start:
1. **Install RUST:**
```bash
https://doc.rust-lang.org/cargo/getting-started/installation.html
```
2. **Clone the repository:**
```bash
git clone this repo
```
3. **Open CMD and Install dependencies:**
```bash
cargo build
```
4. **Run the bot:**
```bash
cargo run
```