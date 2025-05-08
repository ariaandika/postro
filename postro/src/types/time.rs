use time::{
    Duration, PrimitiveDateTime, UtcDateTime,
    format_description::{BorrowedFormatItem as I, Component as C, modifier},
};

use crate::{
    Decode, DecodeError, Encode,
    encode::Encoded,
    postgres::{Oid, PgType},
    row::Column,
};

impl PgType for PrimitiveDateTime {
    /// date and time
    const OID: Oid = 1114;
}

impl PgType for UtcDateTime {
    /// date and time with timezone
    const OID: Oid = 1184;
}

const PRIMITIVE_PG_EPOCH: PrimitiveDateTime = {
    // source: `from_julian_day` docs
    let date = match time::Date::from_julian_day(2_451_545) {
        Ok(ok) => ok,
        Err(_) => panic!("for fuck sake"),
    };
    PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
};

const UTC_PG_EPOCH: UtcDateTime = {
    // source: `from_julian_day` docs
    let date = match time::Date::from_julian_day(2_451_545) {
        Ok(ok) => ok,
        Err(_) => panic!("for fuck sake"),
    };
    UtcDateTime::new(date, time::Time::MIDNIGHT)
};

impl Decode for PrimitiveDateTime {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        if column.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        let value = column.try_into_value()?;
        assert_eq!(
            value.len(),
            size_of::<i64>(),
            "postgres did not return `i64`"
        );
        Ok(
            PRIMITIVE_PG_EPOCH.saturating_add(Duration::microseconds(i64::from_be_bytes(
                value[..].try_into().unwrap(),
            ))),
        )
    }
}

impl Decode for UtcDateTime {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        if column.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        let value = column.try_into_value()?;
        assert_eq!(
            value.len(),
            size_of::<i64>(),
            "postgres did not return `i64`"
        );
        Ok(
            UTC_PG_EPOCH.saturating_add(Duration::microseconds(i64::from_be_bytes(
                value[..].try_into().unwrap(),
            ))),
        )
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
    I::Component(C::Year(modifier::Year::default())),
    I::Literal(b"-"),
    I::Component(C::Month(modifier::Month::default())),
    I::Literal(b"-"),
    I::Component(C::Day(modifier::Day::default())),
    I::Literal(b" "),
    I::Component(C::Hour(modifier::Hour::default())),
    I::Literal(b":"),
    I::Component(C::Minute(modifier::Minute::default())),
    I::Literal(b":"),
    I::Component(C::Second(modifier::Second::default())),
    I::Literal(b"."),
    I::Component(C::Subsecond(modifier::Subsecond::default())),
];

