//! Endpoints for registry accounts.

use std::sync::Arc;
use warp::document::{self, ToDocumentedType};
use warp::filters::BoxedFilter;
use warp::{path, Filter, Rejection, Reply};

use crate::http;
use crate::registry;

/// Prefixed filters.
pub fn filters<R>(registry: &http::Shared<R>) -> BoxedFilter<(impl Reply,)>
where
    R: registry::Client + 'static,
{
    exists_filter(Arc::clone(registry))
        .or(get_balance_filter(Arc::clone(registry)))
        .boxed()
}

/// `GET /<id>/exists`
fn exists_filter<R: registry::Client>(
    registry: http::Shared<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client,
{
    http::with_shared(registry)
        .and(warp::get())
        .and(document::param::<registry::AccountId>("id", "Account id"))
        .and(path("exists"))
        .and(path::end())
        .and(document::document(document::tag("Account")))
        .and(document::document(document::description(
            "Check whether a given account exists on chain",
        )))
        .and_then(handler::exists)
}

/// `GET /<id>/balance`
fn get_balance_filter<R: registry::Client>(
    registry: http::Shared<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    http::with_shared(registry)
        .and(warp::get())
        .and(document::param::<registry::AccountId>("id", "Account id"))
        .and(path("balance"))
        .and(path::end())
        .and(document::document(document::tag("Account")))
        .and(document::document(document::description(
            "Fetch the balance of the account from the Registry",
        )))
        .and(document::document(
            document::response(
                200,
                document::body(registry::Balance::document()).mime("application/json"),
            )
            .description("Successful retrieval"),
        ))
        .and_then(handler::get_balance)
}

/// Account handlers for conversion between core domain and http request fullfilment.
mod handler {
    use warp::{reply, Rejection, Reply};

    use crate::error;
    use crate::http;
    use crate::registry;

    /// Check whether the given account exists on chain
    pub async fn exists<R: registry::Client>(
        registry: http::Shared<R>,
        account_id: registry::AccountId,
    ) -> Result<impl Reply, Rejection> {
        let reg = registry.read().await;
        let exists = reg.account_exists(&account_id).await?;

        Ok(reply::json(&exists))
    }

    /// Get the [`registry::Balance`] of a given account.
    pub async fn get_balance<R: registry::Client>(
        registry: http::Shared<R>,
        account_id: registry::AccountId,
    ) -> Result<impl Reply, Rejection> {
        let reg = registry.read().await;
        match reg.free_balance(&account_id).await {
            Ok(balance) => Ok(reply::json(&balance)),
            Err(error::Error::AccountNotFound(_)) => Err(warp::reject::not_found()),
            Err(other_error) => Err(Rejection::from(other_error)),
        }
    }
}

#[allow(clippy::unwrap_used, clippy::all, clippy::panic)]
#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::convert::TryFrom;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use warp::http::StatusCode;
    use warp::test::request;

    use radicle_registry_client::CryptoPair;

    use crate::error;
    use crate::http;
    use crate::registry::{self, Client as _};

    #[tokio::test]
    async fn account_exists() -> Result<(), error::Error> {
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let api = super::filters(&Arc::clone(&registry));
        let author = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;

        // Register the user
        registry
            .write()
            .await
            .register_user(&author, handle.clone(), None, 10)
            .await?;
        let user = registry.read().await.get_user(handle).await?.unwrap();

        let res = request()
            .method("GET")
            .path(&format!("/{}/exists", user.account_id.to_string()))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(true), "Account was expected to exist on chain");
        });

        Ok(())
    }

    #[tokio::test]
    async fn account_does_not_exists() -> Result<(), error::Error> {
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let api = super::filters(&Arc::clone(&registry));
        let author =
            radicle_registry_client::ed25519::Pair::from_legacy_string("//Cloudhead", None);
        let res = request()
            .method("GET")
            .path(&format!("/{}/exists", author.public()))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(
                have,
                json!(false),
                "Account was not expected to exist on chain"
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn existing_account_balance() -> Result<(), error::Error> {
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let api = super::filters(&Arc::clone(&registry));
        let author = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;

        // Register the user
        registry
            .write()
            .await
            .register_user(&author, handle.clone(), None, 10)
            .await?;
        let user = registry.read().await.get_user(handle).await?.unwrap();

        let res = request()
            .method("GET")
            .path(&format!("/{}/balance", user.account_id.to_string()))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(
                have.to_string(),
                "1152921504606846965",
                "Account doesn't have the expected amount"
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn unexisting_account_balance() -> Result<(), error::Error> {
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let api = super::filters(&Arc::clone(&registry));
        let unkown_account =
            radicle_registry_client::ed25519::Pair::from_legacy_string("//Cloudhead", None)
                .public();

        let res = request()
            .method("GET")
            .path(&format!("/{}/balance", unkown_account.to_string()))
            .reply(&api)
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}
