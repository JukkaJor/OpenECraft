//! Requests to the parts server (`openecraft-parts-server`) and the project server
//! (`openecraft-project-server`) and the decoding of their answers.
//! Shared by the background client of the applications (`openecraft_ui::client`) and
//! the blocking client of the command line; only sending the request differs.
//! [`eplan`] wraps the API of the EPLAN Data Portal so that it can be read like a
//! parts server ([`Connection`]).

mod connection;
pub mod eplan;
mod endpoint;
mod projects;

pub use connection::{Call, Connection, Decoder, PortalKind};
pub use ehttp;
pub use endpoint::{check, decode, decode_found, text, Endpoint};
pub use eplan::EplanEndpoint;
pub use projects::ProjectEndpoint;
