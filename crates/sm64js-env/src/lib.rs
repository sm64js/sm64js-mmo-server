use once_cell::sync::OnceCell;
use std::env;

pub static GOOGLE_CLIENT_ID: OnceCell<String> = OnceCell::new();
pub static GOOGLE_CLIENT_SECRET: OnceCell<String> = OnceCell::new();
pub static DISCORD_CLIENT_ID: OnceCell<String> = OnceCell::new();
pub static DISCORD_CLIENT_SECRET: OnceCell<String> = OnceCell::new();
pub static DISCORD_BOT_TOKEN: OnceCell<String> = OnceCell::new();
pub static REDIRECT_URI: OnceCell<String> = OnceCell::new();
pub static POSTGRES_PASSWORD: OnceCell<String> = OnceCell::new();
pub static DATABASE_URL: OnceCell<String> = OnceCell::new();

pub fn load() {
    dotenv::dotenv().ok();

    if let Ok(mut client_id) = env::var("GOOGLE_CLIENT_ID") {
        if !client_id.ends_with(".apps.googleusercontent.com") {
            client_id += ".apps.googleusercontent.com";
        }
        GOOGLE_CLIENT_ID.set(client_id).unwrap();
    } else {
        GOOGLE_CLIENT_ID
            .set(
                "1000892686951-dkp1vpqohmbq64h7jiiop9v6ic4t1mul.apps.googleusercontent.com"
                    .to_string(),
            )
            .unwrap();
    }

    GOOGLE_CLIENT_SECRET
        .set(env::var("GOOGLE_CLIENT_SECRET").unwrap())
        .unwrap();

    if let Ok(client_id) = env::var("DISCORD_CLIENT_ID") {
        DISCORD_CLIENT_ID.set(client_id).unwrap();
    } else {
        DISCORD_CLIENT_ID
            .set("807123464414429184".to_string())
            .unwrap();
    }

    DISCORD_CLIENT_SECRET
        .set(env::var("DISCORD_CLIENT_SECRET").unwrap())
        .unwrap();

    DISCORD_BOT_TOKEN
        .set(env::var("DISCORD_BOT_TOKEN").unwrap())
        .unwrap();

    if let Ok(uri) = env::var("REDIRECT_URI") {
        REDIRECT_URI.set(uri).unwrap();
    } else {
        REDIRECT_URI
            .set("http://localhost:3060".to_string())
            .unwrap();
    }

    POSTGRES_PASSWORD
        .set(env::var("POSTGRES_PASSWORD").unwrap())
        .unwrap();

    DATABASE_URL.set(env::var("DATABASE_URL").unwrap()).unwrap();
}
