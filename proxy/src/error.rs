//! Domain errors returned by the API.

use librad::meta::common::url;
use radicle_registry_client::{DispatchError, Error as ProtocolError};
use radicle_surf as surf;
use radicle_surf::git::git2;
use std::time::SystemTimeError;

/// Project problems.
#[derive(Debug)]
pub enum ProjectValidation {
    /// Project names (String32) can only be 32 characters.
    NameTooLong(String),
    /// Project names (String32) can only be 32 characters.
    DomainTooLong(String),
}

/// All error variants the API will return.
#[derive(Debug)]
pub enum Error {
    /// FileSystem errors from interacting with code in repository.
    FS(radicle_surf::file_system::error::Error),
    /// Originated from `radicle_surf`.
    Git(surf::git::error::Error),
    /// Originated from `radicle_surf::git::git2`.
    Git2(git2::Error),
    /// Integer conversion failed.
    IntConversion(std::num::TryFromIntError),
    /// Originated from `librad`.
    Librad(librad::git::Error),
    /// Parse error for `librad::project::ProjectId`.
    LibradParse(librad::project::projectid::ParseError),
    /// Project error from `librad`.
    LibradProject(librad::project::Error),
    /// Common I/O errors.
    Io(std::io::Error),
    /// Url parse error.
    Url(url::ParseError),
    /// Project name validation.
    ProjectValidation(ProjectValidation),
    /// Issues with the Radicle protocol.
    Protocol(ProtocolError),
    /// Issues with the Radicle runtime.
    Runtime(DispatchError),
    /// Errors from handling time.
    Time(SystemTimeError),
}

impl From<radicle_surf::file_system::error::Error> for Error {
    fn from(fs_error: radicle_surf::file_system::error::Error) -> Self {
        Self::FS(fs_error)
    }
}

impl From<surf::git::error::Error> for Error {
    fn from(surf_error: surf::git::error::Error) -> Self {
        Self::Git(surf_error)
    }
}

impl From<git2::Error> for Error {
    fn from(git2_error: git2::Error) -> Self {
        Self::Git2(git2_error)
    }
}

impl From<librad::git::Error> for Error {
    fn from(librad_error: librad::git::Error) -> Self {
        Self::Librad(librad_error)
    }
}

impl From<librad::project::Error> for Error {
    fn from(project_error: librad::project::Error) -> Self {
        Self::LibradProject(project_error)
    }
}

impl From<librad::project::projectid::ParseError> for Error {
    fn from(parse_error: librad::project::projectid::ParseError) -> Self {
        Self::LibradParse(parse_error)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(int_error: std::num::TryFromIntError) -> Self {
        Self::IntConversion(int_error)
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Self::Io(io_error)
    }
}

impl From<url::ParseError> for Error {
    fn from(url_error: url::ParseError) -> Self {
        Self::Url(url_error)
    }
}

impl From<ProtocolError> for Error {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<DispatchError> for Error {
    fn from(error: DispatchError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ProjectValidation> for Error {
    fn from(error: ProjectValidation) -> Self {
        Self::ProjectValidation(error)
    }
}

impl From<SystemTimeError> for Error {
    fn from(error: SystemTimeError) -> Self {
        Self::Time(error)
    }
}
