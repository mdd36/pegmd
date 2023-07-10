use std::fmt::Display;

use pest::{iterators::{Pairs, Pair}, Parser};
use pest_derive::Parser;
use strum_macros::Display;

#[derive(Debug, PartialEq)]
pub enum Text<'a> {
    Plain(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    Underline(&'a str),
    Strikethrough(&'a str),
    Link(&'a str, &'a str),
    Image(&'a str, &'a str),
    InlineCode(&'a str),
    Linebreak,
}

#[derive(Debug)]
pub struct ListItem<'a> {
    sublist: Option<Box<List<'a>>>,
    text: Vec<Text<'a>>,
}

#[derive(Debug)]
pub struct List<'a> {
    ordered: bool,
    elements: Vec<ListItem<'a>>,
}

#[derive(Debug)]
pub enum Section<'a> {
    Heading(i32, &'a str),
    Paragraph(Vec<Text<'a>>),
    List(List<'a>),
    Verbatim(Vec<Text<'a>>),
    Codeblock(Option<&'a str>, &'a str),
}

#[derive(Debug)]
pub struct Document<'a> {
    pub sections: Vec<Section<'a>>
}

impl <'a> Document<'a> {
    pub fn new(sections: Vec<Section<'a>>) -> Self {
        Self {
            sections
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
  GrammarError(String),
  MalformedInput(String),
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(value: pest::error::Error<Rule>) -> Self {
        ParseError::GrammarError(value.to_string())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GrammarError(msg) => write!(f, "GrammarError({msg})"),
            Self::MalformedInput(msg) => write!(f, "MalformedInput({msg})"),
        }
    }
}

#[derive(Parser)]
#[grammar = "markdown.pest"]
pub struct MarkdownParser;

pub fn parse_document<'a>(input: &'a str) -> Result<Document<'a>, ParseError> {
    let sections = MarkdownParser::parse(Rule::document, input)?;
    let mut parsed_sections = Vec::new();
    for section in sections {
      if let Some(s) = parse_section(section)? {
        parsed_sections.push(s);
      }
    }
    Ok(Document::new(parsed_sections))
}

fn parse_section<'a>(root: Pair<'a, Rule>) -> Result<Option<Section<'a>>, ParseError> {
    match root.as_rule() {
        Rule::paragraph => Ok(Some(Section::Paragraph(inlines_from_pairs(root.into_inner())?))),
        Rule::codeblock => codeblock_from_pairs(root.into_inner()),
        Rule::verbatim => Ok(Some(Section::Verbatim(inlines_from_pairs(root.into_inner())?))),
        Rule::header => header_from_pairs(root.into_inner()).map(|x| Some(x)),
        Rule::EOI => Ok(None),
        ty => Err(ParseError::GrammarError(format!("Section type not implemented yet: {ty:?}"))),
    }
}

fn codeblock_from_pairs<'a>(root: Pairs<'a, Rule>) -> Result<Option<Section<'a>>, ParseError> {
    let lang = root
        .find_first_tagged("lang")
        .map(|n| n.as_str());
    let body = root
        .find_first_tagged("body")
        .ok_or(ParseError::MalformedInput(format!("Failed to find body for paragraph in {}", root.as_str())))?
        .as_str();
    Ok(Some(Section::Codeblock(lang, body)))
}

fn header_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<Section<'a>, ParseError> {
    // While each of the find_map calls do advance the iterator, the nodes must be in this order for the parse to be valid
    // so it's safe mutate
    let hashes = root
        .find_map(|n| if n.as_rule() == Rule::header_hashes { Some(n.as_str()) } else { None })
        .ok_or(ParseError::MalformedInput(format!("Failed to find hashes for header in: {}", root.as_str())))?;
    let title = root
        .find_map(|n| if n.as_rule() == Rule::header_title { Some(n.as_str()) } else { None })
        .ok_or(ParseError::MalformedInput(format!("Failed to find title for header in {}", root.as_str())))?;
    Ok(Section::Heading(hashes.len() as i32, title))

}

fn inlines_from_pairs<'a>(root: Pairs<'a, Rule>) -> Result<Vec<Text<'a>>, ParseError> {
    let mut nodes = Vec::new();
    for node in root {
      if let Some(n) = text_from_pair(node)? {
        nodes.push(n);
      }
    }

    Ok(nodes)
}

