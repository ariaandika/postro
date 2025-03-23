use anyhow::{bail, Context, Result};
use std::{collections::HashMap, mem, str::CharIndices};

pub trait Collector {
    type Output;
    fn add_map(&mut self, map: Map) -> Result<()>;
    fn finish(self) -> Self::Output;
}

enum ParseState {
    Root { comma: bool },
    Object { comma: bool },
}

pub type Map = HashMap<String,String>;

pub struct Parser<'a,C> {
    source: &'a str,
    current_ch: (usize,char),
    iter: CharIndices<'a>,
    state: ParseState,

    current_type: HashMap<String,String>,
    collector: C,
}

impl<'a,C> Parser<'a,C> {
    pub fn new(source: &'a str, collector: C) -> Result<Self> {
        Ok(Self {
            source,
            current_ch: source.char_indices().next().context("source empty")?,
            iter: source.char_indices(),
            state: ParseState::Root { comma: true },
            current_type: <_>::default(),
            collector,
        })
    }
}

impl<C> Parser<'_,C>
where
    C: Collector,
{
    pub fn parse(mut self) -> Result<C::Output> {
        self.skip_wh();
        if self.current_ch.1 != '[' {
            bail!("expected `[`")
        }
        self.try_next()?;
        self.skip_wh();

        loop {
            let current = self.current_ch;

            match self.state {
                ParseState::Root { comma } => {
                    match (comma,current.1) {
                        (true,'{') => self.state = ParseState::Object { comma: true },
                        (false,'{') => return self.bail("expected `{`"),
                        (true,',') => bail!("invalid double `,`"),
                        (false,',') => self.state = ParseState::Root { comma: true },
                        (_,']') => break,
                        (_,ch) => return self.bail(format!("unexpected `{ch}`"))
                    }
                    self.try_next()?;
                    self.skip_wh();
                }

                ParseState::Object { comma } => {
                    match (comma,current.1) {
                        (true,ch) if ch.is_alphabetic() => {
                            let ident = self.collect_ident().to_owned();
                            self.skip_wh();
                            self.collect_fat_arrow()?;
                            self.skip_wh();
                            let value = self.collect_litstr()?.to_owned();
                            self.current_type.insert(ident, value);
                            self.state = ParseState::Object { comma: false };
                            self.skip_wh();
                            continue;
                        }
                        (false,ch) if ch.is_alphabetic() => return self.bail("expected `,`"),
                        (false,',') => self.state = ParseState::Object { comma: true },
                        (_,'}') => {
                            self.collector.add_map(mem::take(&mut self.current_type))?;
                            self.state = ParseState::Root { comma: false };
                        }
                        (_,ch) => bail!("unexpected `{ch}`")
                    }
                    self.try_next()?;
                    self.skip_wh();
                }
            }
        }


        if !self.current_type.is_empty() {
            panic!("leftover type")
        }

        Ok(self.collector.finish())
    }

    fn source_left(&self) -> &str {
        self.source
            .get(self.current_ch.0..self.current_ch.0 + 30)
            .unwrap_or(&self.source[self.current_ch.0..])
    }

    fn error(&self, msg: impl std::fmt::Display) -> anyhow::Error {
        anyhow::anyhow!("{msg} near {:?}",self.source_left())
    }

    fn bail<T>(&self, msg: impl std::fmt::Display) -> Result<T> {
        Err(self.error(msg))
    }

    fn try_next(&mut self) -> Result<(usize, char)> {
        self.current_ch = self.iter.next().context("unexpected EOF")?;
        Ok(self.current_ch)
    }

    fn skip_wh(&mut self) {
        loop {
            match self.current_ch {
                (_,c) if c.is_whitespace() => if self.try_next().is_err() {
                    return;
                },
                (_,'#') => loop {
                    match self.try_next() {
                        Ok((_,'\n')) => { let _ = self.try_next(); break }
                        Ok(_) => {}
                        Err(_) => return,
                    }
                },
                _ => return,
            }
        }
    }

    fn collect_ident(&mut self) -> &str {
        let start = self.current_ch.0;
        loop {
            match self.try_next() {
                Ok((_,c)) if c.is_alphabetic() => {},
                Ok((_,'_')) => {},
                Ok((end,_)) => break &self.source[start..end],
                Err(_) => break &self.source[start..],
            }
        }
    }

    fn collect_fat_arrow(&mut self) -> anyhow::Result<()> {
        if self.current_ch.1 != '=' {
            return self.bail("expected `=`")
        }
        if self.try_next()?.1 != '>' {
            return self.bail("expected `>`")
        }
        self.try_next()?;
        Ok(())
    }

    fn collect_litstr(&mut self) -> anyhow::Result<String> {
        let mut output = String::new();
        if self.current_ch.1 != '\'' {
            return self.bail("expected `'`");
        }
        loop {
            match self.try_next()? {
                (_,'\'') => break,
                (_,'\\') => { output.push(self.try_next()?.1); },
                (_,c) => output.push(c),
            }
        };
        self.try_next()?;
        Ok(output)
    }
}

