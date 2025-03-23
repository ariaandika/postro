use std::fmt::Debug;

use crate::type_info::TypeInfo;

pub trait Database: 'static + Sized + Send + Debug {

    /// The concrete `TypeInfo` implementation for this database.
    type TypeInfo: TypeInfo;

    /// The display name for this database driver.
    const NAME: &'static str;

    /// The schemes for database URLs that should match this driver.
    const URL_SCHEMES: &'static [&'static str];
}

/// A [`Database`] that maintains a client-side cache of prepared statements.
pub trait HasStatementCache {}

