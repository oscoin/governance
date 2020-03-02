use std::convert::From;
use std::convert::TryFrom;
use std::env;
use std::str::FromStr;
use std::sync;

use librad::paths::Paths;
use librad::surf;
use librad::surf::git::git2;
use radicle_registry_client::ed25519;

use super::project;
use crate::coco;
use crate::error;
use crate::registry;

/// Glue to bundle our read and write APIs together.
pub type Schema = juniper::RootNode<'static, Query, Mutation>;

/// Returns a `Schema` with the default parameterised `Query` and `Mutation`.
#[must_use]
pub fn create() -> Schema {
    Schema::new(Query {}, Mutation {})
}

/// Container for data access from handlers.
#[derive(Clone)]
pub struct Context {
    /// Root on the filesystem for the librad config and storage paths.
    librad_paths: Paths,
    /// Wrapper to interact with the Registry.
    registry: sync::Arc<sync::RwLock<registry::Registry>>,
}

impl Context {
    /// Returns a new `Context`.
    #[must_use]
    pub fn new(librad_paths: Paths, registry_client: radicle_registry_client::Client) -> Self {
        Self {
            librad_paths,
            registry: sync::Arc::new(sync::RwLock::new(registry::Registry::new(registry_client))),
        }
    }
}

impl juniper::Context for Context {}

/// Encapsulates write path in API.
pub struct Mutation;

#[juniper::object(
    Context = Context,
    name = "UpstreamMutation",
)]
impl Mutation {
    fn create_project(
        ctx: &Context,
        metadata: project::MetadataInput,
        path: String,
        publish: bool,
    ) -> Result<project::Project, error::Error> {
        if surf::git::git2::Repository::open(path.clone()).is_err() {
            coco::init_repo(path.clone())?;
        };

        let (id, meta) = coco::init_project(
            &ctx.librad_paths,
            &path,
            &metadata.name,
            &metadata.description,
            &metadata.default_branch,
            &metadata.img_url,
        )?;

        Ok(project::Project {
            id: id.to_string().into(),
            metadata: meta.into(),
        })
    }

    fn register_project(
        ctx: &Context,
        project_name: String,
        org_id: String,
        maybe_librad_id_input: Option<juniper::ID>,
    ) -> Result<registry::Transaction, error::Error> {
        let maybe_librad_id = maybe_librad_id_input.map(|id| {
            librad::project::ProjectId::from_str(&id.to_string())
                .expect("unable to parse project id")
        });

        // TODO(xla): Get keypair from persistent storage.
        let fake_pair = ed25519::Pair::from_legacy_string("//Robot", None);
        // TODO(xla): Remove single-threaded executor once async/await lands in juniper:
        // https://github.com/graphql-rust/juniper/pull/497
        futures::executor::block_on(ctx.registry.read().unwrap().register_project(
            &fake_pair,
            project_name,
            org_id,
            maybe_librad_id,
        ))
    }
}

/// Encapsulates read paths in API.
pub struct Query;

#[juniper::object(
    Context = Context,
    name = "UpstreamQuery",
)]
impl Query {
    fn apiVersion() -> &str {
        "1.0"
    }

    fn blob(
        ctx: &Context,
        id: juniper::ID,
        revision: String,
        path: String,
    ) -> Result<coco::Blob, error::Error> {
        coco::blob(&ctx.librad_paths, &id.to_string(), &revision, &path)
    }

    fn commit(ctx: &Context, id: juniper::ID, sha1: String) -> Result<coco::Commit, error::Error> {
        coco::commit(&ctx.librad_paths, &id.to_string(), &sha1)
    }

    fn branches(ctx: &Context, id: juniper::ID) -> Result<Vec<String>, error::Error> {
        Ok(coco::branches(&ctx.librad_paths, &id.to_string())?
            .into_iter()
            .map(|t| t.to_string())
            .collect())
    }

    fn local_branches(ctx: &Context, path: String) -> Result<Vec<String>, error::Error> {
        Ok(coco::local_branches(&path)?
            .into_iter()
            .map(|t| t.to_string())
            .collect())
    }

    fn tags(ctx: &Context, id: juniper::ID) -> Result<Vec<String>, error::Error> {
        Ok(coco::tags(&ctx.librad_paths, &id.to_string())?
            .into_iter()
            .map(|t| t.to_string())
            .collect())
    }

    fn tree(
        ctx: &Context,
        id: juniper::ID,
        revision: String,
        prefix: String,
    ) -> Result<coco::Tree, error::Error> {
        coco::tree(&ctx.librad_paths, &id, &revision, &prefix)
    }

