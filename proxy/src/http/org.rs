//! Endpoints for Org.

use librad::paths::Paths;
use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize, Serializer};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::document::{self, ToDocumentedType};
use warp::{path, Filter, Rejection, Reply};

use crate::avatar;
use crate::http;
use crate::notification;
use crate::project;
use crate::registry;

/// Prefixed filters.
pub fn routes<R: registry::Client>(
    paths: Arc<RwLock<Paths>>,
    registry: http::Shared<R>,
    subscriptions: notification::Subscriptions,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("orgs").and(
        get_filter(Arc::clone(&registry))
            .or(get_project_filter(Arc::clone(&registry)))
            .or(get_projects_filter(paths, Arc::clone(&registry)))
            .or(register_filter(registry, subscriptions)),
    )
}

/// Combination of all org routes.
#[cfg(test)]
fn filters<R: registry::Client>(
    paths: Arc<RwLock<Paths>>,
    registry: http::Shared<R>,
    subscriptions: notification::Subscriptions,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_filter(Arc::clone(&registry))
        .or(get_project_filter(Arc::clone(&registry)))
        .or(get_projects_filter(paths, Arc::clone(&registry)))
        .or(register_filter(registry, subscriptions))
}

/// `GET /<id>`
fn get_filter<R: registry::Client>(
    registry: http::Shared<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    http::with_shared(registry)
        .and(warp::get())
        .and(document::param::<String>("id", "Unique ID of the Org"))
        .and(path::end())
        .and(document::document(document::description("Find Org by ID")))
        .and(document::document(document::tag("Org")))
        .and(document::document(
            document::response(
                200,
                document::body(registry::Org::document()).mime("application/json"),
            )
            .description("Successful retrieval"),
        ))
        .and(document::document(
            document::response(
                404,
                document::body(http::error::Error::document()).mime("application/json"),
            )
            .description("Org not found"),
        ))
        .and_then(handler::get)
}

/// `GET /<id>/projects/<project_name>`
fn get_project_filter<R: registry::Client>(
    registry: http::Shared<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    http::with_shared(registry)
        .and(warp::get())
        .and(document::param::<String>("org_id", "Unique ID of the Org"))
        .and(path("projects"))
        .and(document::param::<String>(
            "project_name",
            "Name of the project",
        ))
        .and(path::end())
        .and(document::document(document::description(
            "Find Project for Org",
        )))
        .and(document::document(document::tag("Org")))
        .and(document::document(
            document::response(
                200,
                document::body(registry::Project::document()).mime("application/json"),
            )
            .description("Successful retrieval"),
        ))
        .and(document::document(
            document::response(
                404,
                document::body(http::error::Error::document()).mime("application/json"),
            )
            .description("Project not found"),
        ))
        .and_then(handler::get_project)
}

/// `GET /<id>/projects`
fn get_projects_filter<R: registry::Client>(
    paths: Arc<RwLock<Paths>>,
    registry: http::Shared<R>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    http::with_paths(paths)
        .and(http::with_shared(registry))
        .and(warp::get())
        .and(document::param::<String>("org_id", "Unique ID of the Org"))
        .and(path("projects"))
        .and(path::end())
        .and(document::document(document::description(
            "Lists all Projects of the Org",
        )))
        .and(document::document(document::tag("Org")))
        .and(document::document(
            document::response(
                200,
                document::body(registry::Project::document()).mime("application/json"),
            )
            .description("Successful retrieval"),
        ))
        .and_then(handler::get_projects)
}

/// `POST /`
fn register_filter<R: registry::Client>(
    registry: http::Shared<R>,
    subscriptions: notification::Subscriptions,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    http::with_shared(registry)
        .and(http::with_subscriptions(subscriptions))
        .and(warp::post())
        .and(warp::body::json())
        .and(path::end())
        .and(document::document(document::description(
            "Register a new unique Org",
        )))
        .and(document::document(document::tag("Org")))
        .and(document::document(
            document::body(RegisterInput::document()).mime("application/json"),
        ))
        .and(document::document(
            document::response(
                201,
                document::body(registry::Org::document()).mime("application/json"),
            )
            .description("Creation succeeded"),
        ))
        .and_then(handler::register)
}

