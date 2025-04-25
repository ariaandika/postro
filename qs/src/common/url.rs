use std::fmt;

use super::ByteStr;

#[derive(Debug)]
pub struct Url {
    #[allow(unused)] // sokay
    pub scheme: ByteStr,
    pub user: ByteStr,
    pub pass: ByteStr,
    pub host: ByteStr,
    pub port: u16,
    pub dbname: ByteStr,
}

impl Url {
    pub fn parse(url: impl Into<ByteStr>) -> Result<Self, ParseError> {
        let url: ByteStr = url.into();
        let mut read = url.as_ref();

        macro_rules! eat {
                (@ $delim:literal,$id:tt,$len:literal) => {{
                    let Some(idx) = read.find($delim) else {
                        return Err(ParseError(concat!(stringify!($id), " missing")))
                    };
                    let capture = &read[..idx];
                    read = &read[idx + $len..];
                    url.slice_ref(capture)
                }};
                ($delim:literal,$id:tt) => {
                    eat!(@ $delim,$id,1)
                };
                ($delim:literal,$id:tt,$len:literal) => {
                    eat!(@ $delim,$id,$len)
                };
            }

        let scheme = eat!("://", user, 3);
        let user = eat!(':', password);
        let pass = eat!('@', host);
        let host = eat!(':', port);
        let port = eat!('/', dbname);
        let dbname = url.slice_ref(read);

        // match port.parse() {
        //     Ok(port) => Ok(Self { scheme, user, pass, host, port, dbname, }),
        //     Err(_) => Err(ParseError("invalid port")),
        // }
        port.parse()
            .map_or(Err(ParseError("invalid port")), |port| {
                Ok(Self {
                    scheme,
                    user,
                    pass,
                    host,
                    port,
                    dbname,
                })
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_url() {
        let url = ByteStr::from_static("postgres://user2:passwd@localhost:5432/post");
        let opt = Url::parse(url.clone()).unwrap();
        assert_eq!(opt.scheme,"postgres");
        assert_eq!(opt.user,"user2");
        assert_eq!(opt.pass,"passwd");
        assert_eq!(opt.host,"localhost");
        assert_eq!(opt.port,5432);
        assert_eq!(opt.dbname,"post");
    }

    #[test]
    fn empty_passwd() {
        let url = ByteStr::from_static("postgres://user2:@localhost:5432/post");
        let opt = Url::parse(url.clone()).unwrap();
        assert_eq!(opt.scheme,"postgres");
        assert_eq!(opt.user,"user2");
        assert_eq!(opt.pass,"");
        assert_eq!(opt.host,"localhost");
        assert_eq!(opt.port,5432);
        assert_eq!(opt.dbname,"post");
    }
}

pub struct ParseError(&'static str);

impl std::error::Error for ParseError { }
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse url: {}", self.0)
    }
}
impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

