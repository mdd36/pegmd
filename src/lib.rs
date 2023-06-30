
#[derive(Debug)]
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
pub enum Section<'a> {
    Paragraph(Vec<Text<'a>>),
    UnorderedList(Vec<Text<'a>>),
    OrderedList(Vec<Text<'a>>),
    Verbatium(Vec<Text<'a>>),
    Codeblock(String, Vec<Text<'a>>),
}

#[derive(Debug)]
pub struct Document<'a> {
    sections: Vec<Section<'a>>
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

            // ----------------- Primitive patterns -----------------

            /// The end of file token
            rule eof() -> () = ![_]

            /// Contiguous empty space
            rule _() -> &'input str = $[' ' | '\t']

            /// A carriage return or line feed
            rule new_line() -> () = ['\n' | '\r'] { () }

            /// A line with only empty space
            rule blank_line() -> () = _* new_line()

            /// 3 or fewer spaces. At four spaces, we'll treat the spaces as a verbatim directive instead
            rule non_indent_space() -> Text<'input> = txt:$(" "*<1,3>) { Text::Plain(txt) }

            /// Special characters. Will not match the special character if it's escaped with a backslash
            rule special_char() -> &'input str = !"\\" ch:$(['*'|'_'|'`'|'&'|'['|']'|'('|')'|'<'|'!'|'#'|'\\'|'\''|'"']) { ch }
            
            /// Basic characters
            rule normal_char() -> &'input str = !_ !new_line() !special_char() ch:$[_] { ch }

            /// An asterisk without a preceding backslash to escape it. This will be interpreted as a control directive
            rule unescaped_asterisk() -> () = !"\\" "*"

            /// A plaintext string. Must start with a non-space character and cannot contain special characters.
            rule str() -> Text<'input> = !_ !new_line() txt:$((!markup() !endline() [_])+) { Text::Plain(txt) }

            /// Stylized text rule
            rule markup() -> Text<'input> = strong() / italic()

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
            rule normal_endline() -> () = _? new_line() !blank_line() !(_* ">")

            /// Matches the last line before the EoF. Previous trailing lines are matched as blank lines.
            rule tailing_endline() -> () = _* new_line() eof()

            /// Two or more spaces before a newline will insert a linebreak.
            rule linebreak() -> Text<'input> = " "*<2,> normal_endline() { Text::Linebreak }

            // ----------------- Overall document -----------------

            /// Main entrypoint for the parser.
            pub rule document() -> Document<'input> = blank_line()* sections:section()* blank_line()* eof() { Document::new(sections) }

            /// Parse a single section of the document. This might be a paragraph, block quote, list, etc
            rule section() -> Section<'input> = blank_line()* txt:section_contents() blank_line()* { txt }

            /// Basically a union type over the types of sections just to clean up the section rule.
            rule section_contents() -> Section<'input> = paragraph() /* / other / types */

            // ----------------- Paragraph sections -----------------
            
            /// Entrypoint for a paragraph section
            rule paragraph() -> Section<'input> = non_indent_space()? txt:inlines() (blank_line()+ / eof()) { Section::Paragraph(txt) }

            // ----------------- Bold control directives -----------------

            /// Find the text that should be bolded.
            rule strong() -> Text<'input> = two_star_open() txt:$((!two_star_close() [_])+) two_star_close() { Text::Bold(txt) }
            
            /// Opening for a bold text block
            rule two_star_open() -> () = unescaped_asterisk()*<2> !_ !new_line()

            /// Closing for a bold text block
            rule two_star_close() -> () = !_ !new_line() unescaped_asterisk()*<2>

            // ----------------- Italic control directives -----------------

            /// Find the text that should be italicized
            rule italic() -> Text<'input> = one_star_open() txt:$((!one_star_close() [_])+) one_star_close() { Text::Italic(txt) }

            /// Opening for an italic text block
            rule one_star_open() -> () = unescaped_asterisk() !_ !new_line() 

            /// Closing for an italic text block
            rule one_star_close() -> () = !_ !new_line() unescaped_asterisk()

        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn basic_bold_test() {
        let nodes = super::grammar::markdown_parser::document("with a new section!  \nwith a linebreak in it!");

        println!("{nodes:?}");
    }
}