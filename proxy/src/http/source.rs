//! Endpoints and serialisation for source code browsing.

use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize, Serializer};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use warp::document::{self, ToDocumentedType};
use warp::{path, Filter, Rejection, Reply};

use librad::peer;

use crate::coco;
use crate::http;
use crate::identity;
use crate::registry;

/// Prefixed filters.
pub fn routes<R>(
    peer: Arc<Mutex<coco::PeerApi>>,
    registry: http::Shared<R>,
    store: Arc<RwLock<kv::Store>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client,
{
    path("source").and(
        blob_filter(Arc::clone(&peer), registry, store)
            .or(branches_filter(Arc::clone(&peer)))
            .or(commit_filter(Arc::clone(&peer)))
            .or(commits_filter(Arc::clone(&peer)))
            .or(local_state_filter())
            .or(revisions_filter(Arc::clone(&peer)))
            .or(tags_filter(Arc::clone(&peer)))
            .or(tree_filter(peer)),
    )
}

/// Combination of all source filters.
#[cfg(test)]
fn filters<R>(
    peer: Arc<Mutex<coco::PeerApi>>,
    registry: http::Shared<R>,
    store: Arc<RwLock<kv::Store>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client,
{
    blob_filter(Arc::clone(&peer), registry, store)
        .or(branches_filter(Arc::clone(&peer)))
        .or(commit_filter(Arc::clone(&peer)))
        .or(commits_filter(Arc::clone(&peer)))
        .or(local_state_filter())
        .or(revisions_filter(Arc::clone(&peer)))
        .or(tags_filter(Arc::clone(&peer)))
        .or(tree_filter(peer))
}

/// `GET /blob/<project_id>?revision=<revision>&path=<path>`
fn blob_filter<R>(
    peer: Arc<Mutex<coco::PeerApi>>,
    registry: http::Shared<R>,
    store: Arc<RwLock<kv::Store>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    R: registry::Client,
{
    path("blob")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(http::with_shared(registry))
        .and(http::with_store(store))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(warp::filters::query::query::<BlobQuery>())
        .and(document::document(
            document::query("revision", document::string()).description("Git revision"),
        ))
        .and(document::document(
            document::query("path", document::string())
                .description("Location of the file in the repo tree"),
        ))
        .and(document::document(document::description("Fetch a Blob")))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(coco::Blob::document()).mime("application/json"),
            )
            .description("Blob for path found"),
        ))
        .and_then(handler::blob)
}

/// `GET /branches/<project_id>`
fn branches_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("branches")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(document::document(document::description("List Branches")))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(
                    document::array(coco::Branch::document()).description("List of branches"),
                )
                .mime("application/json"),
            )
            .description("List of branches"),
        ))
        .and_then(handler::branches)
}

/// `GET /commit/<project_id>/<sha1>`
fn commit_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("commit")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(document::param::<String>("sha1", "Git object id"))
        .and(document::document(document::description("Fetch a Commit")))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(coco::Commit::document()).mime("application/json"),
            )
            .description("Commit for SHA1 found"),
        ))
        .and_then(handler::commit)
}

/// `GET /commits/<project_id>?branch=<branch>`
fn commits_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("commits")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(warp::filters::query::query::<CommitsQuery>())
        .and(document::document(
            document::query("branch", document::string()).description("Git branch"),
        ))
        .and(document::document(document::description(
            "Fetch Commits from a Branch",
        )))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(document::array(coco::Commit::document())).mime("application/json"),
            )
            .description("Branch found"),
        ))
        .and_then(handler::commits)
}

/// `GET /branches/<project_id>`
fn local_state_filter() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("local-state")
        .and(warp::get())
        .and(document::tail(
            "path",
            "Location of the repository on the filesystem",
        ))
        .and(document::document(document::description(
            "List Branches, Remotes and if it is managed by coco for a local Repository",
        )))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(
                    document::array(coco::Branch::document()).description("List of branches"),
                )
                .mime("application/json"),
            )
            .description("List of branches"),
        ))
        .and_then(handler::local_state)
}

/// `GET /revisions/<project_id>`
fn revisions_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("revisions")
        .and(warp::get())
        .and(super::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(document::document(document::description(
            "List both branches and tags",
        )))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(
                    document::array(Revision::document()).description("List of revisions per repo"),
                )
                .mime("application/json"),
            )
            .description("List of branches and tags"),
        ))
        .and_then(handler::revisions)
}

