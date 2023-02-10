use anyhow::Error;
use chrono::{Duration, Utc};
use clap::Parser;
use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use session::*;
use uuid::Uuid;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None, trailing_var_arg=true)]
struct Args {
    #[clap(short, long, env = "ACCOUNT_ID")]
    account_id: Uuid,
    #[clap(short, long, env = "ROLE_ID")]
    role_id: Uuid,
    #[clap(long = "private", env = "SESSION_JWT_PRIVATE_CERTIFICATE")]
    session_jwt_private_certificate: String,
    #[clap(long = "public", env = "SESSION_JWT_PUBLIC_CERTIFICATE")]
    session_jwt_public_certificate: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Fields {
    role_ids: Vec<Uuid>,
}

fn main() -> Result<(), Error> {
    let Args {
        account_id,
        role_id,
        session_jwt_private_certificate,
        session_jwt_public_certificate,
    } = Args::parse();
    let session_jwt_private_certificate: EncodingKey = parse_encoding_key(session_jwt_private_certificate); // expects RSA-PKCS1.5 PEM format
    let session_jwt_public_certificate: DecodingKey = parse_decoding_key(session_jwt_public_certificate); // expects RSA-PKCS1.5 PEM format
    let claims = AccountSessionClaims::new(
        AccountSessionState {
            account_id,
            fields: Fields {
                role_ids: vec![role_id],
            },
        },
        ACCOUNTS_ISSUER,
        Utc::now().naive_utc() + Duration::weeks(5200),
    );

    let encoded = claims.encode(&ACCOUNT_SESSION_JWT_HEADER, &session_jwt_private_certificate)?;

    // ensure that session can be decoded
    <Session<AccountSessionToken<()>> as RawSession<AccountSession<Uuid, Fields>>>::try_decode(
        Session {
            session_id: Uuid::new_v4(),
            created_at: Utc::now().naive_utc(),
            value: AccountSessionToken {
                token: encoded.token.clone(),
                claims: (),
            },
            max_age: None,
            expires: None,
        },
        &session_jwt_public_certificate,
        &ACCOUNT_SESSION_JWT_VALIDATION,
    )?;

    println!("{}", encoded.token);

    Ok(())
}
