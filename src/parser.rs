use std::fmt::Display;

use pest::{iterators::{Pairs, Pair}, Parser};
use pest_derive::Parser;

#[derive(Debug, PartialEq)]
pub enum Text<'a> {
    Plain(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    Underline(&'a str),
    Strikethrough(&'a str),
    Link(Link<'a>),
    Image(Link<'a>),
    InlineCode(&'a str),
    Linebreak,
}

#[derive(Debug, PartialEq)]
pub struct Link<'a> {
    alt: &'a str,
    uri: &'a str,
}

#[derive(Debug, Default, PartialEq)]
pub struct ListItem<'a> {
    inline_text: Option<Vec<Text<'a>>>,
    block_items: Option<Vec<Section<'a>>>,
}

#[derive(Debug, PartialEq)]
pub struct List<'a> {
    ordered: bool,
    items: Vec<ListItem<'a>>,
}

impl <'a> List<'a> {
    pub fn new(ordered: bool, items: Vec<ListItem<'a>>) -> Self {
        Self {
            ordered,
            items
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Section<'a> {
    Heading(i32, &'a str),
    Paragraph(Vec<Text<'a>>),
    List(List<'a>),
    Verbatim(Vec<Text<'a>>),
    Codeblock(Option<&'a str>, &'a str),
    Plain(&'a str),
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
            Self::GrammarError(msg) => write!(f, "GrammarError: {msg}"),
            Self::MalformedInput(msg) => write!(f, "MalformedInput: {msg}"),
        }
    }
}

#[derive(Parser)]
#[grammar = "markdown.pest"]
pub struct MarkdownParser;

pub fn parse_document<'a>(input: &'a str) -> Result<Document<'a>, ParseError> {
    let sections = MarkdownParser::parse(Rule::document, input)?;
    let parsed_sections: Result<Vec<Section<'a>>, ParseError> = sections.into_iter()
        .filter_map(|section| parse_section(section).transpose())
        .collect();
    Ok(Document::new(parsed_sections?))

}

fn parse_section<'a>(root: Pair<'a, Rule>) -> Result<Option<Section<'a>>, ParseError> {
    match root.as_rule() {
        Rule::paragraph => Ok(Some(Section::Paragraph(inlines_from_pairs(root.into_inner())?))),
        Rule::codeblock => codeblock_from_pairs(root.into_inner()),
        Rule::verbatim => Ok(Some(Section::Verbatim(inlines_from_pairs(root.into_inner())?))),
        Rule::header => header_from_pairs(root.into_inner()).map(|x| Some(x)),
        Rule::bullet_list => Ok(Some(Section::List(List::new(false, list_items_from_pairs(root.into_inner())?)))),
        Rule::ordered_list => Ok(Some(Section::List(List::new(true, list_items_from_pairs(root.into_inner())?)))),
        Rule::EOI | Rule::COMMENT => Ok(None),
        ty => Err(ParseError::GrammarError(format!("Section type not implemented yet: {ty:?}\n\t{root:?}"))),
    }
}

fn list_items_from_pairs<'a>(root: Pairs<'a, Rule>) -> Result<Vec<ListItem<'a>>, ParseError> {
    root.into_iter()
        .map(|list_item| list_item_from_pairs(list_item.into_inner()))
        .collect()
}

fn list_item_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<ListItem<'a>, ParseError> {
    let first_node = match root.next() {
        Some(n) => n,
        None => return Ok(ListItem::default())
    };

    if first_node.as_rule() == Rule::list_content_continuation {
        let block_items: Result<Vec<Section<'a>>, ParseError> = first_node.into_inner()
            .filter_map(|section| parse_section(section).transpose())
            .collect();
        return Ok(ListItem { inline_text: None, block_items: Some(block_items?) })
    }

    let inline_text = Some(inlines_from_pairs(first_node.into_inner())?);
    let block_items = match root.next() {
        Some(pair) => {
            let parsed_content: Result<Vec<Section<'a>>, ParseError> = pair.into_inner()
            .flat_map(|section| parse_section(section).transpose())
            .collect();
            Some(parsed_content?)
        },
        None => None,
    };

    Ok(ListItem { inline_text, block_items })
}