/// `GET /tags/<project_id>`
fn tags_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("tags")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(document::document(document::description("List Tags")))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(document::array(coco::Tag::document()).description("List of tags"))
                    .mime("application/json"),
            )
            .description("List of tags"),
        ))
        .and_then(handler::tags)
}

/// `GET /tree/<project_id>/<revision>/<prefix>`
fn tree_filter(
    peer: Arc<Mutex<coco::PeerApi>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("tree")
        .and(warp::get())
        .and(http::with_peer(peer))
        .and(document::param::<String>(
            "project_id",
            "ID of the project the blob is part of",
        ))
        .and(warp::filters::query::query::<TreeQuery>())
        .and(document::document(
            document::query("revision", document::string()).description("Git revision"),
        ))
        .and(document::document(
            document::query("prefix", document::string())
                .description("Prefix to filter files and folders by"),
        ))
        .and(document::document(document::description("Fetch a Tree")))
        .and(document::document(document::tag("Source")))
        .and(document::document(
            document::response(
                200,
                document::body(coco::Tree::document()).mime("application/json"),
            )
            .description("Tree for path found"),
        ))
        .and_then(handler::tree)
}

/// Source handlers for conversion between core domain and http request fullfilment.
mod handler {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use warp::path::Tail;
    use warp::{reply, Rejection, Reply};

    use crate::avatar;
    use crate::coco;
    use crate::error::Error;
    use crate::http;
    use crate::identity;
    use crate::registry;
    use crate::session;

    /// Fetch a [`coco::Blob`].
    pub async fn blob<R>(
        api: Arc<Mutex<coco::PeerApi>>,
        registry: http::Shared<R>,
        store: http::Shared<kv::Store>,
        project_urn: String,
        super::BlobQuery {
            path,
            peer_id,
            revision,
            highlight,
        }: super::BlobQuery,
    ) -> Result<impl Reply, Rejection>
    where
        R: registry::Client,
    {
        let registry = registry.read().await;
        let store = store.read().await;
        let session = session::current(Arc::clone(&api), &*registry, &store).await?;

        let api = api.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let project = coco::get_project(&*api, &urn)?;
        let default_branch = project.default_branch();
        let theme = if let Some(true) = highlight {
            Some(&session.settings.appearance.theme)
        } else {
            None
        };
        let blob = coco::with_browser(&*api, &urn, |mut browser| {
            coco::blob(
                &mut browser,
                peer_id.as_ref(),
                default_branch,
                revision,
                &path,
                theme,
            )
        })?;

        Ok(reply::json(&blob))
    }

    /// Fetch the list [`coco::Branch`].
    pub async fn branches(
        peer: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
    ) -> Result<impl Reply, Rejection> {
        let peer = peer.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let branches = coco::with_browser(&peer, &urn, |browser| coco::branches(browser))?;

        Ok(reply::json(&branches))
    }

    /// Fetch a [`coco::Commit`].
    pub async fn commit(
        api: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
        sha1: String,
    ) -> Result<impl Reply, Rejection> {
        let api = api.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let commit =
            coco::with_browser(&api, &urn, |mut browser| coco::commit(&mut browser, &sha1))?;

        Ok(reply::json(&commit))
    }

    /// Fetch the list of [`coco::Commit`] from a branch.
    pub async fn commits(
        api: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
        super::CommitsQuery { peer_id, branch }: super::CommitsQuery,
    ) -> Result<impl Reply, Rejection> {
        let api = api.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let commits = coco::with_browser(&api, &urn, |mut browser| {
            coco::commits(&mut browser, peer_id.as_ref(), &branch)
        })?;

        Ok(reply::json(&commits))
    }

    /// Fetch the list [`coco::Branch`] for a local repository.
    pub async fn local_state(path: Tail) -> Result<impl Reply, Rejection> {
        let state = coco::local_state(path.as_str())?;

        Ok(reply::json(&state))
    }

