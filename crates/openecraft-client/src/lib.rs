//! Requests to the parts server (`openecraft-parts-server`) and the project server
//! (`openecraft-project-server`) and the decoding of their answers.
//! Shared by the background client of the applications (`openecraft_ui::client`) and
//! the blocking client of the command line; only sending the request differs.

mod endpoint;
mod projects;

pub use ehttp;
pub use endpoint::{check, decode, decode_found, text, Endpoint};
pub use projects::ProjectEndpoint;
