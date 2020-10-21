//! Endpoints and serialisation for source code browsing.

use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, path, Filter, Rejection, Reply};

use radicle_surf::vcs::git;

use crate::{context, http};

/// Combination of all source filters.
pub fn filters(ctx: context::Context) -> BoxedFilter<(impl Reply,)> {
    blob_filter(ctx.clone())
        .or(branches_filter(ctx.clone()))
        .or(commit_filter(ctx.clone()))
        .or(commits_filter(ctx.clone()))
        .or(local_state_filter())
        .or(tags_filter(ctx.clone()))
        .or(tree_filter(ctx))
        .boxed()
}

/// `GET /blob/<project_urn>?revision=<revision>&path=<path>`
fn blob_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("blob")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(http::with_qs::<BlobQuery>())
        .and_then(handler::blob)
}

/// `GET /branches/<project_urn>?peerId=<peer_id>`
fn branches_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("branches")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(warp::query::<BranchQuery>())
        .and_then(handler::branches)
}

/// `GET /commit/<project_urn>/<sha1>`
fn commit_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("commit")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(path::param::<coco::oid::Oid>())
        .and_then(handler::commit)
}

/// `GET /commits/<project_urn>?branch=<branch>&peerId=<peer_id>`
fn commits_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("commits")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(warp::query::<CommitsQuery>())
        .and_then(handler::commits)
}

/// `GET /local-state/<path>`
fn local_state_filter() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("local-state")
        .and(warp::get())
        .and(path::tail())
        .and_then(handler::local_state)
}

/// `GET /tags/<project_urn>?peer_id=<peer_id>`
fn tags_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("tags")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(warp::query::<TagQuery>())
        .and(path::end())
        .and_then(handler::tags)
}

/// `GET /tree/<project_id>?peerId=<peer_id>&prefix=<prefix>*revision=<revision>`
fn tree_filter(
    ctx: context::Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path("tree")
        .and(warp::get())
        .and(http::with_context(ctx))
        .and(path::param::<coco::Urn>())
        .and(http::with_qs::<TreeQuery>())
        .and_then(handler::tree)
}

/// Source handlers for conversion between core domain and http request fullfilment.
mod handler {
    use warp::{path::Tail, reply, Rejection, Reply};

    use coco::oid;

    use crate::{context, error, session, session::settings};

    /// Fetch a [`coco::Blob`].
    pub async fn blob(
        ctx: context::Context,
        project_urn: coco::Urn,
        super::BlobQuery {
            path,
            peer_id,
            revision,
            highlight,
        }: super::BlobQuery,
    ) -> Result<impl Reply, Rejection> {
        let current_session = session::current(ctx.state.clone(), &ctx.store).await?;
        let peer_id = super::http::guard_self_peer_id(&ctx.state, peer_id);
        let revision = super::http::guard_self_revision(&ctx.state, revision);

        let theme = if let Some(true) = highlight {
            Some(match &current_session.settings.appearance.theme {
                settings::Theme::Dark => "base16-ocean.dark",
                settings::Theme::Light => "base16-ocean.light",
            })
        } else {
            None
        };

        let branch = ctx
            .state
            .get_branch(project_urn, peer_id, None)
            .await
            .map_err(error::Error::from)?;
        let blob = ctx
            .state
            .with_browser(branch, |mut browser| {
                coco::blob(&mut browser, revision, &path, theme)
            })
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&blob))
    }

    /// Fetch the list [`coco::Branch`].
    pub async fn branches(
        ctx: context::Context,
        project_urn: coco::Urn,
        super::BranchQuery { peer_id }: super::BranchQuery,
    ) -> Result<impl Reply, Rejection> {
        let peer_id = super::http::guard_self_peer_id(&ctx.state, peer_id);
        let default_branch = ctx
            .state
            .get_branch(project_urn, peer_id, None)
            .await
            .map_err(error::Error::from)?;
        let branches = ctx
            .state
            .with_browser(default_branch, |browser| {
                coco::branches(browser, Some(coco::into_branch_type(peer_id)))
            })
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&branches))
    }

    /// Fetch a [`coco::Commit`].
    pub async fn commit(
        ctx: context::Context,
        project_urn: coco::Urn,
        sha1: oid::Oid,
    ) -> Result<impl Reply, Rejection> {
        let default_branch = ctx
            .state
            .find_default_branch(project_urn)
            .await
            .map_err(error::Error::from)?;
        let commit = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::commit(&mut browser, sha1)
            })
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&commit))
    }

    /// Fetch the list of [`coco::Commit`] from a branch.
    pub async fn commits(
        ctx: context::Context,
        project_urn: coco::Urn,
        mut query: super::CommitsQuery,
    ) -> Result<impl Reply, Rejection> {
        let peer_id = super::http::guard_self_peer_id(&ctx.state, query.peer_id);
        query.peer_id = peer_id;

        let default_branch = ctx
            .state
            .find_default_branch(project_urn)
            .await
            .map_err(error::Error::from)?;
        let commits = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::commits(&mut browser, query.into())
            })
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&commits))
    }

    /// Fetch the list [`coco::Branch`] for a local repository.
    pub async fn local_state(path: Tail) -> Result<impl Reply, Rejection> {
        let state = coco::local_state(path.as_str())
            .map_err(coco::state::Error::from)
            .map_err(error::Error::from)?;

        Ok(reply::json(&state))
    }

    /// Fetch the list [`coco::Tag`].
    pub async fn tags(
        ctx: context::Context,
        project_urn: coco::Urn,
        super::TagQuery { peer_id }: super::TagQuery,
    ) -> Result<impl Reply, Rejection> {
        if let Some(_peer_id) = peer_id {
            let tags: Vec<coco::Tag> = vec![];
            return Ok(reply::json(&tags));
        }

        let branch = ctx
            .state
            .find_default_branch(project_urn)
            .await
            .map_err(error::Error::from)?;
        let tags = ctx
            .state
            .with_browser(branch, |browser| coco::tags(browser))
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&tags))
    }

    /// Fetch a [`coco::Tree`].
    pub async fn tree(
        ctx: context::Context,
        project_urn: coco::Urn,
        super::TreeQuery {
            prefix,
            peer_id,
            revision,
        }: super::TreeQuery,
    ) -> Result<impl Reply, Rejection> {
        let peer_id = super::http::guard_self_peer_id(&ctx.state, peer_id);
        let revision = super::http::guard_self_revision(&ctx.state, revision);
        let branch = ctx
            .state
            .get_branch(project_urn, peer_id, None)
            .await
            .map_err(error::Error::from)?;
        let tree = ctx
            .state
            .with_browser(branch, |mut browser| {
                coco::tree(&mut browser, revision, prefix)
            })
            .await
            .map_err(error::Error::from)?;

        Ok(reply::json(&tree))
    }
}

