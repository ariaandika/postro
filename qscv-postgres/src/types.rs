
pub type Oid = i32;

#[derive(Debug)]
pub enum PgType {
    Bool,
    Int4,
    Int8,
    Text,
    Date,
}

impl PgType {
    pub fn from_oid(oid: Oid) -> Option<PgType> {
        Some(match oid {
            20 => Self::Int8,
            23 => Self::Int4,
            25 => Self::Text,
            _ => return None
        })
    }
    pub fn oid(&self) -> Oid {
        match self {
            Self::Bool => 16,
            Self::Int8 => 20,
            Self::Int4 => 23,
            Self::Text => 25,
            Self::Date => 1082,
        }
    }
}

pub trait AsPgType {
    const PG_TYPE: PgType;
}

impl AsPgType for bool {
    const PG_TYPE: PgType = PgType::Bool;
}

impl AsPgType for i32 {
    const PG_TYPE: PgType = PgType::Int4;
}

impl AsPgType for str {
    const PG_TYPE: PgType = PgType::Text;
}

impl AsPgType for String {
    const PG_TYPE: PgType = PgType::Text;
}