    fn project(ctx: &Context, id: juniper::ID) -> Result<project::Project, error::Error> {
        let meta = coco::get_project_meta(&ctx.librad_paths, &id.to_string())?;

        Ok(project::Project {
            id,
            metadata: meta.into(),
        })
    }

    fn projects(ctx: &Context) -> Result<Vec<project::Project>, error::Error> {
        let projects = coco::list_projects(&ctx.librad_paths)
            .into_iter()
            .map(|(id, meta)| project::Project {
                id: juniper::ID::new(id.to_string()),
                metadata: meta.into(),
            })
            .collect::<Vec<project::Project>>();

        Ok(projects)
    }

    fn list_registry_projects(ctx: &Context) -> Result<Vec<juniper::ID>, error::Error> {
        let ids = futures::executor::block_on(ctx.registry.read().unwrap().list_projects())?;

        Ok(ids
            .iter()
            .map(|id| juniper::ID::from(id.0.to_string()))
            .collect::<Vec<juniper::ID>>())
    }
}

/// Bundles `Query` and `Mutation` used for controlling raw state.
pub type Control = juniper::RootNode<'static, ControlQuery, ControlMutation>;

/// Returns the [`Control`] schema used for controlling raw state.
#[must_use]
pub fn create_control() -> Control {
    Control::new(ControlQuery {}, ControlMutation {})
}

/// Control mutations.
pub struct ControlMutation;

#[juniper::object(
    Context = Context,
    name = "ControlMutation",
    description = "Mutations to control raw proxy state.",
)]
impl ControlMutation {
    fn create_project_with_fixture(
        ctx: &Context,
        metadata: project::MetadataInput,
    ) -> Result<project::Project, error::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let repos_dir = tempfile::tempdir_in(tmp_dir.path())?;

        // Craft the absolute path to git-platinum fixtures.
        let mut platinum_path = env::current_dir().expect("unable to get working directory");
        platinum_path.push("../fixtures/git-platinum");
        let mut platinum_from = String::from("file://");
        platinum_from.push_str(
            platinum_path
                .to_str()
                .expect("unable to get fixtures path string"),
        );

        // Construct path for fixtures to clone into.
        let platinum_into = tmp_dir.path().join("git-platinum");

        // Clone a copy into temp directory.
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.download_tags(git2::AutotagOption::All);

        let platinum_repo = git2::build::RepoBuilder::new()
            .branch("master")
            .clone_local(git2::build::CloneLocal::Auto)
            .fetch_options(fetch_options)
            .clone(&platinum_from, platinum_into.as_path())
            .expect("unable to clone fixtures repo");

        let (id, meta) = coco::init_project(
            &ctx.librad_paths,
            &platinum_into.to_str().unwrap(),
            &metadata.name,
            &metadata.description,
            &metadata.default_branch,
            &metadata.img_url,
        )?;

        Ok(project::Project {
            id: id.to_string().into(),
            metadata: meta.into(),
        })
    }

    fn nuke_coco_state(ctx: &Context) -> Result<bool, error::Error> {
        std::fs::remove_dir_all(ctx.librad_paths.keys_dir())?;
        std::fs::remove_dir_all(ctx.librad_paths.profiles_dir())?;
        std::fs::remove_dir_all(ctx.librad_paths.projects_dir())?;

        Ok(true)
    }

    fn nuke_registry_state(ctx: &Context) -> Result<bool, error::Error> {
        ctx.registry
            .write()
            .unwrap()
            .reset(radicle_registry_client::Client::new_emulator());

        Ok(true)
    }
}

/// Control query endpoints.
pub struct ControlQuery;

#[juniper::object(
    Context = Context,
    name = "ControlQuery",
    description = "Queries to access raw proxy state.",
)]
impl ControlQuery {}

#[juniper::object]
impl coco::Blob {
    fn binary(&self) -> bool {
        match &self.content {
            coco::BlobContent::Ascii(_content) => false,
            coco::BlobContent::Binary => true,
        }
    }

    fn content(&self) -> Option<String> {
        match &self.content {
            coco::BlobContent::Ascii(content) => Some(content.clone()),
            coco::BlobContent::Binary => None,
        }
    }

    fn info(&self) -> &coco::Info {
        &self.info
    }
}

#[juniper::object]
impl coco::Commit {
    fn sha1(&self) -> String {
        self.sha1.to_string()
    }

    fn author(&self) -> &coco::Person {
        &self.author
    }

    fn summary(&self) -> &str {
        &self.summary
    }

    fn message(&self) -> &str {
        &self.message
    }

    fn committer_time(&self) -> String {
        self.committer_time.seconds().to_string()
    }
}