fn text_from_pair<'a>(node: Pair<'a, Rule>) -> Result<Option<Text<'a>>, ParseError> {
    match node.as_rule() {
        Rule::str => Ok(Some(Text::Plain(node.as_str()))),
        Rule::strong => Ok(Some(Text::Bold(node.as_str()))),
        Rule::italic => Ok(Some(Text::Italic(node.as_str()))),
        Rule::underline => Ok(Some(Text::Underline(node.as_str()))),
        Rule::inline_code => Ok(Some(Text::InlineCode(node.as_str()))),
        Rule::strikethrough => Ok(Some(Text::Strikethrough(node.as_str()))),
        Rule::link => {
          let inner = node.into_inner();
          let url = inner
            .find_first_tagged("url")
            .ok_or(ParseError::MalformedInput(format!("Failed to find url in {}", inner.as_str())))?
            .as_str();
          let alt = inner
            .find_first_tagged("alt")
            .map_or(url, |node| node.as_str());
          Ok(Some(Text::Link(alt, url)))
        }
        Rule::image => {
          let inner = node.into_inner();
          let url = inner
            .find_first_tagged("url")
            .ok_or(ParseError::MalformedInput(format!("Failed to find url in {}", inner.as_str())))?
            .as_str();
          let alt = inner
            .find_first_tagged("alt")
            .map_or(url, |node| node.as_str());
          Ok(Some(Text::Image(alt, url)))
        }
        Rule::EOI => Ok(None),
        Rule::linebreak => Ok(Some(Text::Linebreak)),
        ty => unreachable!("Text not implemented yet: {ty:?}"),
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn basic_bold_test() {
        let document = match parse_document("A basic test that **bold** text renders. Ignores **bold\n** if a line break occurs.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A basic test that "),
            Text::Bold("bold"),
            Text::Plain(" text renders. Ignores **bold") ,
            Text::Plain("** if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_italic_test() {
        let document = match parse_document("A basic test that *italic* text renders. Ignores *italic\n* if a line break occurs.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A basic test that "),
            Text::Italic("italic"),
            Text::Plain(" text renders. Ignores *italic"),
            Text::Plain("* if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_strike_through_test() {
        let document = match parse_document("A basic test that ~~strike through~~ text renders. Ignores ~~strike through\n~~ if a line break occurs.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A basic test that "),
            Text::Strikethrough("strike through"),
            Text::Plain(" text renders. Ignores ~~strike through"),
            Text::Plain("~~ if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_underline_test() {
        let document = match parse_document("A basic test that _underline_ text renders. Ignores _underline\n_ if a line break occurs.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A basic test that "),
            Text::Underline("underline"),
            Text::Plain(" text renders. Ignores _underline"),
            Text::Plain("_ if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_inline_code_test() {
        let document = match parse_document("A basic test that `code` text renders. Ignores `code\n` if a line break occurs.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A basic test that "),
            Text::InlineCode("code"),
            Text::Plain(" text renders. Ignores `code"),
            Text::Plain("` if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_link_test() {
        let document = match parse_document("A simple test that [links](www.google.com) to <www.wikipedia.com> render.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A simple test that "),
            Text::Link("links", "www.google.com"),
            Text::Plain(" to "),
            Text::Link("www.wikipedia.com", "www.wikipedia.com"),
            Text::Plain(" render."),
        ]);
    }

    #[test]
    pub fn intermixed_markup_test() {
        let document = match parse_document("Some **intermixed** *styles* like _underlines_, [links](www.google.com), and `code`.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("Some "),
            Text::Bold("intermixed"),
            Text::Plain(" "),
            Text::Italic("styles"),
            Text::Plain(" like "),
            Text::Underline("underlines"),
            Text::Plain(", "),
            Text::Link("links", "www.google.com"),
            Text::Plain(", and "),
            Text::InlineCode("code"),
            Text::Plain("."),
        ]);
    }

    #[test]
    pub fn paragraph_separation_test() {
        let document = match parse_document("This is the first paragraph.\nThis line is still part of that paragraph.\n\n  This is a new paragraph.  \nIt has different lines within it.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        assert_eq!(document.sections.len(), 2);

        let paragraph_one = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_one, &vec![
            Text::Plain("This is the first paragraph."),
            Text::Plain("This line is still part of that paragraph.")
        ]);

       let paragraph_two = match &document.sections[1] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_two, &vec![
            Text::Plain("This is a new paragraph."),
            Text::Linebreak,
            Text::Plain("It has different lines within it."),
        ]); 
    }

    #[test]
    pub fn block_quote_test() {
        let document = match parse_document("This is the first paragraph.\n\n    This is a block quote.\n\n>This is also a block quote.") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        assert_eq!(document.sections.len(), 3);

        let paragraph_one = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_one, &vec![
            Text::Plain("This is the first paragraph."),
        ]);

       let paragraph_two = match &document.sections[1] {
            Section::Verbatim(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_two, &vec![
            Text::Plain("This is a block quote."),
        ]); 

        let paragraph_two = match &document.sections[2] {
            Section::Verbatim(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_two, &vec![
            Text::Plain("This is also a block quote."),
        ]); 
    }

    #[test]
    pub fn code_block_test() {
        let document = match parse_document("```python\nprint('hello world')\n```\n\n``` sh \n    echo 'hello world'\n```\n\n```  \n  puts 'hello world'\n```") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        assert_eq!(document.sections.len(), 3);

        match &document.sections[0] {
            Section::Codeblock(lang, body) => {
                assert_eq!(*lang, Some("python"));
                assert_eq!(*body, "print('hello world')")
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[1] {
            Section::Codeblock(lang, body) => {
                assert_eq!(*lang, Some("sh"));
                assert_eq!(*body, "    echo 'hello world'");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[2] {
            Section::Codeblock(lang, body) => {
                assert!(matches!(*lang, None));
                assert_eq!(*body, "  puts 'hello world'")
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
    }

    #[test]
    pub fn image_test() {
        let document = match parse_document("![alt text](destination.co) ![another one](target.io)") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        match &document.sections[0] {
            Section::Paragraph(items) => {
                assert_eq!(*items, vec![
                    Text::Image("alt text", "destination.co"),
                    Text::Image("another one", "target.io")
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
    }

    #[test]
    pub fn header_test() {
        let document = match parse_document("  # Header 1 \n\n####   Header 4\n\n####### Too many hashes for a header") {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        match &document.sections[0] {
            Section::Heading(level, titles) => {
                assert_eq!(*level, 1);
                assert_eq!(*titles, "Header 1");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[1] {
            Section::Heading(level, titles) => {
                assert_eq!(*level, 4);
                assert_eq!(*titles, "Header 4");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
        match &document.sections[2] {
            Section::Paragraph(items) => {
                assert_eq!(*items, vec![
                    Text::Plain("####### Too many hashes for a header")
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
    }    
}