/// Bundled query params to pass to the commits handler.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitsQuery {
    /// PeerId to scope the query by.
    peer_id: Option<coco::PeerId>,
    /// Branch to get the commit history for.
    branch: String,
}

impl From<CommitsQuery> for git::Branch {
    fn from(CommitsQuery { peer_id, branch }: CommitsQuery) -> Self {
        match peer_id {
            None => Self::local(&branch),
            Some(peer_id) => Self::remote(&format!("heads/{}", branch), &peer_id.to_string()),
        }
    }
}

/// Bundled query params to pass to the blob handler.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobQuery {
    /// Location of the blob in tree.
    path: String,
    /// PeerId to scope the query by.
    peer_id: Option<coco::PeerId>,
    /// Revision to query at.
    revision: Option<coco::Revision<coco::PeerId>>,
    /// Whether or not to syntax highlight the blob.
    highlight: Option<bool>,
}

/// A query param for [`handler::branches`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchQuery {
    /// PeerId to scope the query by.
    peer_id: Option<coco::PeerId>,
}

/// Bundled query params to pass to the tree handler.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeQuery {
    /// Path prefix to query the tree.
    prefix: Option<String>,
    /// PeerId to scope the query by.
    peer_id: Option<coco::PeerId>,
    /// Revision to query at.
    revision: Option<coco::Revision<coco::PeerId>>,
}

/// A query param for [`handler::tags`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagQuery {
    /// PeerId to scope the query by.
    peer_id: Option<coco::PeerId>,
}

