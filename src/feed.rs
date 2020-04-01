use std::borrow::Cow;
use std::str;

use lazy_static::lazy_static;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::BytesStart;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader as XmlReader;
use regex::Regex;

/*
trait PullFn {
    type Output;
    fn pull<B>(
        &mut self,
        reader: &mut XmlReader<B>,
        event: XmlEvent,
    ) -> quick_xml::Result<Option<Self::Output>>
    where
        B: std::io::BufRead;
}

        enum InnerFn {
            SkipElement(SkipElement),
            ParseText(ParseText),
        }

struct ParseRss {
    output: RSS,
    reading_rss_1_0_head: bool,
    inner_fn: Option<InnerFn>,
}

impl PullFn for ParseRss {
    type Output = RSS;
    fn pull<B>(
        &mut self,
        reader: &mut XmlReader<B>,
        event: XmlEvent,
    ) -> quick_xml::Result<Option<Self::Output>>
    where
        B: std::io::BufRead,
    {
        if let Some(f) = &mut self.inner_fn {
           return match f {
                InnerFn::SkipElement(f) => f.pull(reader, event),
            };
        }
        match event {
            XmlEvent::Empty(ref e) => {
                if reader.decode(e.local_name())? == "link" {
                    match parse_atom_link(reader, e.attributes())? {
                        Some(AtomLink::Alternate(link)) => self.output.link = link,
                        Some(AtomLink::Source(link)) => self.output.source = Some(link),
                        _ => {}
                    }
                }
            }
            XmlEvent::Start(ref e) => {
                match reader.decode(e.local_name())? {
                    "channel" => {
                        // RSS 0.9 1.0
                        self.reading_rss_1_0_head = true;
                    }
                    "title" => {
                        if let Some(title) = try_parse_text(reader)? {
                            self.output.title = title;
                        }
                    }
                    "link" => {
                        if let Some(link) = try_parse_text(reader)? {
                            // RSS
                            self.output.link = link;
                        } else {
                            // ATOM
                            match parse_atom_link(reader, e.attributes())? {
                                Some(AtomLink::Alternate(link)) => self.output.link = link,
                                Some(AtomLink::Source(link)) => self.output.source = Some(link),
                                _ => {}
                            }
                        }
                    }
                    "item" | "entry" => {
                        self.output.items.push(Item::from_xml(reader, e)?);
                    }
                    // skip this element
                    _ => (),
                }
            }
            XmlEvent::End(_) if self.reading_rss_1_0_head => {
                // reader.decode(e.local_name())? == "channel";
                self.reading_rss_1_0_head = false;
            }
            XmlEvent::End(_) | XmlEvent::Eof => {
                return Ok(PullFnCmd::Done(std::mem::replace(
                    &mut self.output,
                    Default::default(),
                )))
            }
            _ => (),
        }
        Ok(PullFnCmd::Continue)
    }
}

struct ParseText {}

impl PullFn for ParseText {
    type Output = Option<String>;
    fn pull<B>(
        &mut self,
        reader: &mut XmlReader<B>,
        event: XmlEvent,
    ) -> quick_xml::Result<PullFnCmd<Self::Output>>
    where
        B: std::io::BufRead,
    {
        match event {
            XmlEvent::Start(_) => {
                skip_element(reader)?;
            }
            XmlEvent::Text(ref e) => {
                let text = e.unescape_and_decode(reader)?;
                return Ok(PullFnCmd::Done(Some(text)));
            }
            XmlEvent::CData(ref e) => {
                let text = reader.decode(e)?.to_string();
                return Ok(PullFnCmd::Done(Some(text)));
            }
            XmlEvent::End(_) | XmlEvent::Eof => return Ok(PullFnCmd::Done(None)),
            _ => (),
        }
        Ok(PullFnCmd::Continue)
    }
}

struct SkipElement {
    depth: usize,
}

impl SkipElement {
    fn new() -> Self {
        SkipElement { depth: 1 }
    }
}

impl PullFn for SkipElement {
    type Output = ();
    fn pull<B>(
        &mut self,
        _reader: &mut XmlReader<B>,
        event: XmlEvent,
    ) -> quick_xml::Result<PullFnCmd<Self::Output>>
    where
        B: std::io::BufRead,
    {
        match event {
            XmlEvent::Start(_) => {
                self.depth = self.depth.checked_add(1).unwrap();
            }
            XmlEvent::End(_) => {
                self.depth = self.depth.checked_sub(1).unwrap();
            }
            XmlEvent::Eof => return Ok(PullFnCmd::Done(())), // ignore unexpected EOF
            _ if self.depth == 0 => return Ok(PullFnCmd::Done(())),
            _ => (),
        }
        Ok(PullFnCmd::Continue)
    }
}*/

