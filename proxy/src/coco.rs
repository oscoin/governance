//! Abstractions and utilities for git interactions through the API.

pub use librad::uri::rad_urn::ParseError;
pub use librad::uri::RadUrn as Urn;

pub use radicle_surf::diff::{Diff, FileDiff};
pub use radicle_surf::vcs::git::Stats;

pub mod config;
pub mod control;
mod peer;
pub use peer::{verify_user, Api, User, UserRevisions};

mod source;
pub use source::{
    blob, branches, commit, commit_header, commits, local_state, tags, tree, Blob, BlobContent,
    Branch, Commit, CommitHeader, Info, ObjectType, Person, Revision, Tag, Tree, TreeEntry,
};