#[juniper::object]
impl coco::Info {
    fn name(&self) -> &str {
        &self.name
    }

    fn object_type(&self) -> ObjectType {
        match self.object_type {
            coco::ObjectType::Blob => ObjectType::Blob,
            coco::ObjectType::Tree => ObjectType::Tree,
        }
    }

    fn last_commit(&self) -> Option<&coco::Commit> {
        self.last_commit.as_ref()
    }
}

/// Git object types.
///
/// <https://git-scm.com/book/en/v2/Git-Internals-Git-Objects>
#[derive(GraphQLEnum)]
enum ObjectType {
    /// Directory tree.
    Tree,
    /// Text or binary blob of a file.
    Blob,
}

/// Contextual information for an org registration message.
#[derive(juniper::GraphQLObject)]
struct OrgRegistration {
    /// The ID of the org.
    org_id: String,
}

/// Contextual information for an org unregistration message.
#[derive(juniper::GraphQLObject)]
struct OrgUnregistration {
    /// The ID of the org.
    org_id: String,
}

/// Contextual information for a project registration message.
#[derive(juniper::GraphQLObject)]
struct ProjectRegistration {
    /// Actual project name, unique under org.
    project_name: String,
    /// The org under which to register the project.
    org_id: String,
}

/// Message types supproted in transactions.
enum Message {
    /// Registration of a new org.
    OrgRegistration(OrgRegistration),

    /// Registration of a new org.
    OrgUnregistration(OrgUnregistration),

    /// Registration of a new project.
    ProjectRegistration(ProjectRegistration),
}

juniper::graphql_union!(Message: () where Scalar = <S> |&self| {
    instance_resolvers: |_| {
        &ProjectRegistration => match *self {
            Message::ProjectRegistration(ref p) => Some(p),
            _ => None
        },
        &OrgRegistration => match *self {
            Message::OrgRegistration(ref o) => Some(o),
            _ => None
        },
        &OrgUnregistration => match *self {
            Message::OrgUnregistration(ref o) => Some(o),
            _ => None
        },
    }
});

#[juniper::object]
impl coco::Person {
    fn name(&self) -> &str {
        &self.name
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn avatar(&self) -> &str {
        &self.avatar
    }
}

#[juniper::object]
impl registry::Transaction {
    fn id(&self) -> juniper::ID {
        juniper::ID::new(self.id.to_string())
    }

    fn messages(&self) -> Vec<Message> {
        self.messages
            .iter()
            .map(|m| match m {
                registry::Message::OrgRegistration(org_id) => {
                    Message::OrgRegistration(OrgRegistration {
                        org_id: org_id.to_string(),
                    })
                },
                registry::Message::OrgUnregistration(org_id) => {
                    Message::OrgUnregistration(OrgUnregistration {
                        org_id: org_id.to_string(),
                    })
                },
                registry::Message::ProjectRegistration {
                    project_name,
                    org_id,
                } => Message::ProjectRegistration(ProjectRegistration {
                    project_name: project_name.to_string(),
                    org_id: org_id.to_string(),
                }),
            })
            .collect()
    }

    fn state(&self) -> TransactionState {
        match self.state {
            registry::TransactionState::Applied(block_hash) => TransactionState::Applied(Applied {
                block: juniper::ID::new(block_hash.to_string()),
            }),
        }
    }

    fn timestamp(&self) -> juniper::FieldResult<String> {
        let since_epoch = i64::try_from(
            self.timestamp
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        )?;
        let git_time = git2::Time::new(since_epoch, 0).seconds().to_string();

        Ok(git_time)
    }
}

/// States a transaction can go through.
enum TransactionState {
    /// The transaction has been applied to a block.
    Applied(Applied),
}

/// Context for a chain applied transaction.
#[derive(GraphQLObject)]
struct Applied {
    /// Block hash the transaction was included in.
    block: juniper::ID,
}

juniper::graphql_union!(TransactionState: () where Scalar = <S> |&self| {
    instance_resolvers: |_| {
        &Applied => match *self { TransactionState::Applied(ref a) => Some(a) },
    }
});

#[juniper::object]
impl coco::Tree {
    fn path(&self) -> &str {
        &self.path
    }

    fn entries(&self) -> &Vec<coco::TreeEntry> {
        self.entries.as_ref()
    }

    fn info(&self) -> &coco::Info {
        &self.info
    }
}

#[juniper::object]
impl coco::TreeEntry {
    fn info(&self) -> &coco::Info {
        &self.info
    }

    fn path(&self) -> String {
        self.path.clone()
    }
}