fn codeblock_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<Option<Section<'a>>, ParseError> {
    let lang = root
        .next()
        .map(|n| n.as_str())
        .filter(|lang| !lang.is_empty());
    let body = root
        .next()
        .ok_or(ParseError::MalformedInput(format!("Failed to find body for paragraph in {root:?}")))?
        .as_str();
    Ok(Some(Section::Codeblock(lang, body)))
}

fn header_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<Section<'a>, ParseError> {
    // While each of the find_map calls do advance the iterator, the nodes must be in this order for the parse to be valid
    // so it's safe mutate
    let hashes = root
        .find_map(|n| if n.as_rule() == Rule::header_hashes { Some(n.as_str()) } else { None })
        .ok_or(ParseError::MalformedInput(format!("Failed to find hashes for header in: {root:?}")))?;
    let title = root
        .find_map(|n| if n.as_rule() == Rule::header_title { Some(n.as_str()) } else { None })
        .ok_or(ParseError::MalformedInput(format!("Failed to find title for header in {root:?}")))?;
    Ok(Section::Heading(hashes.len() as i32, title))

}

fn inlines_from_pairs<'a>(root: Pairs<'a, Rule>) -> Result<Vec<Text<'a>>, ParseError> {
    root.into_iter()
        .filter_map(|inline| text_from_pair(inline).transpose())
        .collect()
}

fn text_from_pair<'a>(node: Pair<'a, Rule>) -> Result<Option<Text<'a>>, ParseError> {
    match node.as_rule() {
        Rule::str => Ok(Some(Text::Plain(node.as_str()))),
        Rule::strong => Ok(Some(Text::Bold(node.as_str()))),
        Rule::italic => Ok(Some(Text::Italic(node.as_str()))),
        Rule::underline => Ok(Some(Text::Underline(node.as_str()))),
        Rule::inline_code => Ok(Some(Text::InlineCode(node.as_str()))),
        Rule::strikethrough => Ok(Some(Text::Strikethrough(node.as_str()))),
        Rule::directed_link | Rule::autolink => Ok(Some(Text::Link(link_from_pair(node)?))),
        Rule::image => Ok(Some(Text::Image(link_from_pair(node)?))),
        Rule::EOI | Rule::COMMENT => Ok(None),
        Rule::linebreak => Ok(Some(Text::Linebreak)),
        ty => Err(ParseError::GrammarError(format!("Text rule not implemented yet: {ty:?}\n\t{node:?}"))),
    }
}

fn link_from_pair<'a>(node: Pair<'a, Rule>) -> Result<Link<'a>, ParseError> {
    let link_type = node.as_rule();
    let mut inner = node.into_inner();
    let alt = match link_type {
        Rule::autolink => None,
        Rule::image | Rule::directed_link => Some(
            inner.next()
                .map(|n| n.as_str())
                .ok_or(ParseError::MalformedInput(format!("Failed to find alt text for link in {inner:?}")))?
        ),
        rule => return Err(ParseError::MalformedInput(format!("Unexpected rule found while attempting to parse a link: {rule:?}\n\t{inner:?}"))),
    };

    let uri = inner.next()
        .map(|n| n.as_str())
        .ok_or(ParseError::MalformedInput(format!("URI missing from link {inner:?}")))?;

    Ok(Link { alt: alt.unwrap_or(uri), uri })
}


