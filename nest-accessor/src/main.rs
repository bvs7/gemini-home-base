use serde::Deserialize;
use reqwest::Error;

#[derive(Deserialize, Debug)]
struct User {
    login: String,
    id: u32,
}

const AUTH_TOKEN_API: &str = "https://api.home.nest.com/oauth2/access_token"

const ROOT_API: &str = "https://developer-api.nest.com";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let request_url = format!("{root}/{owner}/{repo}/stargazers",
                              root = ROOT_API,
                              owner = "rust-lang-nursery",
                              repo = "rust-cookbook");
    println!("{}", request_url);
    let response = reqwest::get(&request_url).await?;

    let users: Vec<User> = response.json().await?;
    println!("{:?}", users);
    Ok(())
}
