#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate maplit;

mod auth;
mod identity;

use std::collections::{HashMap, HashSet};

pub use auth::Auth;
pub use identity::Identity;

#[derive(Clone, Debug)]
pub struct AuthInfo(sm64js_db::AuthInfo);

impl AuthInfo {
    pub fn into_inner(self) -> sm64js_db::AuthInfo {
        self.0
    }

    pub fn get_account_id(&self) -> i32 {
        self.0.account.id
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

    pub fn get_discord_id(&self) -> Option<String> {
        if let Some(discord) = &self.0.discord {
            Some(discord.account.id.clone())
        } else {
            None
        }
    }

    pub fn get_google_id(&self) -> Option<String> {
        if let Some(google) = &self.0.google {
            Some(google.account.sub.clone())
        } else {
            None
        }
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Permission {
    GetPlayerList,
    ReadChatLog,
    ReadChatLogWithIp,
    SendAnnouncement,
}

impl Permission {
    fn role_has_permission(&self, role: &str) -> bool {
        PERMISSION_WITH_ROLES
            .get(self)
            .map(|roles| roles.iter().find(|r| *r == &role))
            .flatten()
            .is_some()
    }
}

lazy_static! {
    pub static ref ROLES_WITH_PERMISSIONS: HashMap<&'static str, Vec<Permission>> = hashmap! {
        "755200616267120791" => // Moderator
            vec![
                Permission::GetPlayerList,
                Permission::ReadChatLog,
                Permission::ReadChatLogWithIp,
                Permission::SendAnnouncement
            ],
        "780937094473318420" => // In-game Chat Moderator
            vec![ Permission::GetPlayerList, Permission::ReadChatLog ]
    };

    pub static ref IN_GAME_ADMIN_ROLES: HashSet<&'static str> = hashset! { "755200616267120791", "780937094473318420" };

    pub static ref PERMISSION_WITH_ROLES: HashMap<Permission, Vec<&'static str>> = {
        let mut m: HashMap<Permission, Vec<&'static str>> = HashMap::new();
        ROLES_WITH_PERMISSIONS.clone().into_iter().for_each(|(role, permissions)| {
            permissions.into_iter().for_each(|permission| if let Some(p) = m.get_mut(&permission) {
                p.push(role);
            } else {
                m.insert(permission, vec![role]);
            });
        });
        m
    };
}
