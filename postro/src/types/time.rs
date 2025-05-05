use time::{
    PrimitiveDateTime, UtcDateTime,
    format_description::{BorrowedFormatItem as I, Component as C, modifier},
};

use crate::{
    Decode, DecodeError, Encode,
    common::ByteStr,
    encode::Encoded,
    postgres::{Oid, PgType},
    row::Column,
};

impl PgType for PrimitiveDateTime {
    /// date and time
    const OID: Oid = 1114;
}

impl PgType for UtcDateTime {
    /// date and time
    const OID: Oid = 1114;
}

impl Decode for PrimitiveDateTime {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        if column.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        PrimitiveDateTime::parse(&ByteStr::from_utf8(column.into_value())?, &DESCRIPTION)
            .map_err(<_>::into)
    }
}

impl Decode for UtcDateTime {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        if column.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        UtcDateTime::parse(&ByteStr::from_utf8(column.into_value())?, &DESCRIPTION)
            .map_err(<_>::into)
    }
}

impl Encode<'static> for PrimitiveDateTime {
    fn encode(self) -> Encoded<'static> {
        Encoded::owned(
            self.format(&DESCRIPTION)
                .expect("format is statically known"),
            Self::OID,
        )
    }
}

impl Encode<'static> for UtcDateTime {
    fn encode(self) -> Encoded<'static> {
        Encoded::owned(
            self.format(&DESCRIPTION)
                .expect("format is statically known"),
            Self::OID,
        )
    }
}

const DESCRIPTION: &[I<'_>] = &[
    I::Component {
        0: C::Year(modifier::Year::default()),
    },
    I::Literal { 0: b"-" },
    I::Component {
        0: C::Month(modifier::Month::default()),
    },
    I::Literal { 0: b"-" },
    I::Component {
        0: C::Day(modifier::Day::default()),
    },
    I::Literal { 0: b" " },
    I::Component {
        0: C::Hour(modifier::Hour::default()),
    },
    I::Literal { 0: b":" },
    I::Component {
        0: C::Minute(modifier::Minute::default()),
    },
    I::Literal { 0: b":" },
    I::Component {
        0: C::Second(modifier::Second::default()),
    },
    I::Literal { 0: b"." },
    I::Component {
        0: C::Subsecond(modifier::Subsecond::default()),
    },
];