/// Org handlers for conversion between core domain and http request fullfilment.
mod handler {
    use librad::paths::Paths;
    use radicle_registry_client::Balance;
    use std::convert::TryFrom;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use warp::http::StatusCode;
    use warp::{reply, Rejection, Reply};

    use crate::http;
    use crate::notification;
    use crate::project;
    use crate::registry;

    /// Get the Org for the given `id`.
    pub async fn get<R: registry::Client>(
        registry: http::Shared<R>,
        org_id: String,
    ) -> Result<impl Reply, Rejection> {
        let reg = registry.read().await;
        let org_id = registry::Id::try_from(org_id)?;
        let org = reg.get_org(org_id).await?;

        Ok(reply::json(&org))
    }

    /// Get the [`registry::Project`] under the given org id.
    pub async fn get_project<R: registry::Client>(
        registry: http::Shared<R>,
        org_id: String,
        project_name: String,
    ) -> Result<impl Reply, Rejection> {
        let reg = registry.read().await;
        let org_id = registry::Id::try_from(org_id)?;
        let project_name = registry::ProjectName::try_from(project_name)?;
        let project = reg.get_project(org_id, project_name).await?;

        Ok(reply::json(&project))
    }

    /// Get all projects under the given org id.
    pub async fn get_projects<R: registry::Client>(
        paths: Arc<RwLock<Paths>>,
        registry: http::Shared<R>,
        org_id: String,
    ) -> Result<impl Reply, Rejection> {
        let reg = registry.read().await;
        let org_id = registry::Id::try_from(org_id)?;
        let projects = reg.list_org_projects(org_id).await?;
        let mut mapped_projects = Vec::new();
        for p in &projects {
            let maybe_project = if let Some(id) = &p.maybe_project_id {
                let paths = paths.read().await;
                Some(project::get(&paths, id).await.expect("Project not found"))
            } else {
                None
            };

            let org_project = super::Project {
                name: p.name.to_string(),
                org_id: p.org_id.to_string(),
                shareable_entity_identifier: format!(
                    "%{}/{}",
                    p.org_id.to_string(),
                    p.name.to_string()
                ),
                maybe_project,
            };
            mapped_projects.push(org_project);
        }

        Ok(reply::json(&mapped_projects))
    }

    /// Register an org on the Registry.
    pub async fn register<R: registry::Client>(
        registry: http::Shared<R>,
        subscriptions: notification::Subscriptions,
        input: super::RegisterInput,
    ) -> Result<impl Reply, Rejection> {
        // TODO(xla): Get keypair from persistent storage.
        let fake_pair = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        // TODO(xla): Use real fee defined by the user.
        let fake_fee: Balance = 100;

        let reg = registry.read().await;
        let org_id = registry::Id::try_from(input.id)?;
        let tx = reg.register_org(&fake_pair, org_id, fake_fee).await?;

        subscriptions
            .broadcast(notification::Notification::Transaction(tx.clone()))
            .await;

        Ok(reply::with_status(reply::json(&tx), StatusCode::CREATED))
    }
}

impl ToDocumentedType for registry::Org {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert("avatarFallback".into(), avatar::Avatar::document());
        properties.insert(
            "id".into(),
            document::string()
                .description("The id of the org")
                .example("monadic"),
        );
        properties.insert(
            "shareableEntityIdentifier".into(),
            document::string()
                .description("Unique identifier that can be shared and looked up")
                .example("%monadic"),
        );
        properties.insert(
            "members".into(),
            document::array(registry::User::document()),
        );

        document::DocumentedType::from(properties).description("Org")
    }
}

impl Serialize for registry::Project {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Project", 3)?;
        state.serialize_field("name", &self.name.to_string())?;
        state.serialize_field("orgId", &self.org_id.to_string())?;
        state.serialize_field("maybeProjectId", &self.maybe_project_id)?;

