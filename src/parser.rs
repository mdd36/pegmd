use std::{fmt::Display, num::ParseIntError};
use pest::{iterators::{Pairs, Pair}, Parser};
use pest_derive::Parser;

/// Different forms of text that can be present in the document. These are essentially the leaf
/// nodes of the tree.
#[derive(Clone, Debug, PartialEq)]
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

/// A hosted image.
#[derive(Clone, Debug, PartialEq)]
pub struct Link<'a> {
    alt: &'a str,
    uri: &'a str,
}

impl <'a> Link<'a> {

    /// The alt text for the image.
    pub fn alt(&self) -> &str {
        self.alt
    }

    /// The URI source for the image.
    pub fn uri(&self) -> &str {
        self.uri
    }
}

/// A single element at a list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListItem<'a> {
    index: i32,
    inline_text: Option<Vec<Text<'a>>>,
    block_items: Option<Vec<Block<'a>>>,
}

impl <'a> ListItem<'a> {

    /// The index of the element in the list. If the list is ordered, then this should be the number shown
    /// in the list. 
    /// This field is 1-indexed to simplify rendering.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// The initial, inline text for this list item.
    pub fn inlines(&self) -> &Option<Vec<Text<'a>>> {
        &self.inline_text
    }

    /// Block items that belong to this list entry.
    pub fn block_items(&self) -> &Option<Vec<Block<'a>>> {
        &self.block_items
    }
}

/// Different types of sections supported in Markdown.
#[derive(Clone, Debug, PartialEq)]
pub enum Block<'a> {

    /// An ATX header. The i32 represents heading level, while the &str represents the title.
    Heading(i32, &'a str),

    /// A paragraph section.
    Paragraph(Vec<Text<'a>>),

    /// A bullet list. Each of its items might itself contain other blocks.
    BulletList(Vec<ListItem<'a>>),

    /// A numbered list. Each of its items might itself contain other blocks.
    OrderedList(Vec<ListItem<'a>>),

    /// A block quote or verbatim block.
    Verbatim(Vec<Text<'a>>),

    /// A contiguous block of code. The optional first parameter is the language for syntax highlighting, and
    /// the second is the plaintext code in the block.
    Codeblock(Option<&'a str>, &'a str),
}

/// The root of the AST. It contains a Vec<[`Section`]> for the distinct blocks in the document.
/// The lifetime of the Document is constrained by the input source's lifetime since it stores
/// references to the input in its nodes.
#[derive(Clone, Debug)]
pub struct Document<'a> {
    sections: Vec<Block<'a>>
}

impl <'a> Document<'a> {

    /// Get a vector of the sections inside the Document.
    pub fn inner(&self) -> &Vec<Block<'a>> {
        &self.sections
    }

    /// Get an iterator over the [`Block`]s in the document without consuming the document.
    pub fn iter(&'a self) -> impl Iterator<Item=&Block<'a>> {
        self.sections.iter()
    }

    /// Unwrap the document to just the vector of [`Block`]s it contains, consuming it in the process.
    pub fn into_inner(self) -> Vec<Block<'a>> {
        self.sections
    }
}

impl <'a> IntoIterator for Document<'a> {
    type Item = Block<'a>;
    type IntoIter = <Vec<Block<'a>> as IntoIterator>::IntoIter;

    /// Produce an iterator over the blocks within the document, consuming the the Document in the process.
    fn into_iter(self) -> Self::IntoIter {
        self.sections.into_iter()
    }
}


/// Errors that the parser might encounter while reading the document.
#[derive(Debug)]
pub enum ParseError {

    /// Indicates that the PEG parser couldn't apply its rules to read the document.
    TokenizationError(String),

    /// Indicates that while the tokenization succeeded, the token sequence couldn't be read
    /// into an AST. 
    GrammarError(String),
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(value: pest::error::Error<Rule>) -> Self {
        ParseError::TokenizationError(value.to_string())
    }
}

impl From<ParseIntError> for ParseError {
    fn from(value: ParseIntError) -> Self {
        ParseError::GrammarError(value.to_string())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenizationError(msg) => write!(f, "GrammarError: {msg}"),
            Self::GrammarError(msg) => write!(f, "MalformedInput: {msg}"),
        }
    }
}

#[derive(Parser)]
#[grammar = "markdown.pest"]
struct MarkdownParser;

pub fn parse_document<'a>(input: &'a str) -> Result<Document<'a>, ParseError> {
    let raw_tokens = MarkdownParser::parse(Rule::document, input)?;
    let parsed_sections: Result<Vec<Block<'a>>, ParseError> = raw_tokens.into_iter()
        .filter_map(|section| parse_section(section).transpose())
        .collect();
    Ok(Document { sections: parsed_sections? } )
}

