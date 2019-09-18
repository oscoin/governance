//! Proxy to serve a specialised HTTP to the oscoin MVP.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::unseparated_literal_suffix)]

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

#[macro_use]
extern crate juniper;

mod schema;
// mod server_actix;
mod server_warp;
mod source;

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    pretty_env_logger::init();

    let osc = oscoin_client::Client::new_from_file().unwrap();
    let source = source::Mixed::new(source::Ledger::new(osc), source::Local::new());

    info!("Creating GraphQL schema and context");
    let schema = schema::create();
    let context = schema::Context::new(source);

    info!("Starting HTTP server");
    server_warp::run(schema, context);
}