    /// Fetch the list [`coco::Branch`] and [`coco::Tag`].
    pub async fn revisions(
        peer: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
    ) -> Result<impl Reply, Rejection> {
        let peer = peer.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let (branches, tags) = coco::with_browser(&peer, &urn, |browser| {
            Ok((coco::branches(browser)?, coco::tags(browser)?))
        })?;

        // TODO(rudolfs): the order of the returned peers/revisions determines the default peer in
        // the repository selector in the UI. Make sure the list always returns the default peer
        // first.
        let revs = [
            (
                "cloudhead",
                "rad:git:hwd1yre85ddm5ruz4kgqppdtdgqgqr4wjy3fmskgebhpzwcxshei7d4ouwe",
            ),
            (
                "rudolfs",
                "rad:git:hwd1yrereyss6pihzu3f3k4783boykpwr1uzdn3cwugmmxwrpsay5ycyuro",
            ),
            (
                "xla",
                "rad:git:hwd1yreyu554sa1zgx4fxciwju1pk77uka84nrz5fu64at9zxuc8f698xmc",
            ),
        ]
        .iter()
        .map(|(fake_handle, fake_peer_urn)| super::Revision {
            branches: branches.clone(),
            tags: tags.clone(),
            identity: identity::Identity {
                // TODO(finto): Get the right URN
                id: fake_peer_urn
                    .parse()
                    .expect("failed to parse hardcoded URN"),
                metadata: identity::Metadata {
                    handle: (*fake_handle).to_string(),
                },
                avatar_fallback: avatar::Avatar::from(fake_handle, avatar::Usage::Identity),
                registered: None,
                shareable_entity_identifier: identity::SharedIdentifier {
                    handle: (*fake_handle).to_string(),
                    urn: fake_peer_urn
                        .parse()
                        .expect("failed to parse hardcoded URN"),
                },
            },
        })
        .collect::<Vec<super::Revision>>();

        Ok(reply::json(&revs))
    }

    /// Fetch the list [`coco::Tag`].
    pub async fn tags(
        peer: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
    ) -> Result<impl Reply, Rejection> {
        let peer = peer.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let tags = coco::with_browser(&peer, &urn, |browser| coco::tags(browser))?;

        Ok(reply::json(&tags))
    }

    /// Fetch a [`coco::Tree`].
    pub async fn tree(
        api: Arc<Mutex<coco::PeerApi>>,
        project_urn: String,
        super::TreeQuery {
            prefix,
            peer_id,
            revision,
        }: super::TreeQuery,
    ) -> Result<impl Reply, Rejection> {
        let api = api.lock().await;
        let urn = project_urn.parse().map_err(Error::from)?;
        let project = coco::get_project(&api, &urn)?;
        let default_branch = project.default_branch();
        let tree = coco::with_browser(&api, &urn, |mut browser| {
            coco::tree(
                &mut browser,
                peer_id.as_ref(),
                default_branch,
                revision,
                prefix,
            )
        })?;

        Ok(reply::json(&tree))
    }
}

/// Bundled query params to pass to the commits handler.
#[derive(Debug, Deserialize)]
pub struct CommitsQuery {
    /// PeerId to scope the query by.
    peer_id: Option<peer::PeerId>,
    /// Branch to get the commit history for.
    branch: String,
}

/// Bundled query params to pass to the blob handler.
#[derive(Debug, Deserialize)]
pub struct BlobQuery {
    /// Location of the blob in tree.
    path: String,
    /// PeerId to scope the query by.
    peer_id: Option<peer::PeerId>,
    /// Revision to use for the history of the repo.
    revision: Option<String>,
    /// Whether or not to syntax highlight the blob.
    highlight: Option<bool>,
}

/// Bundled query params to pass to the tree handler.
#[derive(Debug, Deserialize)]
pub struct TreeQuery {
    /// Path prefix to query the tree.
    prefix: Option<String>,
    /// PeerId to scope the query by.
    peer_id: Option<peer::PeerId>,
    /// Revision to query at.
    revision: Option<String>,
}

/// Bundled response to retrieve both branches and tags for a user repo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    /// Owner of the repo.
    identity: identity::Identity,
    /// List of [`coco::Branch`].
    branches: Vec<coco::Branch>,
    /// List of [`coco::Tag`].
    tags: Vec<coco::Tag>,
}

impl ToDocumentedType for Revision {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert("identity".into(), identity::Identity::document());
        properties.insert("branches".into(), document::array(coco::Branch::document()));
        properties.insert("tags".into(), document::array(coco::Tag::document()));

        document::DocumentedType::from(properties).description("Revision")
    }
}

impl Serialize for coco::Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Blob", 5)?;
        state.serialize_field("binary", &self.is_binary())?;
        state.serialize_field("html", &self.is_html())?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("info", &self.info)?;
        state.serialize_field("path", &self.path)?;
        state.end()
    }
}

