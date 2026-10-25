//! Requests to the parts server (`openecraft-parts-server`), the project server
//! (`openecraft-project-server`) and Autodesk Vault ([`vault`]) and the decoding of
//! their answers. Shared by the background clients of the applications and the
//! blocking clients of the command line; only sending the request differs.

mod endpoint;
mod projects;
pub mod vault;

pub use ehttp;
pub use endpoint::{check, decode, decode_found, text, Endpoint};
pub use projects::ProjectEndpoint;
