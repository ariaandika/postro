use bytes::{Buf, Bytes};

/// an unencoded row
#[derive(Debug)]
pub struct RawRow {
    /// expected column length
    col_len: i16,
    /// already read column
    read: i16,
    /// raw buffer
    bytes: Bytes,
}

impl RawRow {
    pub fn new(col_len: i16, bytes: Bytes) -> Self {
        Self { col_len, bytes, read: 0 }
    }
}

impl Iterator for RawRow {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read == self.col_len {
            return None
        }

        // The length of the column value, in bytes (this count does not include itself).
        // Can be zero. As a special case, -1 indicates a NULL column value.
        // No value bytes follow in the NULL case.
        let len = self.bytes.get_i32();

        // The value of the column, in the format indicated by the associated format code.
        // n is the above length.
        let data = match len {
            -1 => Bytes::from_static(b"NULL") ,
            len => self.bytes.split_to(len as _) ,
        };

        self.read += 1;

        Some(data)
    }
}

