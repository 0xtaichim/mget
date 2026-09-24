use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

/// Minimal indented XML writer for NFO documents.
pub struct Xml {
    writer: Writer<Vec<u8>>,
}

impl Xml {
    /// Builds a complete document with an XML declaration and a single root element.
    pub fn document(root: &str, build: impl FnOnce(&mut Self)) -> String {
        let mut xml = Self {
            writer: Writer::new_with_indent(Vec::new(), b' ', 2),
        };
        xml.emit(Event::Decl(BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            Some("yes"),
        )));
        xml.block(root, &[], build);
        String::from_utf8(xml.writer.into_inner()).expect("quick-xml emits valid UTF-8")
    }

    /// `<name>value</name>`, omitted when `value` is empty.
    pub fn text(&mut self, name: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.text_with(name, &[], value);
        }
        self
    }

    pub fn opt(&mut self, name: &str, value: Option<&str>) -> &mut Self {
        if let Some(v) = value {
            self.text(name, v);
        }
        self
    }

    /// `<name attr="..">value</name>`, written even when `value` is empty.
    pub fn text_with(&mut self, name: &str, attrs: &[(&str, &str)], value: &str) -> &mut Self {
        self.emit(Event::Start(start(name, attrs)));
        self.emit(Event::Text(BytesText::new(value)));
        self.emit(Event::End(BytesEnd::new(name)));
        self
    }

    /// `<name attr="..">…children…</name>`
    pub fn block(
        &mut self,
        name: &str,
        attrs: &[(&str, &str)],
        children: impl FnOnce(&mut Self),
    ) -> &mut Self {
        self.emit(Event::Start(start(name, attrs)));
        children(self);
        self.emit(Event::End(BytesEnd::new(name)));
        self
    }

    fn emit(&mut self, event: Event<'_>) {
        self.writer
            .write_event(event)
            .expect("writing to Vec<u8> cannot fail");
    }
}

fn start<'a>(name: &'a str, attrs: &[(&str, &str)]) -> BytesStart<'a> {
    let mut start = BytesStart::new(name);
    for &attr in attrs {
        start.push_attribute(attr);
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_escaped_indented_document() {
        let doc = Xml::document("movie", |x| {
            x.text("title", "Tom & Jerry")
                .text("empty", "")
                .opt("none", None);
            x.block("set", &[("id", "1")], |x| {
                x.text("name", "<A>");
            });
        });
        assert_eq!(
            doc,
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <movie>\n  <title>Tom &amp; Jerry</title>\n  <set id=\"1\">\n    \
             <name>&lt;A&gt;</name>\n  </set>\n</movie>"
        );
    }
}