pub trait FromXml: Sized {
    fn from_xml<B: std::io::BufRead>(
        bufs: &mut Vec<Vec<u8>>,
        reader: &mut XmlReader<B>,
        start: &BytesStart,
    ) -> quick_xml::Result<Self>;
}

#[derive(Debug, Eq, PartialEq)]
enum AtomLink<'a> {
    Alternate(String),
    Source(String),
    Hub(String),
    Other(String, Cow<'a, str>),
}

fn parse_atom_link<'a, B: std::io::BufRead>(
    reader: &mut XmlReader<B>,
    attributes: Attributes<'a>,
) -> quick_xml::Result<Option<AtomLink<'a>>> {
    let mut href = None;
    let mut rel = None;
    for attribute in attributes {
        let attribute = attribute?;
        match reader.decode(attribute.key)? {
            "href" => href = Some(attribute.unescape_and_decode_value(reader)?),
            "rel" => {
                rel = Some(reader.decode(if let Cow::Borrowed(s) = attribute.value {
                    s
                } else {
                    // Attrbute.value is always Borrowed
                    // https://docs.rs/quick-xml/0.18.1/src/quick_xml/events/attributes.rs.html#244
                    unreachable!()
                })?)
            }
            _ => (),
        }
    }
    Ok(href.map(move |href| {
        if let Some(rel) = rel {
            match rel {
                "alternate" => AtomLink::Alternate(href),
                "self" => AtomLink::Source(href),
                "hub" => AtomLink::Hub(href),
                _ => AtomLink::Other(href, Cow::Borrowed(rel)),
            }
        } else {
            AtomLink::Alternate(href)
        }
    }))
}

struct SkipThisElement;

impl FromXml for SkipThisElement {
    fn from_xml<B: std::io::BufRead>(
        bufs: &mut Vec<Vec<u8>>,
        reader: &mut XmlReader<B>,
        _start: &BytesStart,
    ) -> quick_xml::Result<Self> {
        let mut buf = bufs.pop().unwrap_or_default();
        let mut depth = 1u64;
        loop {
            match reader.read_event(&mut buf) {
                Ok(XmlEvent::Start(_)) => depth += 1,
                Ok(XmlEvent::End(_)) if depth == 1 => break,
                Ok(XmlEvent::End(_)) => depth -= 1,
                Ok(XmlEvent::Eof) => break, // just ignore EOF
                Err(err) => return Err(err.into()),
                _ => (),
            }
            buf.clear();
        }
        bufs.push(buf);
        Ok(SkipThisElement)
    }
}