impl ToDocumentedType for coco::Blob {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(4);
        properties.insert(
            "binary".into(),
            document::boolean()
                .description("Flag to indicate if the content of the Blob is binary")
                .example(true),
        );
        properties.insert(
            "html".into(),
            document::boolean()
                .description("Flag to indicate if the content of the Blob is HTML")
                .example(true),
        );
        properties.insert("content".into(), coco::BlobContent::document());
        properties.insert("info".into(), coco::Info::document());

        document::DocumentedType::from(properties).description("Blob")
    }
}

impl Serialize for coco::BlobContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Ascii(content) | Self::Html(content) => serializer.serialize_str(content),
            Self::Binary => serializer.serialize_none(),
        }
    }
}

impl ToDocumentedType for coco::BlobContent {
    fn document() -> document::DocumentedType {
        document::string()
            .description("BlobContent")
            .example("print 'hello world'")
            .nullable(true)
    }
}

impl ToDocumentedType for coco::Branch {
    fn document() -> document::DocumentedType {
        document::string().description("Branch").example("master")
    }
}

impl Serialize for coco::Commit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut changeset = serializer.serialize_struct("Commit", 3)?;
        changeset.serialize_field("header", &self.header)?;
        changeset.serialize_field("stats", &self.stats)?;
        changeset.serialize_field("diff", &self.diff)?;
        changeset.end()
    }
}

impl Serialize for coco::CommitHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CommitHeader", 6)?;
        state.serialize_field("sha1", &self.sha1.to_string())?;
        state.serialize_field("author", &self.author)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("description", &self.description())?;
        state.serialize_field("committer", &self.committer)?;
        state.serialize_field("committerTime", &self.committer_time.seconds())?;
        state.end()
    }
}

impl ToDocumentedType for coco::CommitHeader {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(6);
        properties.insert(
            "sha1".into(),
            document::string()
                .description("SHA1 of the Commit")
                .example("1e0206da8571ca71c51c91154e2fee376e09b4e7"),
        );
        properties.insert("author".into(), coco::Person::document());
        properties.insert(
            "summary".into(),
            document::string()
                .description("Commit message summary")
                .example("Add text files"),
        );
        properties.insert(
            "description".into(),
            document::string()
                .description("Commit description text")
                .example("Longer desription of the Commit changes."),
        );
        properties.insert("committer".into(), coco::Person::document());
        properties.insert(
            "committerTime".into(),
            document::string()
                .description("Time of the commit")
                .example("1575283425"),
        );
        document::DocumentedType::from(properties).description("CommitHeader")
    }
}

impl ToDocumentedType for coco::Commit {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert("header".into(), coco::CommitHeader::document());
        properties.insert(
            "stats".into(),
            document::string().description("Commit stats"),
        );
        properties.insert(
            "diff".into(),
            document::string().description("Commit changeset"),
        );
        document::DocumentedType::from(properties).description("Commit")
    }
}

impl Serialize for coco::Info {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Info", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("objectType", &self.object_type)?;
        state.serialize_field("lastCommit", &self.last_commit)?;
        state.end()
    }
}

impl ToDocumentedType for coco::Info {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "name".into(),
            document::string()
                .description("Name of the file")
                .example("arrows.txt"),
        );
        properties.insert("objectType".into(), coco::ObjectType::document());
        properties.insert("lastCommit".into(), coco::Commit::document());

        document::DocumentedType::from(properties).description("Info")
    }
}

impl Serialize for coco::ObjectType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Blob => serializer.serialize_unit_variant("ObjectType", 0, "BLOB"),
            Self::Tree => serializer.serialize_unit_variant("ObjectType", 1, "TREE"),
        }
    }
}

impl ToDocumentedType for coco::ObjectType {
    fn document() -> document::DocumentedType {
        document::enum_string(vec!["BLOB".to_string(), "TREE".to_string()])
            .description("Object type variants")
            .example(Self::Blob)
    }
}

impl Serialize for coco::Person {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Person", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("email", &self.email)?;
        state.end()
    }
}

impl ToDocumentedType for coco::Person {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "name".into(),
            document::string()
                .description("Name part of the commit signature.")
                .example("Alexis Sellier"),
        );
        properties.insert(
            "email".into(),
            document::string()
                .description("Email part of the commit signature.")
                .example("self@cloudhead.io"),
        );

        document::DocumentedType::from(properties).description("Person")
    }
}

