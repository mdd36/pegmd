
#[derive(Debug, PartialEq)]
pub enum Text<'a> {
    Plain(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    Underline(&'a str),
    Strikethrough(&'a str),
    Link(&'a str, &'a str),
    InlineCode(&'a str),
    Linebreak,
}

#[derive(Debug)]
pub struct ListElement<'a> {
    sublist: Option<Box<List<'a>>>,
    text: Vec<Text<'a>>,
}

#[derive(Debug)]
pub struct List<'a> {
    ordered: bool,
    elements: Vec<ListElement<'a>>,
}

#[derive(Debug)]
pub enum Section<'a> {
    Heading(u8, &'a str),
    Paragraph(Vec<Text<'a>>),
    List(List<'a>),
    Verbatim(Vec<Text<'a>>),
    Codeblock(Option<&'a str>, &'a str),
    Image(&'a str, &'a str)
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

mod grammar {
    use super::*;

    peg::parser!{
        pub grammar markdown_parser() for str {

            // ----------------- Overall document -----------------

            /// Main entrypoint for the parser.
            pub rule document() -> Document<'input> = blank_line()* sections:section()* blank_line()* eof() { Document::new(sections) }

            /// Parse a single section of the document. This might be a paragraph, block quote, list, etc
            rule section() -> Section<'input> = blank_line()* txt:section_contents() blank_line()* { txt }

            /// Basically a union type over the types of sections just to clean up the section rule and provide priority.
            /// Priority is actually backwards from what's reported in the docs, so items higher up in the list have higher priority.
            rule section_contents() -> Section<'input> = precedence! {
                i:image() { i }
                cb:code_block() { cb }
                h:header() { h }
                --
                v:verbatim() { v }
                --
                p:paragraph() { p }
            }

            // ----------------- Primitive patterns -----------------

            /// The end of file token
            rule eof() = ![_]

            /// Empty space
            rule _() -> &'input str = $[' ' | '\t']

            /// A carriage return or line feed
            rule new_line() = ['\n' | '\r'] { () }

            /// A line with only empty space
            rule blank_line() = _* new_line()

            /// 4 spaces or a tab makes an indent, causing a sublist or verbatim block
            rule indent() -> &'input str = $(" "*<4> / ['\t'])

            /// 3 or fewer spaces. At four spaces, we'll treat the spaces as a verbatim directive instead
            rule non_indent_space() -> &'input str = $(" "*<1,3>)

            /// Special characters. Will not match the special character if it's escaped with a backslash
            rule special_char() -> &'input str = !"\\" ch:$(['*'|'_'|'`'|'&'|'['|']'|'('|')'|'<'|'!'|'#'|'\\'|'\''|'"']) { ch }
            
            /// Basic characters
            rule normal_char() -> &'input str = !_ !new_line() !special_char() ch:$[_] { ch }

            /// An asterisk without a preceding backslash to escape it. This will be interpreted as a control directive
            rule unescaped_asterisk() -> &'input str = !"\\" txt:$"*" { txt }

            /// A tilde without a preceding backslash to escape it.
            rule unescaped_tilde() -> &'input str = !"\\" txt:$"~" { txt }

            /// A backtick without a preceding backslash to escape it.
            rule unescaped_backtick() -> &'input str = !"\\" txt:$"`" { txt }

            /// A plaintext string. Must start with a non-space character and cannot contain special characters.
            rule str() -> Text<'input> = txt:$((!markup() !endline() !new_line() [_])+) { Text::Plain(txt) }

            /// Stylized text rule
            rule markup() -> Text<'input> = strong() 
                / italic()
                / strikethrough()
                / inline_code()
                / underline()
                / link()

            /// Some inline text, potentially with style
            rule inline() -> Text<'input> = str() / markup()

            /// A collection of inline fields, potentially with rendered and un-rendered linebreaks in the markdown source
            rule inlines() -> Vec<Text<'input>> = inline_elems:(!endline() txt:inline() { Some(txt) } / e:endline() &inline() { e })+ endline()? {
                inline_elems.into_iter()
                .filter_map(|line| line)
                .collect::<Vec<Text<'input>>>()
            }

            /// A linebreak within a segment. This doesn't make a new segment, and may not even result in a line break within the section.
            rule endline() -> Option<Text<'input>> = normal_endline() { None } / tailing_endline() { None } / e:linebreak() { Some(e) }
            
            /// A single new line not followed by either a blank line or a quote block. Shouldn't render a linebreak in the final output.
            rule normal_endline() = _? new_line() !blank_line() _*

            /// Matches the last line before the EoF. Previous trailing lines are matched as blank lines.
            rule tailing_endline() = _* new_line() eof()

            /// Two or more spaces before a newline will insert a linebreak.
            rule linebreak() -> Text<'input> = " "*<2,> normal_endline() { Text::Linebreak }

            /// Matches text in an alt box for a link
            rule alt_text() -> &'input str = $((!"]" !new_line() [_])+)

            /// Matches a URL in a link block
            rule url() -> &'input str = $((![')' | '>'] !_ !new_line() [_])+)

            // ----------------- Paragraph sections -----------------
            
            /// Entrypoint for a paragraph section
            rule paragraph() -> Section<'input> = non_indent_space()? txt:inlines() (blank_line()+ / eof()) { Section::Paragraph(txt) }

            // ----------------- Image sections -----------------

            /// Embedded image
            rule image() -> Section<'input> = _* "![" alt:alt_text() "](" src:url() ")" { Section::Image(alt, src) }

            // ----------------- Header sections -----------------

            rule header() -> Section<'input> = _* hashes:header_hash() _* title:$((!new_line() [_])+) { Section::Heading(hashes.len() as u8, title) }

            rule unescaped_hash() -> &'input str = !"\\" txt:$"#" { txt }

            rule header_hash() -> &'input str = _* hashes:$(unescaped_hash()*<1,6>) _* { hashes }

            // ----------------- Block quote or verbatim sections -----------------

            rule verbatim() -> Section<'input> = (indent()+ / (_* ">")) _* txt:inlines() { Section::Verbatim(txt) }
            
            // ----------------- Codeblock sections -----------------

            /// Create a code block. 
            /// TODO this probably has some funky indent things going on, may need to improve the leading space grammar
            rule code_block() -> Section<'input> = codeblock_open() lang:codeblock_language()? new_line() body:$codeblock_body() codeblock_close() {
                 Section::Codeblock(lang, body) 
            }

            /// Three backticks to open the block
            rule codeblock_open() = _* unescaped_backtick()*<3> _*

            rule codeblock_language() -> &'input str = lang:$((!new_line() !_ [_])+) _* { lang }

            rule codeblock_body() -> &'input str = $((!codeblock_close() [_])+)

            /// Three backticks and a newline to end the block
            rule codeblock_close() = new_line()? _* unescaped_backtick()*<3> _* new_line()?

            // ----------------- Bold control directives -----------------

            /// Find the text that should be bolded.
            rule strong() -> Text<'input> = two_star_open() txt:$((!two_star_close() !new_line() [_])+) two_star_close() { Text::Bold(txt) }
            
            /// Opening for a bold text block
            rule two_star_open() = unescaped_asterisk()*<2> !_ !new_line()

            /// Closing for a bold text block
            rule two_star_close() = !_ !new_line() unescaped_asterisk()*<2>

            // ----------------- Italic control directives -----------------

            /// Find the text that should be italicized
            rule italic() -> Text<'input> = one_star_open() txt:$((!one_star_close() !new_line() [_])+) one_star_close() { Text::Italic(txt) }

            /// Opening for an italic text block
            rule one_star_open() = unescaped_asterisk() !_ !new_line() 

            /// Closing for an italic text block
            rule one_star_close() = !_ !new_line() unescaped_asterisk()

            // ----------------- Strikethrough control directives -----------------

            /// Find the text that should be struck through 
            rule strikethrough() -> Text<'input> = tilde_open() txt:$((!tilde_close() !new_line() [_])+) tilde_close() { Text::Strikethrough(txt) }

            /// Opening for an struck through text block
            rule tilde_open() = unescaped_tilde()*<2> !_ !new_line() 

            /// Closing for an struck through text block
            rule tilde_close() = !_ !new_line() unescaped_tilde()*<2>

            // ----------------- Inline code control directives -----------------

            /// Find the inline code text
            rule inline_code() -> Text<'input> = inline_code_open() txt:$((!inline_code_close() !new_line() [_])+) inline_code_close() { Text::InlineCode(txt) }

            /// Opening for inline code
            rule inline_code_open() = unescaped_backtick() !_ !new_line() 

            /// Closing for inline code
            rule inline_code_close() = !_ !new_line() unescaped_backtick()


            // ----------------- Underline code control directives -----------------

            /// Find the inline code text
            rule underline() -> Text<'input> = one_underscore_open() txt:$((!two_underscore_close() !new_line() [_])+) two_underscore_close() { Text::Underline(txt) }

            rule unescaped_underscore() = !"\\" "_"

            /// Opening for an underline block
            rule one_underscore_open() = unescaped_underscore() !_ !new_line() 

            /// Closing for and underline block
            rule two_underscore_close() = !_ !new_line() unescaped_underscore()

            // ----------------- Link control directives -----------------

            rule link() -> Text<'input> = directed_link() / auto_link()
            
            rule directed_link() -> Text<'input> = "[" display:alt_text() "](" href:url() ")"  { Text::Link(display, href) }

            rule auto_link() -> Text<'input> = "<" href:url() ">" { Text::Link(href, href) }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Section, Text};
    use super::grammar::markdown_parser; 

    #[test]
    pub fn basic_bold_test() {
        let document = match markdown_parser::document("A basic test that **bold** text renders. Ignores **bold\n** if a line break occurs.") {
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
            Text::Plain(" text renders. Ignores **bold"),
            Text::Plain("** if a line break occurs."),
        ]);
    }

    #[test]
    pub fn basic_italic_test() {
        let document = match markdown_parser::document("A basic test that *italic* text renders. Ignores *italic\n* if a line break occurs.") {
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
        let document = match markdown_parser::document("A basic test that ~~strike through~~ text renders. Ignores ~~strike through\n~~ if a line break occurs.") {
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
        let document = match markdown_parser::document("A basic test that _underline_ text renders. Ignores _underline\n_ if a line break occurs.") {
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
        let document = match markdown_parser::document("A basic test that `code` text renders. Ignores `code\n` if a line break occurs.") {
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
        let document = match markdown_parser::document("A simple test that [links](www.google.com) to <www.wikipedia.com> render.") {
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
        let document = match markdown_parser::document("Some **intermixed** *styles* like _underlines_, [links](www.google.com), and `code`.") {
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
        let document = match markdown_parser::document("This is the first paragraph.\nThis line is still part of that paragraph.\n\n  This is a new paragraph.  \nIt has different lines within it.") {
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
}