fn parse_section<'a>(root: Pair<'a, Rule>) -> Result<Option<Block<'a>>, ParseError> {
    match root.as_rule() {
        Rule::paragraph => Ok(Some(Block::Paragraph(inlines_from_pairs(root.into_inner())?))),
        Rule::codeblock => codeblock_from_pairs(root.into_inner()),
        Rule::verbatim => Ok(Some(Block::Verbatim(inlines_from_pairs(root.into_inner())?))),
        Rule::header => header_from_pairs(root.into_inner()).map(|x| Some(x)),
        Rule::bullet_list => Ok(Some(Block::BulletList(list_items_from_pairs(root.into_inner())?))),
        Rule::ordered_list => Ok(Some(Block::OrderedList(list_items_from_pairs(root.into_inner())?))),
        Rule::EOI | Rule::COMMENT => Ok(None),
        ty => Err(ParseError::TokenizationError(format!("Section type not implemented yet: {ty:?}\n\t{root:?}"))),
    }
}

fn list_items_from_pairs<'a>(root: Pairs<'a, Rule>) -> Result<Vec<ListItem<'a>>, ParseError> {
    root.into_iter()
        .enumerate()
        .map(|(index, list_item)| list_item_from_pairs(list_item.into_inner(), index as i32))
        .collect()
}

fn list_item_from_pairs<'a>(mut root: Pairs<'a, Rule>, index: i32) -> Result<ListItem<'a>, ParseError> {
    let item_marker = root.next()
        .ok_or(ParseError::GrammarError(format!("No contents found when parsing list item: {root:?}")))?;
    let index = match item_marker.as_rule() {
        Rule::bullet => index + 1,
        Rule::enumerator => std::cmp::max(item_marker.as_str().parse()?, index + 1), // Max to handle the case where the same number is used for each item
        _ => return Err(ParseError::GrammarError(format!("Unexpected node found when parsing list item demarcator: {item_marker:?}"))),
    };

    let first_node = match root.next() {
        Some(n) => n,
        None => return Ok(ListItem::default())
    };

    if first_node.as_rule() == Rule::list_content_continuation {
        let block_items: Result<Vec<Block<'a>>, ParseError> = first_node.into_inner()
            .filter_map(|section| parse_section(section).transpose())
            .collect();
        return Ok(ListItem { inline_text: None, block_items: Some(block_items?), index })
    }

    let inline_text = Some(inlines_from_pairs(first_node.into_inner())?);
    let block_items = match root.next() {
        Some(pair) => {
            let parsed_content: Result<Vec<Block<'a>>, ParseError> = pair.into_inner()
            .flat_map(|section| parse_section(section).transpose())
            .collect();
            Some(parsed_content?)
        },
        None => None,
    };

    Ok(ListItem { inline_text, block_items, index })
}

fn codeblock_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<Option<Block<'a>>, ParseError> {
    let lang = root
        .next()
        .map(|n| n.as_str())
        .filter(|lang| !lang.is_empty());
    let body = root
        .next()
        .ok_or(ParseError::GrammarError(format!("Failed to find body for paragraph in {root:?}")))?
        .as_str();
    Ok(Some(Block::Codeblock(lang, body)))
}