        state.end()
    }
}

impl ToDocumentedType for registry::Project {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "name".into(),
            document::string()
                .description("Name of the project")
                .example("upstream"),
        );
        properties.insert(
            "orgId".into(),
            document::string()
                .description("The id of the org")
                .example("radicle"),
        );
        properties.insert(
            "shareableEntityIdentifier".into(),
            document::string()
                .description("Unique identifier that can be shared and looked up")
                .example("%monadic/radicle-link"),
        );
        properties.insert(
            "maybeProjectId".into(),
            document::string()
                .description("The id project attested in coco")
                .example("123abdcd.git")
                .nullable(true),
        );

        document::DocumentedType::from(properties).description("Project")
    }
}

/// Object the API returns for a project that is registered under an org.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Id of the Org.
    org_id: String,
    /// Unambiguous identifier pointing at this identity.
    shareable_entity_identifier: String,
    /// Name of the project.
    name: String,
    /// Associated CoCo project.
    maybe_project: Option<project::Project>,
}

/// Bundled input data for org registration.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterInput {
    /// Id of the Org.
    id: String,
}

impl ToDocumentedType for RegisterInput {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(1);
        properties.insert(
            "id".into(),
            document::string()
                .description("ID of the org")
                .example("monadic"),
        );

        document::DocumentedType::from(properties).description("Input for org registration")
    }
}

#[allow(
    clippy::option_unwrap_used,
    clippy::result_unwrap_used,
    clippy::indexing_slicing
)]
#[cfg(test)]
mod test {
    use librad::paths::Paths;
    use pretty_assertions::assert_eq;
    use serde_json::{json, Value};
    use std::convert::TryFrom;
    use std::str::FromStr;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use warp::http::StatusCode;
    use warp::test::request;

    use radicle_registry_client as protocol;

    use crate::avatar;
    use crate::coco;
    use crate::error;
    use crate::notification;
    use crate::registry::{self, Cache as _, Client as _};

    #[tokio::test]
    async fn get() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let librad_paths = Paths::from_root(tmp_dir.path())?;
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let subscriptions = notification::Subscriptions::default();
        let api = super::filters(
            Arc::new(RwLock::new(librad_paths.clone())),
            Arc::clone(&registry),
            subscriptions,
        );
        let author = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;
        let org_id = registry::Id::try_from("radicle")?;

        // Register the user
        registry
            .write()
            .await
            .register_user(&author, handle.clone(), None, 10)
            .await?;

        let user = registry.read().await.get_user(handle).await?.unwrap();

        // Register the org
        let fee: radicle_registry_client::Balance = 100;
        registry
            .write()
            .await
            .register_org(&author, org_id.clone(), fee)
            .await?;

        let res = request()
            .method("GET")
            .path(&format!("/{}", org_id.to_string()))
            .reply(&api)
            .await;

        let have: Value = serde_json::from_slice(res.body()).unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            have,
            json!(registry::Org {
                id: org_id.clone(),
                shareable_entity_identifier: format!("%{}", org_id.to_string()),
                avatar_fallback: avatar::Avatar::from(&org_id.to_string(), avatar::Usage::Org),
                members: vec![user]
            })
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_project() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let librad_paths = Paths::from_root(tmp_dir.path())?;
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let subscriptions = notification::Subscriptions::default();
        let api = super::filters(
            Arc::new(RwLock::new(librad_paths.clone())),
            Arc::clone(&registry),
            subscriptions,
        );
        let author = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;
        let org_id = registry::Id::try_from("radicle")?;
        let project_name = registry::ProjectName::try_from("upstream")?;

        // Register the user
        registry
            .write()
            .await
            .register_user(&author, handle, None, 10)
            .await?;

        // Register the org.
        registry
            .write()
            .await
            .register_org(&author, org_id.clone(), 10)
            .await?;

