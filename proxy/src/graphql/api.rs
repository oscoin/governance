use std::sync::Arc;
use tokio::sync::RwLock;
use warp::filters;
use warp::http;
use warp::{Filter, Rejection, Reply};

use crate::registry;

use super::schema;

/// `GraphQL` API routes.
#[must_use]
pub fn routes(
    librad_paths: Arc<RwLock<librad::paths::Paths>>,
    registry: Arc<RwLock<registry::Registry>>,
    store: Arc<RwLock<kv::Store>>,
    enable_control: bool,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let context = schema::Context::new(librad_paths, registry, store);
    let state = warp::any().map(move || context.clone());
    let graphql_filter = make_graphql_filter(schema::create(), state.clone().boxed());
    let control_filter = make_graphql_filter(schema::create_control(), state.boxed());

    warp::path("control")
        .map(move || enable_control)
        .and_then(|enable_control| async move {
            if enable_control {
                Ok(())
            } else {
                Err(warp::reject::not_found())
            }
        })
        .untuple_one()
        .and(control_filter)
        .or(warp::path("graphql").and(graphql_filter))
        .with(warp::log("proxy::graphql"))
}

/// Filter for the graphql endpoint.
fn make_graphql_filter<Context, Mutation, Query>(
    schema: juniper::RootNode<'static, Query, Mutation>,
    context_extractor: filters::BoxedFilter<(Context,)>,
) -> impl Filter<Extract = (http::Response<Vec<u8>>,), Error = Rejection> + Clone
where
    Context: Clone + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Query: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
{
    let schema = Arc::new(schema);

    warp::post()
        .map(move || Arc::<juniper::RootNode<'static, Query, Mutation>>::clone(&schema))
        .and(context_extractor)
        .and(warp::body::json())
        .and_then(handle_request)
}

/// Executes the request and crafts the serialised response.
async fn handle_request<Context, Mutation, Query>(
    schema: Arc<juniper::RootNode<'static, Query, Mutation>>,
    context: Context,
    request: juniper::http::GraphQLRequest,
) -> Result<http::Response<Vec<u8>>, std::convert::Infallible>
where
    Context: Clone + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Query: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
{
    match serde_json::to_vec(&request.execute(&schema, &context)) {
        Ok(body) => Ok(http::Response::builder()
            .header("content-type", "application/json; charset=utf-8")
            .body(body)
            .expect("unable to build response")),
        Err(_) => Ok(http::Response::builder()
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("unable to build response")),
    }
}
