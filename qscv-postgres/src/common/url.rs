use super::bytestr::ByteStr;

#[derive(Debug)]
pub struct Url {
    pub scheme: ByteStr,
    pub user: ByteStr,
    pub pass: ByteStr,
    pub host: ByteStr,
    pub port: u16,
    pub dbname: ByteStr,
}

impl Url {
    pub fn parse(url: ByteStr) -> Result<Self, ParseError> {
        let mut read = url.as_ref();

        macro_rules! eat {
            (@ $delim:literal,$id:tt,$len:literal) => {{
                let Some(idx) = read.find($delim) else {
                    return Err(ParseError::new(format!("required {}", stringify!($id))))
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

        let scheme = eat!("://",user,3);
        let user = eat!(':',password);
        let pass = eat!('@',host);
        let host = eat!(':',port);
        let port = eat!('/',dbname);
        let dbname = url.slice_ref(read);

        let Ok(port) = port.parse() else {
            return Err(ParseError::new("failed to parse port".to_owned()))
        };

        Ok(Self { scheme, user, pass, host, port, dbname, })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    message: String
}

impl ParseError {
    pub fn new(message: String) -> Self {
        Self { message }
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

