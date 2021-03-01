mod auth;
mod identity;

pub use auth::Auth;
pub use identity::Identity;

#[derive(Clone, Debug)]
pub struct AuthInfo(sm64js_db::AuthInfo);

impl AuthInfo {
    pub fn into_inner(self) -> sm64js_db::AuthInfo {
        self.0
    }

    pub fn get_discord_username(&self) -> Option<String> {
        if let Some(discord) = &self.0.discord {
            Some(format!(
                "{}#{}",
                discord.account.username, discord.account.discriminator
            ))
        } else {
            None
        }
    }
}
