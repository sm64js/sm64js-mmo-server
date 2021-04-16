table! {
    accounts (id) {
        id -> Int4,
        username -> Nullable<Varchar>,
        last_ip -> Varchar,
    }
}

table! {
    bans (id) {
        id -> Int4,
        ip -> Varchar,
        reason -> Nullable<Varchar>,
        expires_at -> Nullable<Timestamp>,
        account_id -> Nullable<Int4>,
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
        nick -> Nullable<Varchar>,
        roles -> Array<Text>,
        joined_at -> Varchar,
        premium_since -> Nullable<Varchar>,
        deaf -> Bool,
        mute -> Bool,
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
    geolocations (id) {
        id -> Int4,
        query -> Varchar,
        country_code -> Varchar,
        region -> Varchar,
        city -> Varchar,
        zip -> Varchar,
        lat -> Float8,
        lon -> Float8,
        timezone -> Varchar,
        isp -> Varchar,
        mobile -> Bool,
        proxy -> Bool,
        discord_session_id -> Nullable<Int4>,
        google_session_id -> Nullable<Int4>,
        ban_id -> Nullable<Int4>,
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

table! {
    ip_bans (ip) {
        ip -> Varchar,
        reason -> Nullable<Varchar>,
        expires_at -> Nullable<Timestamp>,
    }
}

table! {
    mutes (id) {
        id -> Int4,
        reason -> Nullable<Varchar>,
        expires_at -> Nullable<Timestamp>,
        account_id -> Int4,
    }
}

joinable!(bans -> accounts (account_id));
joinable!(discord_accounts -> accounts (account_id));
joinable!(discord_sessions -> discord_accounts (discord_account_id));
joinable!(geolocations -> bans (ban_id));
joinable!(geolocations -> discord_sessions (discord_session_id));
joinable!(geolocations -> google_sessions (google_session_id));
joinable!(google_accounts -> accounts (account_id));
joinable!(google_sessions -> google_accounts (google_account_id));
joinable!(mutes -> accounts (account_id));

allow_tables_to_appear_in_same_query!(
    accounts,
    bans,
    discord_accounts,
    discord_sessions,
    geolocations,
    google_accounts,
    google_sessions,
    ip_bans,
    mutes,
);
