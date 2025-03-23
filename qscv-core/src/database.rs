use std::fmt::Debug;

use crate::{
    arguments::Arguments,
    column::Column,
    connection::Connection,
    row::Row,
    type_info::TypeInfo,
    value::{Value, ValueRef},
};

pub trait Database: 'static + Sized + Send + Debug {
    /// The concrete `Connection` implementation for this database.
    type Connection: Connection<Database = Self>;

    /// The concrete `Row` implementation for this database.
    type Row: Row<Database = Self>;

    /// The concrete `QueryResult` implementation for this database.
    type QueryResult: 'static + Sized + Send + Sync + Default + Extend<Self::QueryResult>;

    /// The concrete `Column` implementation for this database.
    type Column: Column<Database = Self>;

    /// The concrete `TypeInfo` implementation for this database.
    type TypeInfo: TypeInfo;

    /// The concrete type used to hold an owned copy of the not-yet-decoded value that was
    /// received from the database.
    type Value: Value<Database = Self> + 'static;
    /// The concrete type used to hold a reference to the not-yet-decoded value that has just been
    /// received from the database.
    type ValueRef<'r>: ValueRef<'r, Database = Self>;

    /// The concrete `Arguments` implementation for this database.
    type Arguments<'q>: Arguments<'q, Database = Self>;
    /// The concrete type used as a buffer for arguments while encoding.
    type ArgumentBuffer<'q>;

    /// The display name for this database driver.
    const NAME: &'static str;

    /// The schemes for database URLs that should match this driver.
    const URL_SCHEMES: &'static [&'static str];
}

/// A [`Database`] that maintains a client-side cache of prepared statements.
pub trait HasStatementCache {}

