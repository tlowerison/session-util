# session-util

Utilities for managing user sessions in distributed key value stores.

Provides integrations with:
- [tower](https://github.com/tokio-rs/tower), for extracting and verifying sessions from http request cookies
- [axum](https://github.com/tokio-rs/axum), for extracting verified sessions from http request extensions in request handlers

## Example
Note that this example does not demonstrate creating sessions.
```rs
use axum::Router;
use jsonwebtoken::{Algorithm, Validation};
use http::StatusCode;
use hyper::{Body, Response};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use session_util::{AccountSessionClaims, CookieConfig, DynAccountSessionStore, SessionLayer};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceBuilder;
use uuid::Uuid;

type AccountId = Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountSessionFields {
    pub role_ids: Vec<Uuid>,
}

// useful to alias the constructed AccountSession type in your application
// to avoid needing to plug in these fields everywhere
type AccountSession = session_util::AccountSession<AccountId, AccountSessionFields>;


pub const ACCOUNT_SESSION_JWT_ALGORITHM: jsonwebtoken::Algorithm = jsonwebtoken::Algorithm::RS512;
lazy_static! {
    pub static ref ACCOUNT_SESSION_JWT_HEADER: jsonwebtoken::Header = jsonwebtoken::Header::new(ACCOUNT_SESSION_JWT_ALGORITHM);
    pub static ref ACCOUNT_SESSION_JWT_VALIDATION: jsonwebtoken::Validation = {
        let mut validation = jsonwebtoken::Validation::new(ACCOUNT_SESSION_JWT_ALGORITHM);
        validation.set_issuer(&[ACCOUNTS_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation
    };
    pub static ref ACCOUNT_SESSION_DECODING_KEY: jsonwebtoken::DecodingKey = {
        let jwt_public_certificate = std::env::var().expect("expected an environment variable JWT_PUBLIC_CERTIFICATE to exist");
        session_util::parse_decoding_key(jwt_public_certificate).expect("unable to parse JWT_PUBLIC_CERTIFICATE as a valid public key")
    };
    pub static ref ACCOUNT_SESSION_ENCODING_KEY: jsonwebtoken::EncodingKey = {
        let jwt_private_certificate = std::env::var().expect("expected an environment variable JWT_PRIVATE_CERTIFICATE to exist");
        session_util::parse_encoding_key(jwt_private_certificate).expect("unable to parse JWT_PRIVATE_CERTIFICATE as a valid private key")
    };
}

#[tokio::main]
async fn main() {
    let account_session_store = account_session_store();

    let middleware = ServiceBuilder::new()
        // additional layers
        .layer(SessionLayer::<AccountSession, _, _, _>::encoded(
            account_session_store.clone(),
            std::env::var("SESSION_JWT_PUBLIC_CERTIFICATE")?,
            &ACCOUNT_SESSION_JWT_VALIDATION,
        ))
        // additional layers
        .into_inner();

    let router = Router::new()
        .post("/sign-in", sign_in)
        .get("/my-account-id", my_account_id);

    let app = router
        .layer(Extension(account_session_store))
        .layer(middleware);

    axum::Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .serve(app.into_make_service())
        .with_graceful_shutdown(service_util::shutdown_signal())
        .await?;

    Ok(())
}

pub async fn account_session_store() -> Result<DynAccountSessionStore, anyhow::Error> {
    redis_store_standalone(
        RedisStoreConfig {
            key_name: "session_id",
            key: std::env::var("SESSION_SECRET")?,
            username: std::env::var("REDIS_USERNAME").ok(),
            password: std::env::var("REDIS_PASSWORD").ok(),
        },
        RedisStoreNodeConfig {
            db: std::env::var("REDIS_DB").ok().map(|x| str::parse(&x)).transpose()?,
            host: std::env::var("REDIS_HOST")?,
            port: std::env::var("REDIS_PORT").ok().map(|x| str::parse(&x)).transpose()?,
        },
    )
    .await
}

#[derive(Deserialize)]
pub struct SignInPost {
    pub email: String,
    pub password: String,
}

async fn sign_in(
    Extension(account_session_store): Extension<DynAccountSessionStore>,
    raw_body: RawBody,
) -> Result<Response<Body>, StatusCode> {
    let sign_in_post: SignInPost = hyper::body::to_bytes(body)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
        .and_then(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|_| StatusCode::BAD_REQUEST)
        })?;


    // check email / password

    let token = AccountSessionClaims::new_exp_in(
        AccountSessionState {
            account_id: db_account.id,
            fields: AccountSessionFields { role_ids: vec![] },
        },
        "my-service-name",
        chrono::Duration::hours(12),
    )
    .encode(
        &ACCOUNT_SESSION_JWT_HEADER,
        &ACCOUNT_SESSION_ENCODING_KEY,
    )?;

    let mut response = Response::new(Body::empty());
    account_session_store
        .store_session_and_set_cookie(
            &mut response,
            CookieConfig::new(&token)
              .domain("example.org")
              .secure(false)
              .max_age(chrono::Duration::hours(12)),
            Some(format!("{}", db_account.id)),
        )
        .await?;

    Ok(response)
}

async fn my_account_id(Extension(session): Extension<Option<AccountSession>>) -> Result<Uuid, StatusCode> {
    let session = session.ok_or(StatusCode::BAD_REQUEST)?;
    Ok(*session.account_id())
}
```
