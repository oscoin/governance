//! Endpoints and serialisation for [`identity::Identity`] related types.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::document::{self, ToDocumentedType};
use warp::{path, Filter, Rejection, Reply};

use crate::avatar;
use crate::coco;
use crate::http;
use crate::identity;
use crate::registry;

/// Combination of all identity routes.
pub fn filters<R>(ctx: http::Ctx<R>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client + 'static,
{
    get_filter(Arc::clone(&ctx)).or(create_filter(ctx))
}

/// `POST /identities`
fn create_filter<R>(
    ctx: http::Ctx<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client + 'static,
{
    path!("identities")
        .and(warp::post())
        .and(http::with_context(ctx))
        .and(warp::body::json())
        .and(document::document(document::description(
            "Create a new unique Identity",
        )))
        .and(document::document(document::tag("Identity")))
        .and(document::document(
            document::body(CreateInput::document()).mime("application/json"),
        ))
        .and(document::document(
            document::response(
                201,
                document::body(identity::Identity::document()).mime("application/json"),
            )
            .description("Creation succeeded"),
        ))
        .and_then(handler::create)
}

/// `GET /identities/<id>`
fn get_filter<R>(ctx: http::Ctx<R>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client + 'static,
{
    path("identities")
        .and(http::with_context(ctx))
        .and(document::param::<coco::Urn>(
            "id",
            "Unique ID of the Identity",
        ))
        .and(warp::get())
        .and(document::document(document::description(
            "Find Identity by ID",
        )))
        .and(document::document(document::tag("Identity")))
        .and(document::document(
            document::response(
                200,
                document::body(identity::Identity::document()).mime("application/json"),
            )
            .description("Successful retrieval"),
        ))
        .and(document::document(
            document::response(
                404,
                document::body(super::error::Error::document()).mime("application/json"),
            )
            .description("Identity not found"),
        ))
        .and_then(handler::get)
}

/// Identity handlers for conversion between core domain and http request fullfilment.
mod handler {
    use warp::http::StatusCode;
    use warp::{reply, Rejection, Reply};

    use crate::coco;
    use crate::error;
    use crate::http;
    use crate::identity;
    use crate::registry;
    use crate::session;

    /// Create a new [`identity::Identity`].
    pub async fn create<R>(
        ctx: http::Ctx<R>,
        input: super::CreateInput,
    ) -> Result<impl Reply, Rejection>
    where
        R: registry::Client,
    {
        let ctx = ctx.read().await;

        if let Some(identity) = session::current(&ctx.peer_api, &ctx.registry, &ctx.store)
            .await?
            .identity
        {
            return Err(Rejection::from(error::Error::EntityExists(identity.urn)));
        }

        let key = ctx.keystore.get_librad_key().map_err(error::Error::from)?;
        let id = identity::create(&ctx.peer_api, key, &input.handle)?;

        session::set_identity(&ctx.store, id.clone())?;

        Ok(reply::with_status(reply::json(&id), StatusCode::CREATED))
    }

    /// Get the [`identity::Identity`] for the given `id`.
    pub async fn get<R>(ctx: http::Ctx<R>, id: coco::Urn) -> Result<impl Reply, Rejection>
    where
        R: Send + Sync,
    {
        let ctx = ctx.read().await;
        let id = identity::get(&ctx.peer_api, &id)?;
        Ok(reply::json(&id))
    }
}

impl ToDocumentedType for identity::Identity {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(6);
        properties.insert("avatarFallback".into(), avatar::Avatar::document());
        properties.insert(
            "id".into(),
            document::string()
                .description("The id of the Identity")
                .example("123abcd.git"),
        );
        properties.insert("metadata".into(), identity::Metadata::document());
        properties.insert(
            "registered".into(),
            document::string()
                .description("ID of the user on the Registry")
                .example("cloudhead")
                .nullable(true),
        );
        properties.insert(
            "shareableEntityIdentifier".into(),
            document::string()
                .description("Unique identifier that can be shared and looked up")
                .example("cloudhead@123abcd.git"),
        );
        properties.insert(
            "accountId".into(),
            document::string()
                .description("Public key of identity")
                .example("5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"),
        );

        document::DocumentedType::from(properties).description("Unique identity")
    }
}

impl ToDocumentedType for identity::Metadata {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "handle".into(),
            document::string()
                .description("User chosen nickname")
                .example("cloudhead"),
        );
        document::DocumentedType::from(properties)
            .description("User provided metadata attached to the Identity")
    }
}

