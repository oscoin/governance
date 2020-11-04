//! Endpoints for handling the keystore.

use data_encoding::HEXLOWER;
use rand::Rng;
use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, path, Filter, Rejection, Reply};

use crate::{context, http};

/// Combination of all keystore filters.
pub fn filters(ctx: context::Context) -> BoxedFilter<(impl Reply,)> {
    unseal_filter(ctx.clone()).or(create_filter(ctx)).boxed()
}

/// `POST /unseal`
fn unseal_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("unseal")
        .and(warp::post())
        .and(path::end())
        .and(http::with_context(ctx))
        .and(warp::body::json())
        .and_then(handler::unseal)
}

/// `POST /`
fn create_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(path::end())
        .and(http::with_context(ctx))
        .and(warp::body::json())
        .and_then(handler::create)
}

/// Keystore handlers for conversion between core domain and HTTP request fulfilment.
mod handler {
    use warp::{http::StatusCode, reply, Rejection, Reply};

    use crate::context;

    /// Unseal the keystore.
    pub async fn unseal(
        mut ctx: context::Context,
        input: super::UnsealInput,
    ) -> Result<impl Reply, Rejection> {
        ctx.unseal_keystore(input.passphrase).await?;

        let auth_token_lock = ctx.auth_token();
        let mut auth_token = auth_token_lock.write().await;
        let token = super::gen_token();
        *auth_token = Some(token.clone());
        Ok(warp::reply::with_header(
            reply::with_status(reply(), StatusCode::NO_CONTENT),
            "Set-Cookie",
            super::format_cookie_header(&token),
        )
        .into_response())
    }

    /// Initialize the keystore with a new key.
    pub async fn create(
        mut ctx: context::Context,
        input: super::CreateInput,
    ) -> Result<impl Reply, Rejection> {
        ctx.create_key(input.passphrase).await?;

        let auth_token_lock = ctx.auth_token();
        let mut auth_token = auth_token_lock.write().await;
        let token = super::gen_token();
        *auth_token = Some(token.clone());
        Ok(warp::reply::with_header(
            reply::with_status(reply(), StatusCode::NO_CONTENT),
            "Set-Cookie",
            super::format_cookie_header(&token),
        )
        .into_response())
    }
}

/// Bundled input data for unseal request.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsealInput {
    /// Passphrase to unlock the keystore.
    passphrase: coco::keystore::SecUtf8,
}

/// Bundled input data for `create` request.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInput {
    /// Passphrase to encrypt the keystore with.
    passphrase: coco::keystore::SecUtf8,
}

/// Generates a random auth token.
fn gen_token() -> String {
    let randoms = rand::thread_rng().gen::<[u8; 32]>();
    HEXLOWER.encode(&randoms)
}

/// Format the cookie header attributes.
fn format_cookie_header(token: &str) -> String {
    format!("auth-token={}; Path=/", token)
}
