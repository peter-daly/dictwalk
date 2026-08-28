//! In-place mutation support shared by `set` and `unset`.

mod shared;
pub(crate) use shared::*;

mod set;
pub(crate) use set::*;

mod unset;
pub(crate) use unset::*;
