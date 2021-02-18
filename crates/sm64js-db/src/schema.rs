table! {
    accounts (id) {
        id -> Int4,
        username -> Nullable<Varchar>,
        discord_id -> Nullable<Int4>,
        google_id -> Nullable<Int4>,
    }
}

table! {
    discord_accounts (id) {
        id -> Int4,
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
        expires_in -> Int8,
    }
}

table! {
    google_accounts (id) {
        id -> Int4,
        session -> Nullable<Int4>,
    }
}

table! {
    google_sessions (id) {
        id -> Int4,
        id_token -> Varchar,
        expires_in -> Int8,
    }
}

allow_tables_to_appear_in_same_query!(
    accounts,
    discord_accounts,
    discord_sessions,
    google_accounts,
    google_sessions,
);
