# session-util

Utilities for managing user sessions in distributed key value stores.

Provides integrations with:
- [tower](https://github.com/tokio-rs/tower), for extracting and verifying sessions from http request cookies
- [axum](https://github.com/tokio-rs/axum), for extracting verified sessions from http request extensions in request handlers

## Example
Note that this example would require the `account-session` and `redis-backend` features to be enabled.
```rs
use axum::Router;
use jsonwebtoken as jwt;
use http::StatusCode;
use hyper::{Body, Response};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceBuilder;
use uuid::Uuid;

type AccountId = Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountSessionFields {
    pub role_ids: Vec<Uuid>,
}

// useful to alias the constructed AccountSession type in your application
// to avoid needing to plug in these generics everywhere
type AccountSession = session_util::AccountSession<AccountId, AccountSessionFields>;


pub const ACCOUNT_SESSION_JWT_ALGORITHM: jwt::Algorithm = jwt::Algorithm::RS512;
lazy_static! {
    pub static ref ACCOUNT_SESSION_DECODING_KEY: jwt::DecodingKey = {
        let jwt_public_certificate = std::env::var().expect("expected an environment variable JWT_PUBLIC_CERTIFICATE to exist");
        session_util::parse_decoding_key(jwt_public_certificate).expect("unable to parse JWT_PUBLIC_CERTIFICATE as a valid public key")
    };
    pub static ref ACCOUNT_SESSION_ENCODING_KEY: jwt::EncodingKey = {
        let jwt_private_certificate = std::env::var().expect("expected an environment variable JWT_PRIVATE_CERTIFICATE to exist");
        session_util::parse_encoding_key(jwt_private_certificate).expect("unable to parse JWT_PRIVATE_CERTIFICATE as a valid private key")
    };
    pub static ref ACCOUNT_SESSION_JWT_VALIDATION: jwt::Validation = {
        let mut validation = jwt::Validation::new(ACCOUNT_SESSION_JWT_ALGORITHM);
        validation.set_issuer(&[ACCOUNTS_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation
    };
}

#[tokio::main]
async fn main() {
    let account_session_store = account_session_store();

    let middleware = ServiceBuilder::new()
        // additional layers
        //
        // this layer will attempt to extract an account session from an inbound
        // http request by deserializing its cookies, verifying their signature,
        // retrieving the corresponding session data from a distributed key-value
        // store (Redis in this example), and inserting the session data as an
        // extension on the http request
        .layer(session_util::SessionLayer::<AccountSession, _, _, _>::encoded(
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

// creates a redis session store with decoded tokens of type
// `session_util::AccountSession<Uuid, AccountSessionFields>` which is equivalent to
// `session_util::Session<session_util::AccountSessionToken<session_util::AccountSessionClaims<Uuid, AccountSessionFields>>>`
pub async fn account_session_store() -> Result<session_util::DynAccountSessionStore, anyhow::Error> {
    session_util::redis_store_standalone(
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

// test endpoint for creating sessions when a user signs in
async fn sign_in(
    Extension(account_session_store): Extension<session_util::DynAccountSessionStore>,
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

    let token = session_util::AccountSessionClaims::new_exp_in(
        AccountSessionState {
            account_id: db_account.id,
            fields: AccountSessionFields {
                role_ids: vec![],
            },
        },
        "my-service-name",
        chrono::Duration::hours(12),
    )
    .encode(
        &jwt::Header::new(ACCOUNT_SESSION_JWT_ALGORITHM),
        &ACCOUNT_SESSION_ENCODING_KEY,
    )?;

    let mut response = Response::new(Body::empty());
    account_session_store
        .store_session_and_set_cookie(
            &mut response,
            session_util::CookieConfig::new(&token)
              .domain("example.org")
              .secure(false)
              .max_age(chrono::Duration::hours(12)),
            Some(format!("{}", db_account.id)),
        )
        .await?;

    Ok(response)
}

// test endpoint for extracting sessions from requests if one is supplied and using them
// note that `Extension<Option<AccountSession>>` is used and not `Extension<AccountSession>`
// if no session is found for this request and we tried to extract the latter type, Axum will return
// a typing error for us because it would be unable to retrieve all the arguments required to satisfy
// this function's signature
// using `Option<AccountSession>` allows us to return whatever response we choose if no session is found
async fn my_account_id(Extension(session): Extension<Option<AccountSession>>) -> Result<Uuid, StatusCode> {
    let session = session.ok_or(StatusCode::BAD_REQUEST)?;
    Ok(*session.account_id())
}
```
