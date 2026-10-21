//! Requests to the parts server (`openecraft-server`) and the decoding of its answers.
//! Shared by the background client of the applications (`openecraft_ui::client`) and
//! the blocking client of the command line; only sending the request differs.

mod endpoint;

pub use ehttp;
pub use endpoint::{check, decode, decode_found, text, Endpoint};
