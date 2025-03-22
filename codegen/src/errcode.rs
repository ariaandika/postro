use anyhow::{Context, Result};
use std::{collections::BTreeMap, mem, str::CharIndices};

/// errcode section
///
/// example: `Section: Class HV - Foreign Data Wrapper Error (SQL/MED)`
///
/// ```rust,ignore
/// let section = Section {
///     class: ['H','V'],
///     name: "Foreign Data Wrapper Error".into(),
///     note: Some("SQL/MED".into()),
/// };
/// ```
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Section {
    pub class: [char;2],
    pub name: String,
    pub note: Option<String>,
}

impl PartialEq for Section {
    fn eq(&self, other: &Self) -> bool {
        self.class.eq(&other.class)
    }
}

impl PartialOrd for Section {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.class.cmp(&other.class))
    }
}

impl Eq for Section { }

impl Ord for Section {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.class.cmp(&other.class)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ErrCode {
    /// 5 character SQLSTATE conventions
    pub sqlstate: String,
    /// E/W/S
    pub status: char,
    pub errcode_macro_name: String,
    pub spec_name: Option<String>,
}

pub type ErrCodes = BTreeMap<Section,Vec<ErrCode>>;

pub struct ErrCodeParser<'a> {
    source: &'a str,
    current_ch: (usize,char),
    iter: CharIndices<'a>,

    current_sect: (Section,Vec<ErrCode>),
    errcodes: ErrCodes,
}

impl<'a> ErrCodeParser<'a> {
    pub fn new(source: &'a str) -> Result<Self> {
        Ok(Self {
            source,
            current_ch: source.char_indices().next().context("source empty")?,
            iter: source.char_indices(),
            current_sect: <_>::default(),
            errcodes: <_>::default(),
        })
    }

    pub fn parse(mut self) -> Result<ErrCodes> {
        self.skip_wh();

        self.try_word("Section")?;
        self.current_sect = self.collect_post_section()?;
        self.skip_wh();

        loop {
            let lead = self.collect_word();

            if lead == "Section" {
                let section = self.collect_post_section()?;
                let (section,err) = mem::replace(&mut self.current_sect, section);
                self.errcodes.insert(section, err);

            } else {
                let sqlstate = lead.to_owned();
                self.skip_wh();
                let status = self.pop()?.1;
                self.skip_wh();
                let errcode_macro_name = self.collect_word().to_owned();
                let spec_name = match self.skip_line()? {
                    true => None,
                    false => Some(self.collect_word().to_owned()),
                };

                let errcode = ErrCode {
                    sqlstate, status, errcode_macro_name, spec_name,
                };

                self.current_sect.1.push(errcode);
            }

            if self.skip_wh() {
                break
            }
        }

        let (section,codes) = self.current_sect;
        self.errcodes.insert(section, codes);

        Ok(self.errcodes)
    }

    fn try_advance(&mut self) -> Result<(usize, char)> {
        self.current_ch = self.iter.next().context("unexpected EOF")?;
        Ok(self.current_ch)
    }

    fn pop(&mut self) -> Result<(usize, char)> {
        let pop = self.current_ch;
        self.try_advance()?;
        Ok(pop)
    }

    /// return is EOF
    fn skip_wh(&mut self) -> bool {
        loop {
            match self.current_ch {
                (_,c) if c.is_whitespace() => {
                    if self.try_advance().is_err() {
                        return true;
                    }
                }
                (_,'#') => loop {
                    match self.try_advance() {
                        Ok((_,'\n')) => {
                            if self.try_advance().is_err() {
                                return true;
                            }
                            break
                        }
                        Ok(_) => {}
                        Err(_) => return true,
                    }
                }
                _ => return false,
            }
        }
    }

    /// return is newline found instead of word
    fn skip_line(&mut self) -> Result<bool> {
        loop {
            match self.current_ch {
                (_,'\n') => {
                    self.try_advance()?;
                    return Ok(true);
                },
                (_,c) if c.is_whitespace() => {
                    self.try_advance()?;
                }
                _ => return Ok(false),
            }
        }
    }

    fn collect_word(&mut self) -> &str {
        let start = self.current_ch.0;
        loop {
            match self.try_advance() {
                Ok((_,c)) if c.is_alphanumeric() => {},
                Ok((_,'_')) => {},
                Ok((end,_)) => break &self.source[start..end],
                Err(_) => break &self.source[start..],
            }
        }
    }

    fn try_word(&mut self, expect: &str) -> Result<()> {
        let word = self.collect_word().to_owned();
        self.assert(&word, expect, format!("expected `{expect}`"))?;
        Ok(())
    }

    fn collect_post_section(&mut self) -> Result<(Section,Vec<ErrCode>)> {
        let colon = self.pop()?.1;
        self.assert(colon, ':', "expected `:`")?;
        self.skip_wh();
        self.try_word("Class")?;
        self.skip_wh();

        let c1 = self.pop()?.1;
        let c2 = self.pop()?.1;

        self.skip_wh();
        let sub = self.pop()?.1;
        self.assert(sub, '-', "expected `-`")?;
        self.skip_wh();

        let mut name = String::from(self.current_ch.1);
        let mut note = None;

        loop {
            match self.try_advance()? {
                (_,'(') => {
                    let mut n = String::new();
                    loop {
                        match self.try_advance()? {
                            (_, ')') => break,
                            (_, c) => n.push(c),
                        }
                    }
                    note = Some(n);
                    break;
                }
                (_,'\n') => break,
                (_,c) => name.push(c),
            }
        }

        self.try_advance()?;

        Ok((Section { class: [c1,c2], name: name.trim_end().to_owned(), note, },vec![]))
    }

    fn source_left(&self) -> &str {
        self.source
            .get(self.current_ch.0..self.current_ch.0 + 30)
            .unwrap_or(&self.source[self.current_ch.0..])
    }

    fn bail<T>(&self, msg: impl std::fmt::Display) -> Result<T> {
        let this = &self;
        let msg = msg;
        anyhow::bail!("{msg} near {:?}",this.source_left())
    }

    fn assert<T,E>(&self, left: T, right: E, msg: impl std::fmt::Display) -> Result<()> where T: PartialEq<E> {
        if left != right {
            return self.bail(msg)
        }
        Ok(())
    }
}

