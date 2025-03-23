use crate::{error::Error, row::Row};

/// A record that can be built from a row returned by the database.

pub trait FromRow<'r, R: Row>: Sized {
    fn from_row(row: &'r R) -> Result<Self, Error>;
}

impl<'r, R> FromRow<'r, R> for ()
where
    R: Row,
{
    #[inline]
    fn from_row(_: &'r R) -> Result<Self, Error> {
        Ok(())
    }
}

// implement FromRow for tuples of types that implement Decode
// up to tuples of 16 values

macro_rules! impl_from_row_for_tuple {
    ($( ($idx:tt) -> $T:ident );+;) => {
        impl<'r, R, $($T,)+> FromRow<'r, R> for ($($T,)+)
        where
            R: Row,
            usize: crate::column::ColumnIndex<R>,
            $($T: crate::decode::Decode<'r, R::Database> + crate::types::Type<R::Database>,)+
        {
            #[inline]
            fn from_row(row: &'r R) -> Result<Self, Error> {
                Ok(($(row.try_get($idx as usize)?,)+))
            }
        }
    };
}

impl_from_row_for_tuple!(
    (0) -> T1;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
    (11) -> T12;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
    (11) -> T12;
    (12) -> T13;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
    (11) -> T12;
    (12) -> T13;
    (13) -> T14;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
    (11) -> T12;
    (12) -> T13;
    (13) -> T14;
    (14) -> T15;
);

impl_from_row_for_tuple!(
    (0) -> T1;
    (1) -> T2;
    (2) -> T3;
    (3) -> T4;
    (4) -> T5;
    (5) -> T6;
    (6) -> T7;
    (7) -> T8;
    (8) -> T9;
    (9) -> T10;
    (10) -> T11;
    (11) -> T12;
    (12) -> T13;
    (13) -> T14;
    (14) -> T15;
    (15) -> T16;
);