fn header_from_pairs<'a>(mut root: Pairs<'a, Rule>) -> Result<Block<'a>, ParseError> {
    // While each of the find_map calls do advance the iterator, the nodes must be in this order for the parse to be valid
    // so it's safe mutate
    let hashes = root
        .find_map(|n| if n.as_rule() == Rule::header_hashes { Some(n.as_str()) } else { None })
        .ok_or(ParseError::GrammarError(format!("Failed to find hashes for header in: {root:?}")))?;
    let title = root
        .find_map(|n| if n.as_rule() == Rule::header_title { Some(n.as_str()) } else { None })
        .ok_or(ParseError::GrammarError(format!("Failed to find title for header in {root:?}")))?;
    Ok(Block::Heading(hashes.len() as i32, title))

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
        ty => Err(ParseError::TokenizationError(format!("Text rule not implemented yet: {ty:?}\n\t{node:?}"))),
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
                .ok_or(ParseError::GrammarError(format!("Failed to find alt text for link in {inner:?}")))?
        ),
        rule => return Err(ParseError::GrammarError(format!("Unexpected rule found while attempting to parse a link: {rule:?}\n\t{inner:?}"))),
    };

    let uri = inner.next()
        .map(|n| n.as_str())
        .ok_or(ParseError::GrammarError(format!("URI missing from link {inner:?}")))?;

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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_one, &vec![
            Text::Plain("This is the first paragraph."),
            Text::Plain("This line is still part of that paragraph.")
        ]);

       let paragraph_two = match &document.sections[1] {
            Block::Paragraph(styled_contents) => styled_contents,
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
            Block::Paragraph(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_one, &vec![
            Text::Plain("This is the first paragraph."),
        ]);

       let paragraph_two = match &document.sections[1] {
            Block::Verbatim(styled_contents) => styled_contents,
            section => panic!("Unexpected section type: {section:?}"),
        };

        assert_eq!(paragraph_two, &vec![
            Text::Plain("This is a block quote."),
        ]); 

        let paragraph_two = match &document.sections[2] {
            Block::Verbatim(styled_contents) => styled_contents,
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
            Block::Codeblock(lang, body) => {
                assert_eq!(*lang, Some("python"));
                assert_eq!(*body, "print('hello world')")
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[1] {
            Block::Codeblock(lang, body) => {
                assert_eq!(*lang, Some("sh"));
                assert_eq!(*body, "    echo 'hello world'");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[2] {
            Block::Codeblock(lang, body) => {
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
            Block::Paragraph(items) => {
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
            Block::Heading(level, titles) => {
                assert_eq!(*level, 1);
                assert_eq!(*titles, "Header 1");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };

        match &document.sections[1] {
            Block::Heading(level, titles) => {
                assert_eq!(*level, 4);
                assert_eq!(*titles, "Header 4");
            },
            section => panic!("Unexpected section type: {section:?}"),
        };
        match &document.sections[2] {
            Block::Paragraph(items) => {
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
            
            3. An ordered list
            * Now this is a new bullet list
        "}) {
            Ok(d) => d,
            Err(e) => panic!("Failed to parse document: {e}")
        };

        match &document.sections[0] {
            Block::BulletList(list) => {
                assert_eq!(list, &vec![
                    ListItem {
                        index: 1,
                        inline_text: Some(vec![
                            Text::Plain("List item one"),
                            Text::Plain("This is a continuation of the last block")
                        ]),
                        block_items: Some(vec![
                            Block::Verbatim(vec![Text::Plain("This quote is in li 1")]),
                            Block::BulletList(vec![ 
                                ListItem { index: 1, inline_text: Some(vec![Text::Plain("A sublist")]), block_items: None } 
                            ])
                        ])
                    },
                    ListItem {
                        index: 2,
                        inline_text: Some(vec![Text::Plain("List item two")]),
                        block_items: None,
                    },
                    ListItem {
                        index: 3,
                        inline_text: Some(vec![Text::Plain("Lists can span blank lines")]),
                        block_items: None,
                    },
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[1] {
            Block::Paragraph(contents) => {
                assert_eq!(contents, &vec![Text::Plain("This text is not in the list")])
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[2] {
            Block::OrderedList(list) => {
                assert_eq!(list, &vec![
                    ListItem { index: 3, inline_text: Some(vec![Text::Plain("An ordered list")]), block_items: None }
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }

        match &document.sections[3] {
            Block::BulletList(list) => {
                assert_eq!(list, &vec![
                    ListItem { index: 1, inline_text: Some(vec![Text::Plain("Now this is a new bullet list")]), block_items: None }
                ]);
            },
            section => panic!("Unexpected section type: {section:?}"),
        }
    }
}