impl Serialize for coco::Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl ToDocumentedType for coco::Tag {
    fn document() -> document::DocumentedType {
        document::string().description("Tag").example("v0.1.0")
    }
}

impl Serialize for coco::Tree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 3)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

impl ToDocumentedType for coco::Tree {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(3);
        properties.insert(
            "path".into(),
            document::string()
                .description("Absolute path to the tree object from the repo root.")
                .example("ui/src"),
        );
        properties.insert(
            "entries".into(),
            document::array(coco::TreeEntry::document())
                .description("Entries listed in that tree result."),
        );
        properties.insert("info".into(), coco::Info::document());

        document::DocumentedType::from(properties).description("Tree")
    }
}

impl Serialize for coco::TreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

impl ToDocumentedType for coco::TreeEntry {
    fn document() -> document::DocumentedType {
        let mut properties = std::collections::HashMap::with_capacity(2);
        properties.insert(
            "path".into(),
            document::string()
                .description("Absolute path to the object from the root of the repo.")
                .example("ui/src/main.ts"),
        );
        properties.insert("info".into(), coco::Info::document());

        document::DocumentedType::from(properties).description("TreeEntry")
    }
}

#[allow(clippy::non_ascii_literal, clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio::sync::{Mutex, RwLock};
    use warp::http::StatusCode;
    use warp::test::request;

    use librad::keys::SecretKey;

    use crate::avatar;
    use crate::coco;
    use crate::error;
    use crate::http;
    use crate::identity;
    use crate::registry;

    #[tokio::test]
    async fn blob() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = Arc::new(Mutex::new(coco::create_peer_api(config).await?));
        let owner = coco::init_user(&*peer.lock().await, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &*peer.lock().await,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let revision = "master";
        let default_branch = platinum_project.default_branch();
        let path = "text/arrows.txt";
        let peer_id = (*peer.lock().await).peer_id().clone();
        let want = coco::with_browser(&*peer.lock().await, &urn, |mut browser| {
            coco::blob(
                &mut browser,
                Some(&peer_id),
                default_branch,
                Some(revision.to_string()),
                path,
                None,
            )
        })?;

        let api = super::filters(
            Arc::clone(&peer),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );

        // Get ASCII blob.
        let res = request()
            .method("GET")
            .path(&format!(
                "/blob/{}?revision={}&path={}",
                urn, revision, path
            ))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!({
                    "binary": false,
                    "html": false,
                    "content": "  ;;;;;        ;;;;;        ;;;;;
  ;;;;;        ;;;;;        ;;;;;
  ;;;;;        ;;;;;        ;;;;;
  ;;;;;        ;;;;;        ;;;;;
..;;;;;..    ..;;;;;..    ..;;;;;..
 ':::::'      ':::::'      ':::::'
   ':`          ':`          ':`
",
                    "info": {
                        "name": "arrows.txt",
                        "objectType": "BLOB",
                        "lastCommit": {
                            "sha1": "1e0206da8571ca71c51c91154e2fee376e09b4e7",
                            "author": {
                                "name": "Rūdolfs Ošiņš",
                                "email": "rudolfs@osins.org",
                            },
                            "committer": {
                                "name": "Rūdolfs Ošiņš",
                                "email": "rudolfs@osins.org",
                            },
                            "summary": "Add text files",
                            "description": "",
                            "committerTime": 1_575_283_425,
                        },
                    },
                    "path": "text/arrows.txt",
                })
            );
        });

        // Get binary blob.
        let path = "bin/ls";
        let res = request()
            .method("GET")
            .path(&format!(
                "/blob/{}?revision={}&path={}",
                urn.to_string(),
                revision,
                path
            ))
            .reply(&api)
            .await;

        let want = coco::with_browser(&*peer.lock().await, &urn, |browser| {
            coco::blob(
                browser,
                None,
                default_branch,
                Some(revision.to_string()),
                path,
                None,
            )
        })?;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!({
                    "binary": true,
                    "html": false,
                    "content": Value::Null,
                    "info": {
                        "name": "ls",
                        "objectType": "BLOB",
                        "lastCommit": {
                            "sha1": "19bec071db6474af89c866a1bd0e4b1ff76e2b97",
                            "author": {
                                "name": "Rūdolfs Ošiņš",
                                "email": "rudolfs@osins.org",
                            },
                            "committer": {
                                "name": "Rūdolfs Ošiņš",
                                "email": "rudolfs@osins.org",
                            },
                            "summary": "Add some binary files",
                            "description": "",
                            "committerTime": 1_575_282_964, },
                    },
                    "path": "bin/ls",
                })
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn branches() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let want = coco::with_browser(&peer, &urn, |browser| coco::branches(browser))?;

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!("/branches/{}", urn.to_string()))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(have, json!(["dev", "master"]));
        });

        Ok(())
    }

    #[tokio::test]
    #[allow(clippy::indexing_slicing)]
    async fn commit() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;

        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let sha1 = "3873745c8f6ffb45c990eb23b491d4b4b6182f95";
        let want = coco::with_browser(&peer, &urn, |mut browser| {
            coco::commit_header(&mut browser, sha1)
        })?;

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!("/commit/{}/{}", urn.to_string(), sha1))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have["header"], json!(want));
            assert_eq!(
                have["header"],
                json!({
                    "sha1": sha1,
                    "author": {
                        "name": "Fintan Halpenny",
                        "email": "fintan.halpenny@gmail.com",
                    },
                    "committer": {
                        "email": "noreply@github.com",
                        "name": "GitHub",
                    },
                    "summary": "Extend the docs (#2)",
                    "description": "I want to have files under src that have separate commits.\r\nThat way src\'s latest commit isn\'t the same as all its files, instead it\'s the file that was touched last.",
                    "committerTime": 1_578_309_972,
                }),
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn commits() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let branch = "master";
        let head = "223aaf87d6ea62eef0014857640fd7c8dd0f80b5";
        let peer_id = &peer.peer_id().clone();
        let (want, head_commit) = coco::with_browser(&peer, &urn, |mut browser| {
            let want = coco::commits(&mut browser, Some(peer_id), branch)?;
            let head_commit = coco::commit_header(&mut browser, head)?;
            Ok((want, head_commit))
        })?;

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!("/commits/{}?branch={}", urn.to_string(), branch))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(have.as_array().unwrap().len(), 14);
            assert_eq!(
                have.as_array().unwrap().first().unwrap(),
                &serde_json::to_value(&head_commit).unwrap(),
                "the first commit is the head of the branch"
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn local_state() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let config = coco::config::default(key.clone(), &tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;

        let path = "../fixtures/git-platinum";
        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!("/local-state/{}", path))
            .reply(&api)
            .await;

        let want = coco::local_state(path).unwrap();

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!({
                    "branches": [
                        "dev",
                        "master",
                    ],
                    "managed": false,
                }),
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn revisions() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let want = {
            let (branches, tags) = coco::with_browser(&peer, &urn, |browser| {
                Ok((coco::branches(browser)?, coco::tags(browser)?))
            })?;

            [
                (
                    "cloudhead",
                    "rad:git:hwd1yre85ddm5ruz4kgqppdtdgqgqr4wjy3fmskgebhpzwcxshei7d4ouwe",
                ),
                (
                    "rudolfs",
                    "rad:git:hwd1yrereyss6pihzu3f3k4783boykpwr1uzdn3cwugmmxwrpsay5ycyuro",
                ),
                (
                    "xla",
                    "rad:git:hwd1yreyu554sa1zgx4fxciwju1pk77uka84nrz5fu64at9zxuc8f698xmc",
                ),
            ]
            .iter()
            .map(|(fake_handle, fake_peer_urn)| super::Revision {
                branches: branches.clone(),
                tags: tags.clone(),
                identity: identity::Identity {
                    // TODO(finto): Get the right URN
                    id: fake_peer_urn
                        .parse()
                        .expect("failed to parse hardcoded URN"),
                    metadata: identity::Metadata {
                        handle: (*fake_handle).to_string(),
                    },
                    avatar_fallback: avatar::Avatar::from(fake_handle, avatar::Usage::Identity),
                    registered: None,
                    shareable_entity_identifier: identity::SharedIdentifier {
                        handle: (*fake_handle).to_string(),
                        urn: fake_peer_urn
                            .parse()
                            .expect("failed to parse hardcoded URN"),
                    },
                },
            })
            .collect::<Vec<super::Revision>>()
        };

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );

        let res = request()
            .method("GET")
            .path(&format!("/revisions/{}", urn))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!([
                    {
                        "identity": {
                            "id": "rad:git:hwd1yre85ddm5ruz4kgqppdtdgqgqr4wjy3fmskgebhpzwcxshei7d4ouwe",
                            "metadata": {
                                "handle": "cloudhead",
                            },
                            "registered": Value::Null,
                            "shareableEntityIdentifier": format!("cloudhead@{}", "rad:git:hwd1yre85ddm5ruz4kgqppdtdgqgqr4wjy3fmskgebhpzwcxshei7d4ouwe"),
                            "avatarFallback": {
                                "background": {
                                    "r": 24,
                                    "g": 105,
                                    "b": 216,
                                },
                                "emoji": "🌻",
                            },
                        },
                        "branches": [ "dev", "master" ],
                        "tags": [ "v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0", "v0.5.0" ]
                    },
                    {
                        "identity": {
                            "id": "rad:git:hwd1yrereyss6pihzu3f3k4783boykpwr1uzdn3cwugmmxwrpsay5ycyuro",
                            "metadata": {
                                "handle": "rudolfs",
                            },
                            "registered": Value::Null,
                            "shareableEntityIdentifier": format!("rudolfs@{}", "rad:git:hwd1yrereyss6pihzu3f3k4783boykpwr1uzdn3cwugmmxwrpsay5ycyuro"),
                            "avatarFallback": {
                                "background": {
                                    "r": 24,
                                    "g": 186,
                                    "b": 214,
                                },
                            "emoji": "🧿",
                            },
                        },
                        "branches": [ "dev", "master" ],
                        "tags": [ "v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0", "v0.5.0" ]
                    },
                    {
                        "identity": {
                            "id": "rad:git:hwd1yreyu554sa1zgx4fxciwju1pk77uka84nrz5fu64at9zxuc8f698xmc",
                            "metadata": {
                                "handle": "xla",
                            },
                            "registered": Value::Null,
                            "shareableEntityIdentifier": format!("xla@{}", "rad:git:hwd1yreyu554sa1zgx4fxciwju1pk77uka84nrz5fu64at9zxuc8f698xmc"),
                            "avatarFallback": {
                                "background": {
                                    "r": 155,
                                    "g": 157,
                                    "b": 169,
                                },
                            "emoji": "🗺",
                            },
                        },
                        "branches": [ "dev", "master" ],
                        "tags": [ "v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0", "v0.5.0" ]
                    },
                ]),
            )
        });

        Ok(())
    }

    #[tokio::test]
    async fn tags() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let want = coco::with_browser(&peer, &urn, |browser| coco::tags(browser))?;

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!("/tags/{}", urn.to_string()))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!(["v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0", "v0.5.0"]),
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn tree() -> Result<(), error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let key = SecretKey::new();
        let registry = {
            let (client, _) = radicle_registry_client::Client::new_emulator();
            registry::Registry::new(client)
        };
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;
        let config = coco::config::default(key.clone(), tmp_dir)?;
        let peer = coco::create_peer_api(config).await?;
        let owner = coco::init_user(&peer, key.clone(), "cloudhead")?;
        let owner = coco::verify_user(owner)?;
        let platinum_project = coco::control::replicate_platinum(
            &peer,
            key,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )?;
        let urn = platinum_project.urn();

        let revision = "master";
        let prefix = "src";

        let default_branch = platinum_project.default_branch();
        let peer_id = &peer.peer_id();
        let want = coco::with_browser(&peer, &urn, |mut browser| {
            coco::tree(
                &mut browser,
                Some(peer_id),
                default_branch,
                Some(revision.to_string()),
                Some(prefix.to_string()),
            )
        })?;

        let api = super::filters(
            Arc::new(Mutex::new(peer)),
            Arc::new(RwLock::new(registry)),
            Arc::new(RwLock::new(store)),
        );
        let res = request()
            .method("GET")
            .path(&format!(
                "/tree/{}?revision={}&prefix={}",
                urn.to_string(),
                revision,
                prefix
            ))
            .reply(&api)
            .await;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!({
                    "path": "src",
                    "info": {
                        "name": "src",
                        "objectType": "TREE",
                        "lastCommit": null,                },
                        "entries": [
                        {
                            "path": "src/Eval.hs",
                            "info": {
                                "name": "Eval.hs",
                                "objectType": "BLOB",
                                "lastCommit": null,
                            },
                        },
                        {
                            "path": "src/memory.rs",
                            "info": {
                                "name": "memory.rs",
                                "objectType": "BLOB",
                                "lastCommit": null,
                            },
                        },
                    ],
                }),
            );
        });

        Ok(())
    }
}