impl FromXml for Option<String> {
    fn from_xml<B: std::io::BufRead>(
        bufs: &mut Vec<Vec<u8>>,
        reader: &mut XmlReader<B>,
        _start: &BytesStart,
    ) -> quick_xml::Result<Self> {
        let mut buf = bufs.pop().unwrap_or_default();
        let mut content: Option<String> = None;
        loop {
            match reader.read_event(&mut buf) {
                Ok(XmlEvent::Start(ref e)) => {
                    SkipThisElement::from_xml(bufs, reader, e)?;
                }
                Ok(XmlEvent::Text(ref e)) => {
                    let text = e.unescape_and_decode(reader)?;
                    content = Some(text);
                }
                Ok(XmlEvent::CData(ref e)) => {
                    let text = reader.decode(e)?.to_string();
                    content = Some(text);
                }
                Ok(XmlEvent::End(_)) | Ok(XmlEvent::Eof) => break,
                Err(err) => return Err(err.into()),
                _ => (),
            }
            buf.clear();
        }
        bufs.push(buf);
        Ok(content)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RSS {
    pub title: String,
    pub link: String,
    pub source: Option<String>,
    pub items: Vec<Item>,
}

impl FromXml for RSS {
    fn from_xml<B: std::io::BufRead>(
        bufs: &mut Vec<Vec<u8>>,
        reader: &mut XmlReader<B>,
        _start: &BytesStart,
    ) -> quick_xml::Result<Self> {
        let mut buf = bufs.pop().unwrap_or_default();
        let mut rss = RSS::default();
        let mut reading_rss_1_0_head = false;
        loop {
            match reader.read_event(&mut buf) {
                Ok(XmlEvent::Empty(ref e)) => {
                    if reader.decode(e.local_name())? == "link" {
                        match parse_atom_link(reader, e.attributes())? {
                            Some(AtomLink::Alternate(link)) => rss.link = link,
                            Some(AtomLink::Source(link)) => rss.source = Some(link),
                            _ => {}
                        }
                    }
                }
                Ok(XmlEvent::Start(ref e)) => {
                    match reader.decode(e.local_name())? {
                        "channel" => {
                            // RSS 0.9 1.0
                            reading_rss_1_0_head = true;
                        }
                        "title" => {
                            if let Some(title) =
                                <Option<String> as FromXml>::from_xml(bufs, reader, e)?
                            {
                                rss.title = title;
                            }
                        }
                        "link" => {
                            if let Some(link) =
                                <Option<String> as FromXml>::from_xml(bufs, reader, e)?
                            {
                                // RSS
                                rss.link = link;
                            } else {
                                // ATOM
                                match parse_atom_link(reader, e.attributes())? {
                                    Some(AtomLink::Alternate(link)) => rss.link = link,
                                    Some(AtomLink::Source(link)) => rss.source = Some(link),
                                    _ => {}
                                }
                            }
                        }
                        "item" | "entry" => {
                            rss.items.push(Item::from_xml(bufs, reader, e)?);
                        }
                        _ => {
                            SkipThisElement::from_xml(bufs, reader, e)?;
                        }
                    }
                }
                Ok(XmlEvent::End(_)) if reading_rss_1_0_head => {
                    // reader.decode(e.local_name())? == "channel";
                    reading_rss_1_0_head = false;
                }
                Ok(XmlEvent::End(_)) | Ok(XmlEvent::Eof) => break,
                Err(err) => return Err(err.into()),
                _ => (),
            }
            buf.clear();
        }
        bufs.push(buf);
        Ok(rss)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    pub title: Option<String>,
    pub link: Option<String>,
    pub id: Option<String>,
}

impl FromXml for Item {
    fn from_xml<B: std::io::BufRead>(
        bufs: &mut Vec<Vec<u8>>,
        reader: &mut XmlReader<B>,
        _start: &BytesStart,
    ) -> quick_xml::Result<Self> {
        let mut buf = bufs.pop().unwrap_or_default();
        let mut item = Item::default();
        loop {
            match reader.read_event(&mut buf) {
                Ok(XmlEvent::Empty(ref e)) => {
                    if reader.decode(e.name())? == "link" {
                        if let Some(AtomLink::Alternate(link)) =
                            parse_atom_link(reader, e.attributes())?
                        {
                            item.link = Some(link);
                        }
                    }
                }
                Ok(XmlEvent::Start(ref e)) => {
                    match reader.decode(e.name())? {
                        "title" => {
                            item.title = <Option<String> as FromXml>::from_xml(bufs, reader, e)?;
                        }
                        "link" => {
                            if let Some(link) =
                                <Option<String> as FromXml>::from_xml(bufs, reader, e)?
                            {
                                // RSS
                                item.link = Some(link);
                            } else if let Some(AtomLink::Alternate(link)) =
                                parse_atom_link(reader, e.attributes())?
                            {
                                // ATOM
                                item.link = Some(link);
                            }
                        }
                        "id" | "guid" => {
                            item.id = <Option<String> as FromXml>::from_xml(bufs, reader, e)?;
                        }
                        _ => {
                            SkipThisElement::from_xml(bufs, reader, e)?;
                        }
                    }
                }
                Ok(XmlEvent::End(_)) | Ok(XmlEvent::Eof) => break,
                Err(err) => return Err(err.into()),
                _ => (),
            }
            buf.clear();
        }
        bufs.push(buf);
        Ok(item)
    }
}

pub fn parse<B: std::io::BufRead>(reader: B) -> quick_xml::Result<RSS> {
    let mut reader = XmlReader::from_reader(reader);
    reader.trim_text(true);
    let mut bufs = vec![Vec::with_capacity(512); 4];
    let mut buf = bufs.pop().unwrap_or_default();
    loop {
        match reader.read_event(&mut buf) {
            Ok(XmlEvent::Start(ref e)) => match reader.decode(e.name())? {
                "rss" => continue,
                "channel" | "feed" | "rdf:RDF" => {
                    return RSS::from_xml(&mut bufs, &mut reader, e);
                }
                _ => {
                    SkipThisElement::from_xml(&mut bufs, &mut reader, e)?;
                }
            },
            Ok(XmlEvent::Eof) => return Err(quick_xml::Error::UnexpectedEof("feed".to_string())),
            Err(err) => return Err(err.into()),
            _ => (),
        }
        buf.clear();
    }
}

fn url_relative_to_absolute(link: &mut String, host: &str) {
    match link.as_str() {
        _ if link.starts_with("//") => {
            let mut s = String::from("http:");
            s.push_str(link);
            *link = s;
        }
        _ if link.starts_with('/') => {
            let mut s = String::from(host);
            s.push_str(link);
            *link = s;
        }
        _ => (),
    }
}

pub fn fix_relative_url(mut rss: RSS, rss_link: &str) -> RSS {
    lazy_static! {
        static ref HOST: Regex = Regex::new(r"^(https?://[^/]+)").unwrap();
    }
    let rss_host = HOST
        .captures(rss_link)
        .map_or(rss_link, |r| r.get(0).unwrap().as_str());
    match rss.link.as_str() {
        "" | "/" => rss.link = rss_host.to_owned(),
        _ => url_relative_to_absolute(&mut rss.link, rss_host),
    }
    for item in &mut rss.items {
        if let Some(link) = item.link.as_mut() {
            url_relative_to_absolute(link, rss_host);
        }
    }

    rss
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn atom03() {
        let s = include_str!("../tests/data/atom_0.3.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "atom_0.3.feed.title".into(),
                link: "atom_0.3.feed.link^href".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("atom_0.3.feed.entry[0].title".into()),
                        link: Some("atom_0.3.feed.entry[0].link^href".into()),
                        id: Some("atom_0.3.feed.entry[0]^id".into()),
                    },
                    Item {
                        title: Some("atom_0.3.feed.entry[1].title".into()),
                        link: Some("atom_0.3.feed.entry[1].link^href".into()),
                        id: Some("atom_0.3.feed.entry[1]^id".into()),
                    },
                ],
            }
        );
    }