#[allow(clippy::non_ascii_literal)]
impl ToDocumentedType for avatar::Avatar {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(2);
        properties.insert("background".into(), avatar::Color::document());
        properties.insert(
            "emoji".into(),
            document::string()
                .description("String containing the actual emoji codepoint to display")
                .example("🐽"),
        );

        document::DocumentedType::from(properties)
            .description("Generated avatar based on unique information")
    }
}

impl ToDocumentedType for avatar::Color {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "r".into(),
            document::string().description("Red value").example(122),
        );
        properties.insert(
            "g".into(),
            document::string().description("Green value").example(112),
        );
        properties.insert(
            "b".into(),
            document::string()
                .description("Blue value".to_string())
                .example(90),
        );

        document::DocumentedType::from(properties).description("RGB color")
    }
}

// TODO(xla): Implement Deserialize on identity::Metadata and drop this type entirely, this will
// help to avoid duplicate efforts for documentation.
/// Bundled input data for identity creation.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInput {
    /// Handle the user wants to go by.
    handle: String,
}

impl ToDocumentedType for CreateInput {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "handle".into(),
            document::string()
                .description("User chosen nickname")
                .example("cloudhead"),
        );
        document::DocumentedType::from(properties)
            .description("User provided metadata attached to the Identity")
    }
}

#[allow(clippy::non_ascii_literal, clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use radicle_registry_client::{ed25519, CryptoPair};
    use serde_json::{json, Value};
    use warp::http::StatusCode;
    use warp::test::request;

    use crate::avatar;
    use crate::error;
    use crate::http;
    use crate::identity;
    use crate::session;

    #[tokio::test]
    async fn create() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = http::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let res = request()
            .method("POST")
            .path("/identities")
            .json(&super::CreateInput {
                handle: "cloudhead".into(),
            })
            .reply(&api)
            .await;

        let ctx = ctx.read().await;
        let peer_id = ctx.peer_api.peer_id();
        let session = session::current(&ctx.peer_api, &ctx.registry, &ctx.store).await?;
        let urn = session.identity.expect("failed to set identity").urn;

        // Assert that we set the default owner and it's the same one as the session
        {
            assert_eq!(
                ctx.peer_api.default_owner(),
                Some(ctx.peer_api.get_user(&urn)?)
            );
        }

        http::test::assert_response(&res, StatusCode::CREATED, |have| {
            let avatar = avatar::Avatar::from(&urn.to_string(), avatar::Usage::Identity);
            let shareable_entity_identifier = format!("cloudhead@{}", peer_id);
            assert_eq!(
                have,
                json!({
                    "peerId": peer_id,
                    "avatarFallback": avatar,
                    "urn": urn,
                    "metadata": {
                        "handle": "cloudhead",
                    },
                    "registered": Value::Null,
                    "shareableEntityIdentifier": &shareable_entity_identifier,
                    "accountId": ed25519::Pair::from_legacy_string("//Alice", None).public()
                })
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn get() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = http::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let ctx = ctx.read().await;
        let key = ctx.keystore.get_librad_key()?;
        let user = ctx.peer_api.init_user(key, "cloudhead")?;
        let urn = user.urn();
        let handle = user.name().to_string();
        let peer_id = ctx.peer_api.peer_id();
        let shareable_entity_identifier = (peer_id.clone(), user).into();

        let res = request()
            .method("GET")
            .path(&format!("/identities/{}", urn))
            .reply(&api)
            .await;

        let have: Value = serde_json::from_slice(res.body()).unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            have,
            json!(identity::Identity {
                peer_id,
                urn: urn.clone(),
                shareable_entity_identifier,
                metadata: identity::Metadata { handle },
                registered: None,
                avatar_fallback: avatar::Avatar::from(&urn.to_string(), avatar::Usage::Identity),
                account_id: ed25519::Pair::from_legacy_string("//Alice", None).public(),
            })
        );

        Ok(())
    }
}
