table! {
    accounts (id) {
        id -> Int4,
        username -> Nullable<Varchar>,
        discord_username -> Nullable<Varchar>,
        discord_discriminator -> Nullable<Varchar>,
        google_sub -> Nullable<Varchar>,
    }
}

table! {
    discord_accounts (username, discriminator) {
        username -> Varchar,
        discriminator -> Varchar,
        avatar -> Nullable<Varchar>,
        mfa_enabled -> Nullable<Bool>,
        locale -> Nullable<Varchar>,
        flags -> Nullable<Int4>,
        premium_type -> Nullable<Int2>,
        public_flags -> Nullable<Int4>,
        session -> Nullable<Int4>,
    }
}

table! {
    discord_sessions (id) {
        id -> Int4,
        access_token -> Varchar,
        token_type -> Varchar,
        expires_at -> Timestamp,
    }
}

table! {
    google_accounts (sub) {
        sub -> Varchar,
        session -> Nullable<Int4>,
    }
}

table! {
    google_sessions (id) {
        id -> Int4,
        id_token -> Varchar,
        expires_at -> Timestamp,
    }
}

joinable!(accounts -> google_accounts (google_sub));
joinable!(discord_accounts -> discord_sessions (session));
joinable!(google_accounts -> google_sessions (session));

allow_tables_to_appear_in_same_query!(
    accounts,
    discord_accounts,
    discord_sessions,
    google_accounts,
    google_sessions,
);
