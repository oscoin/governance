//! Domain errors returned by the API.

use juniper::{FieldError, IntoFieldError};
use radicle_surf::git::git2;

/// All error variants the API will return.
#[derive(Debug)]
pub enum Error {
    /// Originated from `radicle_surf`.
    Git(radicle_surf::git::GitError),
    /// Originated from `radicle_surf::git::git2`.
    Git2(git2::Error),
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
}

impl From<radicle_surf::git::GitError> for Error {
    fn from(git_error: radicle_surf::git::GitError) -> Self {
        Self::Git(git_error)
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

/// Helper to convert `std::io::Error` to `FieldError`.
fn convert_io_error_to_field_error(error: &std::io::Error) -> FieldError {
    FieldError::new(
        error.to_string(),
        graphql_value!({
            "type": "IO_ERROR",
        }),
    )
}

/// Helper to convert a `radicle_surf` Git error to `FieldError`.
fn convert_git_error_to_field_error(error: &radicle_surf::git::GitError) -> FieldError {
    match &error {
        radicle_surf::git::GitError::EmptyCommitHistory => FieldError::new(
            "Repository has an empty commit history.",
            graphql_value!({
                "type": "GIT_EMPTY_COMMIT_HISTORY"
            }),
        ),
        radicle_surf::git::GitError::BranchDecode => FieldError::new(
            "Unable to decode the given branch.",
            graphql_value!({
                "type": "GIT_BRANCH_DECODE"
            }),
        ),
        radicle_surf::git::GitError::NotBranch => FieldError::new(
            "Not a known branch.",
            graphql_value!({
                "type": "GIT_NOT_BRANCH"
            }),
        ),
        radicle_surf::git::GitError::NotTag => FieldError::new(
            "Not a known tag.",
            graphql_value!({
                "type": "GIT_NOT_TAG"
            }),
        ),
        radicle_surf::git::GitError::Internal(error) => FieldError::new(
            format!("Internal Git error: {:?}", error),
            graphql_value!({
                "type": "GIT_INTERNAL"
            }),
        ),
    }
}

/// Helper to convert a `git2::error::Error` to `FieldError`.
fn convert_git2_error_to_field_error(error: &git2::Error) -> FieldError {
    FieldError::new(
        error.to_string(),
        graphql_value!({
            "type": "GIT2_ERROR"
        }),
    )
}

/// Helper to convert `librad::git::Error` to `FieldError`.
fn convert_librad_git_error_to_field_error(error: &librad::git::Error) -> FieldError {
    match error {
        librad::git::Error::MissingPgpAddr => FieldError::new(
            "Missing PGP address.",
            graphql_value!({
                "type": "LIBRAD_MISSING_PGP_ADDRESS"
            }),
        ),
        librad::git::Error::MissingPgpUserId => FieldError::new(
            "Missing PGP user ID.",
            graphql_value!({
                "type": "LIBRAD_MISSING_PGP_USER_ID"
            }),
        ),
        librad::git::Error::ProjectExists => FieldError::new(
            "Project already exists.",
            graphql_value!({
                "type": "LIBRAD_PROJECT_EXISTS"
            }),
        ),
        librad::git::Error::NoSuchProject => FieldError::new(
            "No such project exists.",
            graphql_value!({
                "type": "LIBRAD_NO_SUCH_PROJECT"
            }),
        ),
        librad::git::Error::Libgit(git2_error) => convert_git2_error_to_field_error(git2_error),
        librad::git::Error::Io(io_error) => convert_io_error_to_field_error(io_error),
        librad::git::Error::Serde(json_error) => FieldError::new(
            json_error.to_string(),
            graphql_value!({
                "type": "LIBRAD_JSON_ERROR"
            }),
        ),
        librad::git::Error::Pgp(pgp_error) => FieldError::new(
            pgp_error.to_string(),
            graphql_value!({
                "type": "LIBRAD_PGP_ERROR"
            }),
        ),
    }
}

/// Helper to convert `librad::project::projectid::ParseError` to `FieldError`.
fn convert_librad_parse_error_to_field_error(
    error: &librad::project::projectid::ParseError,
) -> FieldError {
    match error {
        librad::project::projectid::ParseError::Git(parse_error) => match parse_error {
            librad::git::projectid::ParseError::InvalidBackend(error) => FieldError::new(
                error.to_string(),
                graphql_value!({
                    "type": "LIBRAD_PARSE_INVALID_BACKEND"
                }),
            ),
            librad::git::projectid::ParseError::InvalidFormat(error) => FieldError::new(
                error.to_string(),
                graphql_value!({
                    "type": "LIBRAD_PARSE_INVALID_FORMAT"
                }),
            ),
            librad::git::projectid::ParseError::InvalidOid(_, git2_error) => {
                convert_git2_error_to_field_error(git2_error)
            }
        },
    }
}

/// Helper to convert `url::ParseError` to `FieldError`.
fn convert_url_parse_error_to_field_error(error: url::ParseError) -> FieldError {
    match error {
        url::ParseError::EmptyHost => FieldError::new(
            "Empty host.",
            graphql_value!({ "type": "URL_PARSE_EMPTY_HOST" }),
        ),
        url::ParseError::IdnaError => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_IDNA" }),
        ),
        url::ParseError::InvalidPort => FieldError::new(
            "Invalid port.",
            graphql_value!({ "type": "URL_PARSE_INVALID_PORT" }),
        ),
        url::ParseError::InvalidIpv4Address => FieldError::new(
            "Invalid IPv4 address.",
            graphql_value!({ "type": "URL_PARSE_INVALID_IPV4" }),
        ),
        url::ParseError::InvalidIpv6Address => FieldError::new(
            "Invalid IPv6 address.",
            graphql_value!({ "type": "URL_PARSE_INVALID_IPV6" }),
        ),
        url::ParseError::InvalidDomainCharacter => FieldError::new(
            "Invalid domain character.",
            graphql_value!({ "type": "URL_PARSE_INVALID_DOMAIN_CHAR" }),
        ),
        url::ParseError::RelativeUrlWithoutBase => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_RELATIVE_WITHOUT_BASE" }),
        ),
        url::ParseError::RelativeUrlWithCannotBeABaseBase => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_RELATIVE_CANNOT_BE_BASE" }),
        ),
        url::ParseError::SetHostOnCannotBeABaseUrl => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_SET_HOST_CANNOT_BE_BASE" }),
        ),
        url::ParseError::Overflow => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_OVERFLOW" }),
        ),
        url::ParseError::__FutureProof => FieldError::new(
            error.to_string(),
            graphql_value!({ "type": "URL_PARSE_FUTURE_PROOF" }),
        ),
    }
}

impl IntoFieldError for Error {
    fn into_field_error(self) -> FieldError {
        match self {
            Self::Git(git_error) => convert_git_error_to_field_error(&git_error),
            Self::Git2(git2_error) => convert_git2_error_to_field_error(&git2_error),
            Self::Io(io_error) => convert_io_error_to_field_error(&io_error),
            Self::Librad(librad_error) => convert_librad_git_error_to_field_error(&librad_error),
            Self::LibradParse(parse_error) => {
                convert_librad_parse_error_to_field_error(&parse_error)
            }
            Self::LibradProject(project_error) => match project_error {
                librad::project::Error::Git(librad_error) => {
                    convert_librad_git_error_to_field_error(&librad_error)
                }
            },
            Self::Url(url_error) => convert_url_parse_error_to_field_error(url_error),
        }
    }
}
