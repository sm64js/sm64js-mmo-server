#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate maplit;

mod auth;
mod identity;

use std::collections::{HashMap, HashSet};

pub use auth::Auth;
use chrono::Duration;
pub use identity::Identity;

#[derive(Clone, Debug)]
pub struct AuthInfo(pub sm64js_db::AuthInfo);

impl AuthInfo {
    pub fn into_inner(self) -> sm64js_db::AuthInfo {
        self.0
    }

    pub fn get_account_id(&self) -> i32 {
        self.0.account.id
    }

    pub fn get_discord_username(&self) -> Option<String> {
        self.0.discord.as_ref().map(|discord| {
            format!(
                "{}#{}",
                discord.account.username, discord.account.discriminator
            )
        })
    }

    pub fn get_discord_id(&self) -> Option<String> {
        self.0
            .discord
            .as_ref()
            .map(|discord| discord.account.id.clone())
    }

    pub fn get_google_id(&self) -> Option<String> {
        self.0
            .google
            .as_ref()
            .map(|google| google.account.sub.clone())
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        if let Some(discord) = &self.0.discord {
            discord
                .account
                .roles
                .iter()
                .any(|role| permission.role_has_permission(role))
        } else {
            false
        }
    }

    pub fn is_in_game_admin(&self) -> bool {
        if let Some(discord) = &self.0.discord {
            discord
                .account
                .roles
                .iter()
                .any(|role| IN_GAME_ADMIN_ROLES.contains(role.as_str()))
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub enum Permission {
    GetAccount,
    GetAccountExt,
    GetPlayerList,
    PermBanAccount,
    PermMuteAccount,
    ReadChatLog,
    SeeIp,
    SendAnnouncement,
    TempBanAccount(Duration),
    TempMuteAccount(Duration),
}

impl PartialEq for Permission {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::GetAccount, Self::GetAccount)
                | (Self::GetAccountExt, Self::GetAccountExt)
                | (Self::GetPlayerList, Self::GetPlayerList)
                | (Self::PermBanAccount, Self::PermBanAccount)
                | (Self::PermMuteAccount, Self::PermMuteAccount)
                | (Self::ReadChatLog, Self::ReadChatLog)
                | (Self::SeeIp, Self::SeeIp)
                | (Self::SendAnnouncement, Self::SendAnnouncement)
                | (Self::TempBanAccount(_), Self::TempBanAccount(_))
                | (Self::TempMuteAccount(_), Self::TempMuteAccount(_))
        )
    }
}

impl Permission {
    fn role_has_permission(&self, role: &str) -> bool {
        ROLES_WITH_PERMISSIONS
            .get(role)
            .and_then(|permissions| permissions.iter().find(|p| *p == self))
            .map(|p| match (self, p) {
                (Self::TempBanAccount(d1), Self::TempBanAccount(d2))
                | (Self::TempMuteAccount(d1), Self::TempMuteAccount(d2)) => d1 <= d2,
                _ => true,
            })
            .unwrap_or_default()
    }
}

lazy_static! {
    pub static ref ROLES_WITH_PERMISSIONS: HashMap<&'static str, Vec<Permission>> = hashmap! {
        "755200616267120791" => // Moderator
            vec![
                Permission::GetAccount,
                Permission::GetAccountExt,
                Permission::GetPlayerList,
                Permission::PermBanAccount,
                Permission::PermMuteAccount,
                Permission::ReadChatLog,
                Permission::SeeIp,
                Permission::SendAnnouncement,
                Permission::TempBanAccount(Duration::weeks(1000)),
                Permission::TempMuteAccount(Duration::weeks(1000)),
            ],
        "780937094473318420" => // In-game Chat Moderator
            vec![
                Permission::GetAccount,
                Permission::GetPlayerList,
                Permission::PermBanAccount,
                Permission::PermMuteAccount,
                Permission::ReadChatLog,
                Permission::SendAnnouncement,
                Permission::TempBanAccount(Duration::weeks(1000)),
                Permission::TempMuteAccount(Duration::weeks(1000)),
            ],
        "801876964892868659" => // Trial mod
            vec![
                Permission::GetAccount,
                Permission::GetPlayerList,
                Permission::ReadChatLog,
                Permission::TempBanAccount(Duration::days(2)),
                Permission::TempMuteAccount(Duration::days(7)),
            ]
    };

    pub static ref IN_GAME_ADMIN_ROLES: HashSet<&'static str> = hashset! { "755200616267120791", "780937094473318420" };
}
