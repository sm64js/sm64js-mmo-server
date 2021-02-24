table! {
    accounts (id) {
        id -> Int4,
        username -> Nullable<Varchar>,
    }
}

table! {
    discord_accounts (id) {
        id -> Varchar,
        username -> Varchar,
        discriminator -> Varchar,
        avatar -> Nullable<Varchar>,
        mfa_enabled -> Nullable<Bool>,
        locale -> Nullable<Varchar>,
        flags -> Nullable<Int4>,
        premium_type -> Nullable<Int2>,
        public_flags -> Nullable<Int4>,
        account_id -> Int4,
    }
}

table! {
    discord_sessions (id) {
        id -> Int4,
        access_token -> Varchar,
        token_type -> Varchar,
        expires_at -> Timestamp,
        discord_account_id -> Varchar,
    }
}

table! {
    google_accounts (sub) {
        sub -> Varchar,
        account_id -> Int4,
    }
}

table! {
    google_sessions (id) {
        id -> Int4,
        id_token -> Varchar,
        expires_at -> Timestamp,
        google_account_id -> Varchar,
    }
}

joinable!(discord_accounts -> accounts (account_id));
joinable!(discord_sessions -> discord_accounts (discord_account_id));
joinable!(google_accounts -> accounts (account_id));
joinable!(google_sessions -> google_accounts (google_account_id));

allow_tables_to_appear_in_same_query!(
    accounts,
    discord_accounts,
    discord_sessions,
    google_accounts,
    google_sessions,
);
