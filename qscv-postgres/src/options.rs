use crate::{
    common::bytestr::ByteStr,
    error::{Error, Result},
};

/// postgres connection options
#[derive(Debug)]
pub struct PgOptions {
    url: ByteStr,
    pub(crate) user: ByteStr,
    pub(crate) pass: ByteStr,
    pub(crate) host: ByteStr,
    pub(crate) port: u16,
    pub(crate) dbname: ByteStr,
}

impl PgOptions {
    pub fn parse(url: &str) -> Result<PgOptions> {
        Self::parse_inner(ByteStr::copy_from_str(url))
    }

    #[allow(dead_code)]
    pub fn parse_static(url: &'static str) -> Result<PgOptions> {
        Self::parse_inner(ByteStr::from_static(url))
    }

    fn parse_inner(url: ByteStr) -> Result<Self> {
        macro_rules! parse_err {
            ($($tt:tt)*) => { return Err(Error::Configuration(format!($($tt)*))) };
        }

        let mut read = url.as_ref();

        {
            let Some(scheme_idx) = read.find("://") else {
                parse_err!("failed to parse url")
            };
            let scheme = &read[..scheme_idx];

            if !matches!(scheme,"postgres"|"postgresql") {
                parse_err!("scheme expected to be `postgres`")
            }

            read = &read[scheme_idx + "://".len()..];
        }

        macro_rules! eat {
            ($delim:literal,$id:tt) => {{
                let Some(idx) = read.find($delim) else {
                    parse_err!("required {}", stringify!($id))
                };
                let capture = &read[..idx];
                read = &read[idx + 1..];
                url.slice_ref(capture)
            }};
        }

        let user = eat!(':',password);
        let pass = eat!('@',host);
        let host = eat!(':',port);
        let Ok(port) = eat!('/',dbname).parse() else {
            parse_err!("failed to parse port")
        };
        let dbname = url.slice_ref(read);

        Ok(Self {
            url,
            user,
            pass,
            host,
            port,
            dbname,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_url() {
        let url = "postgres://user2:passwd@localhost:5432/post";
        let opt = PgOptions::parse_static(url).unwrap();
        assert_eq!(opt.url,url);
        assert_eq!(opt.user,"user2");
        assert_eq!(opt.pass,"passwd");
        assert_eq!(opt.host,"localhost");
        assert_eq!(opt.port,5432);
        assert_eq!(opt.dbname,"post");
    }

    #[test]
    fn empty_passwd() {
        let url = "postgres://user2:@localhost:5432/post";
        let opt = PgOptions::parse_static(url).unwrap();
        assert_eq!(opt.url,url);
        assert_eq!(opt.user,"user2");
        assert_eq!(opt.pass,"");
        assert_eq!(opt.host,"localhost");
        assert_eq!(opt.port,5432);
        assert_eq!(opt.dbname,"post");
    }
}