#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    pub fn basic_bold_test() {
        let document = match parse_document(indoc! {"
            A basic test that **bold** text renders. Ignores **bold
            ** if a line break occurs.
        "}) {
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
        let document = match parse_document(indoc! {"
            A basic test that *italic* text renders. Ignores *italic
            * if a line break occurs.
        "}) {
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
        let document = match parse_document(indoc! {"
            A basic test that ~~strike through~~ text renders. Ignores ~~strike through
            ~~ if a line break occurs.
        "}) {
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
        let document = match parse_document(indoc! {"
            A basic test that _underline_ text renders. Ignores _underline
            _ if a line break occurs.
        "}) {
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
        let document = match parse_document(indoc! {"
            A basic test that `code` text renders. Ignores `code\n` if a line break occurs.
        "}) {
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
        let document = match parse_document(indoc! {"
            A simple test that [links](www.google.com) to <www.wikipedia.com> render.
        "}) {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        let text = match &document.sections[0] {
            Section::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(text, &vec![
            Text::Plain("A simple test that "),
            Text::Link(Link { alt: "links", uri: "www.google.com" }),
            Text::Plain(" to "),
            Text::Link(Link { alt: "www.wikipedia.com", uri: "www.wikipedia.com" }),
            Text::Plain(" render."),
        ]);
    }

    #[test]
    pub fn intermixed_markup_test() {
        let document = match parse_document(indoc! {"
            Some **intermixed** *styles* like _underlines_, [links](www.google.com), and `code`.
        
        "}) {
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
            Text::Link(Link { alt: "links", uri: "www.google.com" }),
            Text::Plain(", and "),
            Text::InlineCode("code"),
            Text::Plain("."),
        ]);
    }

    #[test]
    pub fn paragraph_separation_test() {
        let document = match parse_document(indoc! {"
            This is the first paragraph.
            This line is still part of that paragraph.
            
            This is a new paragraph.  
            It has different lines within it.
        "}) {
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
        let document = match parse_document(indoc! {"
            This is the first paragraph.
            
                This is a block quote.
                
            > This is also a block quote.
        "}) {
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
        let document = match parse_document(indoc! {"
            ```python
            print('hello world')
            ```
            
            ``` sh 
                echo 'hello world'
            ```
            
            ```  
              puts 'hello world'
            ```
        "}) {
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
        let document = match parse_document(indoc! {"
            ![alt text](destination.co) ![another one](target.io)
        "}) {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        match &document.sections[0] {
            Section::Paragraph(items) => {
                assert_eq!(*items, vec![
                    Text::Image(Link { alt: "alt text", uri: "destination.co" } ),
                    Text::Image(Link { alt: "another one", uri: "target.io"}),
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
    }

    #[test]
    pub fn header_test() {
        let document = match parse_document(indoc! {"
              # Header 1 
            
            ####   Header 4
            
            ####### Too many hashes for a header
        "}) {
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

    #[test]
    pub fn simple_list_test() {
        let document = match parse_document(indoc! {"
            * List item one
            This is a continuation of the last block
            
            > This quote is in li 1
            
              * A sublist
            * List item two
            
            * Lists can span blank lines
            
            [//]: # (break the list)
            This text is not in the list
            
            1. An ordered list
            * Now this is a new bullet list
        "}) {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        match &document.sections[0] {
            Section::List(list) => {
                assert_eq!(list.ordered, false);
                assert_eq!(list.items, vec![
                    ListItem {
                        inline_text: Some(vec![
                            Text::Plain("List item one"),
                            Text::Plain("This is a continuation of the last block")
                        ]),
                        block_items: Some(vec![
                            Section::Verbatim(vec![Text::Plain("This quote is in li 1")]),
                            Section::List(List { ordered: false, items: vec![ 
                                ListItem { inline_text: Some(vec![Text::Plain("A sublist")]), block_items: None } 
                            ]})
                        ])
                    },
                    ListItem {
                        inline_text: Some(vec![Text::Plain("List item two")]),
                        block_items: None,
                    },
                    ListItem {
                        inline_text: Some(vec![Text::Plain("Lists can span blank lines")]),
                        block_items: None,
                    },
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[1] {
            Section::Paragraph(contents) => {
                assert_eq!(contents, &vec![Text::Plain("This text is not in the list")])
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[2] {
            Section::List(list) => {
                assert_eq!(list.ordered, true);
                assert_eq!(list.items, vec![
                    ListItem { inline_text: Some(vec![Text::Plain("An ordered list")]), block_items: None }
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[3] {
            Section::List(list) => {
                assert_eq!(list.ordered, false);
                assert_eq!(list.items, vec![
                    ListItem { inline_text: Some(vec![Text::Plain("Now this is a new bullet list")]), block_items: None }
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }
    }
}