#[allow(clippy::non_ascii_literal, clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use std::{convert::TryFrom, env};

    use pretty_assertions::assert_eq;
    use serde_json::{json, Value};
    use warp::{http::StatusCode, test::request};

    use radicle_surf::vcs::git;

    use crate::{context, error, http};

    #[tokio::test]
    async fn blob() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let urn = replicate_platinum(&ctx).await?;
        let revision = coco::Revision::Branch {
            name: "master".to_string(),
            peer_id: None,
        };
        let path = "text/arrows.txt";
        let default_branch = ctx.state.find_default_branch(urn.clone()).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::blob(&mut browser, Some(revision.clone()), path, None)
            })
            .await?;

        let query = super::BlobQuery {
            path: path.to_string(),
            peer_id: None,
            revision: Some(revision.clone()),
            highlight: Some(false),
        };

        let path = format!("/blob/{}?{}", urn, serde_qs::to_string(&query).unwrap());

        // Get ASCII blob.
        let res = request().method("GET").path(&path).reply(&api).await;

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
        let default_branch = ctx.state.find_default_branch(urn.clone()).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |browser| {
                coco::blob(browser, Some(revision.clone()), path, None)
            })
            .await?;

        let query = super::BlobQuery {
            path: path.to_string(),
            peer_id: None,
            revision: Some(revision),
            highlight: Some(false),
        };

        let path = format!("/blob/{}?{}", urn, serde_qs::to_string(&query).unwrap());

        let res = request().method("GET").path(&path).reply(&api).await;

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
    async fn blob_dev_branch() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let urn = replicate_platinum(&ctx).await?;
        let revision = coco::Revision::Branch {
            name: "dev".to_string(),
            peer_id: None,
        };
        let path = "here-we-are-on-a-dev-branch.lol";

        let query = super::BlobQuery {
            path: path.to_string(),
            peer_id: None,
            revision: Some(revision.clone()),
            highlight: Some(false),
        };

        // Get ASCII blob.
        let res = request()
            .method("GET")
            .path(&format!(
                "/blob/{}?{}",
                urn,
                serde_qs::to_string(&query).unwrap()
            ))
            .reply(&api)
            .await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::blob(&mut browser, Some(revision), path, None)
            })
            .await?;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
        });

        Ok(())
    }

    #[tokio::test]
    async fn branches() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());
        let urn = replicate_platinum(&ctx).await?;

        let res = request()
            .method("GET")
            .path(&format!("/branches/{}", urn))
            .reply(&api)
            .await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |browser| coco::branches(browser, None))
            .await?;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(have, json!(["dev", "master"]));
        });

        Ok(())
    }

    #[tokio::test]
    #[allow(clippy::indexing_slicing)]
    async fn commit() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let urn = replicate_platinum(&ctx).await?;
        let sha1 = coco::oid::Oid::try_from("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;

        let res = request()
            .method("GET")
            .path(&format!("/commit/{}/{}", urn, sha1))
            .reply(&api)
            .await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::commit_header(&mut browser, sha1)
            })
            .await?;

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
    async fn commits() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let urn = replicate_platinum(&ctx).await?;

        let branch = git::Branch::local("master");
        let res = request()
            .method("GET")
            .path(&format!("/commits/{}?branch={}", urn.clone(), branch.name))
            .reply(&api)
            .await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::commits(&mut browser, branch.clone())
            })
            .await?;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
        });

        Ok(())
    }

    #[tokio::test]
    async fn local_state() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let mut path = env::current_dir()?;
        path.push("../../fixtures/git-platinum");

        let res = request()
            .method("GET")
            .path(&format!("/local-state/{}", path.to_str().unwrap()))
            .reply(&api)
            .await;

        let want = coco::local_state(path.to_str().unwrap()).unwrap();

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
            assert_eq!(
                have,
                json!({
                    "branches": [
                        "dev",
                        "master",
                    ],
                }),
            );
        });

        Ok(())
    }

    #[tokio::test]
    async fn tags() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());

        let urn = replicate_platinum(&ctx).await?;

        let res = request()
            .method("GET")
            .path(&format!("/tags/{}", urn))
            .reply(&api)
            .await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |browser| coco::tags(browser))
            .await?;
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
    async fn tree() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());
        let urn = replicate_platinum(&ctx).await?;

        let prefix = "src";
        let revision = coco::Revision::Branch {
            name: "master".to_string(),
            peer_id: None,
        };
        let query = super::TreeQuery {
            prefix: Some(prefix.to_string()),
            peer_id: None,
            revision: Some(revision.clone()),
        };
        let path = format!("/tree/{}?{}", urn, serde_qs::to_string(&query).unwrap());
        let res = request().method("GET").path(&path).reply(&api).await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::tree(&mut browser, Some(revision), Some(prefix.to_string()))
            })
            .await?;

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

    #[tokio::test]
    async fn tree_dev_branch() -> Result<(), Box<dyn std::error::Error>> {
        // Testing that the endpoint works with URL encoding
        const FRAGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
            .add(b' ')
            .add(b'"')
            .add(b'[')
            .add(b']')
            .add(b'=');

        let tmp_dir = tempfile::tempdir()?;
        let ctx = context::Context::tmp(&tmp_dir).await?;
        let api = super::filters(ctx.clone());
        let urn = replicate_platinum(&ctx).await?;

        let revision = coco::Revision::Branch {
            name: "dev".to_string(),
            peer_id: None,
        };
        let query = super::TreeQuery {
            prefix: None,
            peer_id: None,
            revision: Some(revision.clone()),
        };
        let path = format!(
            "/tree/{}?{}",
            urn,
            percent_encoding::utf8_percent_encode(&serde_qs::to_string(&query).unwrap(), FRAGMENT)
        );
        let res = request().method("GET").path(&path).reply(&api).await;

        let default_branch = ctx.state.find_default_branch(urn).await?;
        let want = ctx
            .state
            .with_browser(default_branch, |mut browser| {
                coco::tree(&mut browser, Some(revision), None)
            })
            .await?;

        http::test::assert_response(&res, StatusCode::OK, |have| {
            assert_eq!(have, json!(want));
        });

        Ok(())
    }

    async fn replicate_platinum(ctx: &context::Context) -> Result<coco::Urn, error::Error> {
        let owner = ctx.state.init_owner("cloudhead").await?;
        let platinum_project = coco::control::replicate_platinum(
            &ctx.state,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )
        .await?;
        Ok(platinum_project.urn())
    }
}
