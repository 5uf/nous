//! `nous-core` — shared types and contracts for the Nous workspace.
//!
//! Every other crate in the workspace depends on this crate.  Nothing here
//! does I/O; the types are pure data + logic.

mod error;
mod id;
mod meta;
mod object;
mod rights;

pub use error::{Error, Result};
pub use id::{HashAlgo, ObjectId};
pub use meta::Meta;
pub use object::{Commit, Manifest, Object, ObjectKind, Tree, TreeEntry};
pub use rights::Right;