        // Register the project.
        registry
            .write()
            .await
            .register_project(&author, org_id.clone(), project_name.clone(), None, 10)
            .await?;

        let res = request()
            .method("GET")
            .path(&format!("/{}/projects/{}", org_id, project_name))
            .reply(&api)
            .await;

        let have: Value = serde_json::from_slice(res.body()).unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            have,
            json!(registry::Project {
                name: project_name,
                org_id: org_id,
                maybe_project_id: None,
            })
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_projects() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let librad_paths = Paths::from_root(tmp_dir.path())?;
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            Arc::new(RwLock::new(registry::Registry::new(client)))
        };
        let subscriptions = notification::Subscriptions::default();
        let api = super::filters(
            Arc::new(RwLock::new(librad_paths.clone())),
            Arc::clone(&registry),
            subscriptions,
        );

        let repo_dir = tempfile::tempdir_in(tmp_dir.path())?;
        let path = repo_dir.path().to_str().unwrap().to_string();
        coco::init_repo(path.clone())?;

        let project_name = "upstream";
        let project_description = "desktop client for radicle";
        let default_branch = "master";

        let (project_id, _meta) = coco::init_project(
            &librad_paths,
            &path,
            project_name,
            project_description,
            default_branch,
        )?;

        // Register the user
        let author = radicle_registry_client::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;
        let org_id = registry::Id::try_from("radicle")?;
        let project_name = registry::ProjectName::try_from(project_name)?;

        registry
            .write()
            .await
            .register_user(&author, handle, None, 10)
            .await?;

        // Register the org.
        registry
            .write()
            .await
            .register_org(&author, org_id.clone(), 10)
            .await?;

        // Register the project.
        registry
            .write()
            .await
            .register_project(
                &author,
                org_id.clone(),
                project_name.clone(),
                Some(
                    librad::project::ProjectId::from_str(&project_id.to_string())
                        .expect("Project id"),
                ),
                10,
            )
            .await?;

        let res = request()
            .method("GET")
            .path(&format!("/{}/projects", org_id.to_string()))
            .reply(&api)
            .await;

        let have: Value = serde_json::from_slice(res.body()).unwrap();

        let want = json!([{
            "name": project_name.to_string(),
            "orgId": org_id.to_string(),
            "shareableEntityIdentifier": format!("%{}/{}", org_id.to_string(), project_name.to_string()),
            "maybeProject": {
                "id": project_id.to_string(),
                "metadata": {
                    "defaultBranch": default_branch.to_string(),
                    "description": project_description.to_string(),
                    "name": project_name.to_string(),
                },
                "registration": Value::Null,
                "shareableEntityIdentifier": format!("%{}", project_id.to_string()),
                "stats": {
                    "branches": 11,
                    "commits": 267,
                    "contributors": 8,
                },
            }
        }]);

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(have, want);

        Ok(())
    }

    #[tokio::test]
    async fn register() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let librad_paths = Paths::from_root(tmp_dir.path())?;
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let cache = Arc::new(RwLock::new(registry::Cacher::new(registry, &store)));
        let subscriptions = notification::Subscriptions::default();

        let api = super::filters(
            Arc::new(RwLock::new(librad_paths.clone())),
            Arc::clone(&cache),
            subscriptions,
        );
        let author = protocol::ed25519::Pair::from_legacy_string("//Alice", None);
        let handle = registry::Id::try_from("alice")?;
        let org_id = registry::Id::try_from("radicle")?;

        // Register the user
        cache
            .write()
            .await
            .register_user(&author, handle, None, 10)
            .await?;

        let res = request()
            .method("POST")
            .path("/")
            .json(&super::RegisterInput {
                id: org_id.to_string(),
            })
            .reply(&api)
            .await;

        let txs = cache.write().await.list_transactions(vec![])?;

        // Get the registered org
        let org = cache.read().await.get_org(org_id.clone()).await?.unwrap();

        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(txs.len(), 2);
        assert_eq!(org.id, org_id);

        Ok(())
    }
}