    #[test]
    fn atom10() {
        let s = include_str!("../tests/data/atom_1.0.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "atom_1.0.feed.title".into(),
                link: "http://example.com/blog_plain".into(),
                source: Some("http://example.com/blog/atom_1.0.xml".into()),
                items: vec![
                    Item {
                        title: Some("atom_1.0.feed.entry[0].title".into()),
                        link: Some("http://example.com/blog/entry1_plain".into()),
                        id: Some("atom_1.0.feed.entry[0]^id".into()),
                    },
                    Item {
                        title: Some("atom_1.0.feed.entry[1].title".into()),
                        link: Some("http://example.com/blog/entry2".into()),
                        id: Some("atom_1.0.feed.entry[1]^id".into()),
                    },
                ],
            }
        );
    }

    #[test]
    fn rss09() {
        let s = include_str!("../tests/data/rss_0.9.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_0.9.channel.title".into(),
                link: "rss_0.9.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_0.9.item[0].title".into()),
                        link: Some("rss_0.9.item[0].link".into()),
                        id: None,
                    },
                    Item {
                        title: Some("rss_0.9.item[1].title".into()),
                        link: Some("rss_0.9.item[1].link".into()),
                        id: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn rss091() {
        let s = include_str!("../tests/data/rss_0.91.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_0.91.channel.title".into(),
                link: "rss_0.91.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_0.91.channel.item[0].title".into()),
                        link: Some("rss_0.91.channel.item[0].link".into()),
                        id: None,
                    },
                    Item {
                        title: Some("rss_0.91.channel.item[1].title".into()),
                        link: Some("rss_0.91.channel.item[1].link".into()),
                        id: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn rss092() {
        let s = include_str!("../tests/data/rss_0.92.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_0.92.channel.title".into(),
                link: "rss_0.92.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_0.92.channel.item[0].title".into()),
                        link: Some("rss_0.92.channel.item[0].link".into()),
                        id: None,
                    },
                    Item {
                        title: Some("rss_0.92.channel.item[1].title".into()),
                        link: Some("rss_0.92.channel.item[1].link".into()),
                        id: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn rss093() {
        let s = include_str!("../tests/data/rss_0.93.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_0.93.channel.title".into(),
                link: "rss_0.93.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_0.93.channel.item[0].title".into()),
                        link: Some("rss_0.93.channel.item[0].link".into()),
                        id: None,
                    },
                    Item {
                        title: Some("rss_0.93.channel.item[1].title".into()),
                        link: Some("rss_0.93.channel.item[1].link".into()),
                        id: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn rss094() {
        let s = include_str!("../tests/data/rss_0.94.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_0.94.channel.title".into(),
                link: "rss_0.94.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_0.94.channel.item[0].title".into()),
                        link: Some("rss_0.94.channel.item[0].link".into()),
                        id: Some("rss_0.94.channel.item[0].guid".into()),
                    },
                    Item {
                        title: Some("rss_0.94.channel.item[1].title".into()),
                        link: Some("rss_0.94.channel.item[1].link".into()),
                        id: Some("rss_0.94.channel.item[1].guid".into()),
                    },
                ],
            }
        );
    }

    #[test]
    fn rss10() {
        let s = include_str!("../tests/data/rss_1.0.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_1.0.channel.title".into(),
                link: "rss_1.0.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_1.0.item[0].title".into()),
                        link: Some("rss_1.0.item[0].link".into()),
                        id: None,
                    },
                    Item {
                        title: Some("rss_1.0.item[1].title".into()),
                        link: Some("rss_1.0.item[1].link".into()),
                        id: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn rss20() {
        let s = include_str!("../tests/data/rss_2.0.xml");
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(
            r,
            RSS {
                title: "rss_2.0.channel.title".into(),
                link: "rss_2.0.channel.link".into(),
                source: None,
                items: vec![
                    Item {
                        title: Some("rss_2.0.channel.item[0].title".into()),
                        link: Some("rss_2.0.channel.item[0].link".into()),
                        id: Some("rss_2.0.channel.item[0].guid".into()),
                    },
                    Item {
                        title: Some("rss_2.0.channel.item[1].title".into()),
                        link: Some("rss_2.0.channel.item[1].link".into()),
                        id: Some("rss_2.0.channel.item[1].guid".into()),
                    },
                ],
            }
        );
    }

    #[test]
    fn rss_with_atom_ns() {
        let s = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
<atom:link href="self link" rel="self" />
</channel>
</rss>"#;
        let r = parse(Cursor::new(s)).unwrap();
        assert_eq!(r.source, Some("self link".into()));
    }

    #[test]
    fn atom_link_parsing() {
        let data = vec![
            r#"<link href="alternate href" />"#,
            r#"<link href="alternate href" rel="alternate" />"#,
            r#"<link href="self href" rel="self" />"#,
            r#"<link href="hub href" rel="hub" />"#,
            r#"<link href="other href" rel="other" />"#,
            r#"<link />"#,
        ];
        let results = vec![
            Some(AtomLink::Alternate("alternate href".into())),
            Some(AtomLink::Alternate("alternate href".into())),
            Some(AtomLink::Source("self href".into())),
            Some(AtomLink::Hub("hub href".into())),
            Some(AtomLink::Other(
                "other href".into(),
                Cow::Owned("other".into()),
            )),
            None,
        ];
        for (data, result) in data.iter().zip(results) {
            let mut reader = XmlReader::from_reader(Cursor::new(data));
            let mut buf = Vec::new();
            if let XmlEvent::Empty(e) = reader.read_event(&mut buf).unwrap() {
                let r = parse_atom_link(&mut reader, e.attributes()).unwrap();
                assert_eq!(r, result);
            }
        }
    }

    #[test]
    fn empty_input() {
        let r = parse(Cursor::new(&[])).unwrap_err();
        assert!(matches!(r, quick_xml::Error::UnexpectedEof(s) if s == "feed" ))
    }
}
