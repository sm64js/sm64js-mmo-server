use once_cell::sync::OnceCell;
use std::env;

pub static GOOGLE_CLIENT_ID: OnceCell<String> = OnceCell::new();
pub static GOOGLE_CLIENT_SECRET: OnceCell<String> = OnceCell::new();
pub static DISCORD_CLIENT_ID: OnceCell<String> = OnceCell::new();
pub static DISCORD_CLIENT_SECRET: OnceCell<String> = OnceCell::new();
pub static DISCORD_BOT_TOKEN: OnceCell<String> = OnceCell::new();
pub static REDIRECT_URI: OnceCell<String> = OnceCell::new();
pub static DATABASE_URL: OnceCell<String> = OnceCell::new();
pub static ENABLE_PLAYER_LIST: OnceCell<bool> = OnceCell::new();
pub static COOKIE_SAME_SITE_NONE: OnceCell<bool> = OnceCell::new();

#[cfg(debug_assertions)]
pub static DEV_ACCOUNT_ID: i32 = -1337;
#[cfg(debug_assertions)]
pub static DEV_GOOGLE_ACCOUNT_ID: &str = "1456789";
#[cfg(debug_assertions)]
pub static DEV_GOOGLE_SESSION_TOKEN: &str = "supersecretgooglesessiontoken";
#[cfg(debug_assertions)]
pub static DEV_GOOGLE_TEST_USER: &str = "GoogleTestUser";

pub fn load() {
    dotenv::dotenv().ok();

    let google_client_id_fallback = || {
        GOOGLE_CLIENT_ID
            .set(
                "1000892686951-dkp1vpqohmbq64h7jiiop9v6ic4t1mul.apps.googleusercontent.com"
                    .to_string(),
            )
            .unwrap();
    };
    if let Ok(mut client_id) = env::var("GOOGLE_CLIENT_ID") {
        if !client_id.is_empty() {
            if !client_id.ends_with(".apps.googleusercontent.com") {
                client_id += ".apps.googleusercontent.com";
            }
            GOOGLE_CLIENT_ID.set(client_id).unwrap();
        } else {
            google_client_id_fallback();
        }
    } else {
        google_client_id_fallback();
    }

    #[cfg(debug_assertions)]
    if let Ok(secret) = env::var("GOOGLE_CLIENT_SECRET") {
        if !secret.is_empty() {
            GOOGLE_CLIENT_SECRET.set(secret).unwrap();
        }
    }
    #[cfg(not(debug_assertions))]
    {
        GOOGLE_CLIENT_SECRET
            .set(env::var("GOOGLE_CLIENT_SECRET").unwrap())
            .unwrap();
        assert!(!GOOGLE_CLIENT_SECRET.get().unwrap().is_empty());
    }

    let discord_client_id_fallback = || {
        DISCORD_CLIENT_ID
            .set("807123464414429184".to_string())
            .unwrap();
    };
    if let Ok(client_id) = env::var("DISCORD_CLIENT_ID") {
        if !client_id.is_empty() {
            DISCORD_CLIENT_ID.set(client_id).unwrap();
        } else {
            discord_client_id_fallback();
        }
    } else {
        discord_client_id_fallback();
    }

    #[cfg(debug_assertions)]
    if let Ok(secret) = env::var("DISCORD_CLIENT_SECRET") {
        if !secret.is_empty() {
            DISCORD_CLIENT_SECRET.set(secret).unwrap();
        }
    }
    #[cfg(not(debug_assertions))]
    {
        DISCORD_CLIENT_SECRET
            .set(env::var("DISCORD_CLIENT_SECRET").unwrap())
            .unwrap();
        assert!(!DISCORD_CLIENT_SECRET.get().unwrap().is_empty());
    }

    #[cfg(debug_assertions)]
    if let Ok(token) = env::var("DISCORD_BOT_TOKEN") {
        if !token.is_empty() {
            DISCORD_BOT_TOKEN.set(token).unwrap();
        }
    }
    #[cfg(not(debug_assertions))]
    {
        DISCORD_BOT_TOKEN
            .set(env::var("DISCORD_BOT_TOKEN").unwrap())
            .unwrap();
        assert!(!DISCORD_BOT_TOKEN.get().unwrap().is_empty());
    }

    if let Ok(uri) = env::var("REDIRECT_URI") {
        REDIRECT_URI.set(uri).unwrap();
    } else {
        REDIRECT_URI
            .set("http://localhost:3060".to_string())
            .unwrap();
    }

    DATABASE_URL.set(env::var("DATABASE_URL").unwrap()).unwrap();

    if let Some(enable) = env::var("ENABLE_PLAYER_LIST")
        .ok()
        .and_then(|e| e.parse().ok())
    {
        ENABLE_PLAYER_LIST.set(enable).unwrap();
    } else {
        ENABLE_PLAYER_LIST.set(false).unwrap();
    }

    if let Some(enable) = env::var("COOKIE_SAME_SITE_NONE")
        .ok()
        .and_then(|e| e.parse().ok())
    {
        COOKIE_SAME_SITE_NONE.set(enable).unwrap();
    } else {
        COOKIE_SAME_SITE_NONE.set(false).unwrap();
    }
}
