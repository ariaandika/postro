use crate::{database::Database, type_info::TypeInfo};


/// Indicates that a SQL type is supported for a database.
pub trait Type<DB: Database> {
    /// Returns the canonical SQL type for this Rust type.
    ///
    /// When binding arguments, this is used to tell the database what is about to be sent; which,
    /// the database then uses to guide query plans. This can be overridden by `Encode::produces`.
    ///
    /// A map of SQL types to Rust types is populated with this and used
    /// to determine the type that is returned from the anonymous struct type from `query!`.
    fn type_info() -> DB::TypeInfo;

    /// Determines if this Rust type is compatible with the given SQL type.
    ///
    /// When decoding values from a row, this method is checked to determine if we should continue
    /// or raise a runtime type mismatch error.
    ///
    /// When binding arguments with `query!` or `query_as!`, this method is consulted to determine
    /// if the Rust type is acceptable.
    ///
    /// Defaults to checking [`TypeInfo::type_compatible()`].
    fn compatible(ty: &DB::TypeInfo) -> bool {
        Self::type_info().type_compatible(ty)
    }
}

// for references, the underlying SQL type is identical
impl<T: ?Sized + Type<DB>, DB: Database> Type<DB> for &'_ T {
    fn type_info() -> DB::TypeInfo {
        <T as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <T as Type<DB>>::compatible(ty)
    }
}

// for optionals, the underlying SQL type is identical
impl<T: Type<DB>, DB: Database> Type<DB> for Option<T> {
    fn type_info() -> DB::TypeInfo {
        <T as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        ty.is_null() || <T as Type<DB>>::compatible(ty)
    }
}

