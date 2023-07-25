use pest::iterators::{Pair, Pairs};

use crate::{container_type, error::ParseError, first_child, leaf_type, parser::Rule};

/// A newtype wrapper over a Vec<Node>, largely so that we can implement conversion traits
/// between a [`Pair`] and a Vec. This type implements [`std::ops::Deref`] to its wrapped
/// vector to improve developer ergonomics.
#[derive(PartialEq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Children<'input>(
    #[cfg_attr(feature = "serde_support", serde(borrow))] Vec<Node<'input>>,
);

impl<'input> TryFrom<Pair<'input, Rule>> for Children<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_str();
        let span_start = value.as_span().start();
        Self::try_from_pairs(value.into_inner(), span, span_start)
    }
}

impl<'input> std::ops::Deref for Children<'input> {
    type Target = Vec<Node<'input>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'input> std::ops::DerefMut for Children<'input> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'input> std::fmt::Debug for Children<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'input> Children<'input> {
    /// This method is a little gross, but it handles coalescing adjacent plaintext parser tokens
    /// into a single [`Node::Text`], as a naïve implementation would produce a single text node
    /// for every word, space, special character, and escaped control character. By collapsing them
    /// into a single Node, we reduce the memory footprint of the final AST and make it easier
    /// for traversal implementations to reason about the nodes in the tree.
    pub fn try_from_pairs(
        pairs: Pairs<'input, Rule>,
        span: &'input str,
        start_index: usize,
    ) -> Result<Self, ParseError> {
        // Represents the sliding window over the &str that only contains plaintext.
        let mut running_segment_start = start_index;
        let mut running_segment_end = start_index;

        let mut children = Vec::new();

        for child in pairs {
            let child_start = child.as_span().start();
            let child_end = child.as_span().end();

            if child.as_rule().is_plaintext() {
                // If the child's start is after the running segment's end, then
                // the child is the start of a new run so we need to update the
                // start pos of the running segment.
                if child_start > running_segment_end {
                    running_segment_start = child_start;
                }
                // Always update the end since we always want to have the running
                // segment include this span
                running_segment_end = child_end;

                // Skip! Will add the running segment later when we hit a non-
                // plaintext node.
                continue;
            }

            // Not plaintext, so we immediately convert the child into a node.
            let node = Node::try_from(child)?;

            // If theses aren't equal, then we have plaintext to add.
            if running_segment_start != running_segment_end {
                // Convert from absolute position in the input to the absolute position within the span
                let start_index = running_segment_start - start_index;
                let end_index = start_index + (running_segment_end - running_segment_start);
                children.push(Node::Text(Text {
                    literal: &span[start_index..end_index],
                }));
            }

            // Now, we can push the non-plaintext node that came after the stretch of plaintext.
            children.push(node);

            // And reset the running segment
            running_segment_start = child_end;
            running_segment_end = child_end;
        }

        // Last check in case the final segment of the span was plaintext.
        if running_segment_start != running_segment_end {
            // Convert from absolute position in the input to the absolute position within the span
            let start_index = running_segment_start - start_index;
            let end_index = start_index + (running_segment_end - running_segment_start);
            children.push(Node::Text(Text {
                literal: &span[start_index..end_index],
            }));
        }

        Ok(Children(children))
    }
}

// Create all the different AST node types. See the macros.rs file for how they're defined
// and what traits are automatically implemented.
container_type!(Document);
container_type!(Paragraph);
container_type!(BlockQuote);
container_type!(List, (tight, bool), (ordered, bool), (start, u32));
container_type!(ListItem, (index, u32));
container_type!(Emphasis);
container_type!(Strong);
container_type!(Label);
container_type!(Code);
container_type!(CodeBlock, (language, Option<&'input str>));
container_type!(Heading, (level, u8));
container_type!(Link, (source, &'input str));
leaf_type!(Text);
leaf_type!(Linebreak);
leaf_type!(SoftLinebreak);
leaf_type!(Image, (source, &'input str));

/// for that type, except for EOI since EOI contains nothing by definition.
///
/// ### Lifetime Parameters
///
/// * `'input` - The lifetime is constrained to the lifetime of the input to the parser
///              since leaf nodes like Text contain a string slice from the original input.
#[derive(Debug, PartialEq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Deserialize, serde::Serialize)
)]
pub enum Node<'input> {
    // Containers
    #[cfg_attr(feature = "serde_support", serde(borrow))]
    Document(Document<'input>),
    Paragraph(Paragraph<'input>),
    BlockQuote(BlockQuote<'input>),
    Heading(Heading<'input>),
    List(List<'input>),
    ListItem(ListItem<'input>),
    CodeBlock(CodeBlock<'input>),
    Emphasis(Emphasis<'input>),
    Strong(Strong<'input>),
    Label(Label<'input>),
    Link(Link<'input>),
    // Leaves
    Image(Image<'input>),
    Text(Text<'input>),
    Linebreak(Linebreak<'input>),
    SoftLinebreak(SoftLinebreak<'input>),
    Code(Code<'input>),
    // End of input
    EOI,
}

impl<'input> Node<'input> {
    pub fn children(&self) -> Option<&Children> {
        match self {
            Self::Document(c) => Some(c.children()),
            Self::Paragraph(p) => Some(p.children()),
            Self::BlockQuote(bq) => Some(bq.children()),
            Self::Heading(h) => Some(h.children()),
            Self::List(l) => Some(l.children()),
            Self::ListItem(li) => Some(li.children()),
            Self::CodeBlock(cb) => Some(cb.children()),
            Self::Emphasis(emp) => Some(emp.children()),
            Self::Strong(strong) => Some(strong.children()),
            Self::Label(l) => Some(l.children()),
            Self::Link(l) => Some(l.children()),
            Self::Code(c) => Some(c.children()),
            Self::Image(_) => None,
            Self::Text(_) => None,
            Self::Linebreak(_) => None,
            Self::SoftLinebreak(_) => None,
            Self::EOI => None,
        }
    }

    pub fn children_mut(&'input mut self) -> Option<&mut Children> {
        match self {
            Self::Document(c) => Some(c.children_mut()),
            Self::Paragraph(p) => Some(p.children_mut()),
            Self::BlockQuote(bq) => Some(bq.children_mut()),
            Self::Heading(h) => Some(h.children_mut()),
            Self::List(l) => Some(l.children_mut()),
            Self::ListItem(li) => Some(li.children_mut()),
            Self::CodeBlock(cb) => Some(cb.children_mut()),
            Self::Emphasis(emp) => Some(emp.children_mut()),
            Self::Strong(strong) => Some(strong.children_mut()),
            Self::Label(l) => Some(l.children_mut()),
            Self::Link(l) => Some(l.children_mut()),
            Self::Code(c) => Some(c.children_mut()),
            Self::Image(_) => None,
            Self::Text(_) => None,
            Self::Linebreak(_) => None,
            Self::SoftLinebreak(_) => None,
            Self::EOI => None,
        }
    }

    pub fn as_span(&self) -> &str {
        match self {
            Self::Document(c) => c.as_span(),
            Self::Paragraph(p) => p.as_span(),
            Self::BlockQuote(bq) => bq.as_span(),
            Self::Heading(h) => h.as_span(),
            Self::List(l) => l.as_span(),
            Self::ListItem(li) => li.as_span(),
            Self::CodeBlock(cb) => cb.as_span(),
            Self::Emphasis(emp) => emp.as_span(),
            Self::Strong(strong) => strong.as_span(),
            Self::Label(l) => l.as_span(),
            Self::Link(l) => l.as_span(),
            Self::Code(c) => c.as_span(),
            Self::Image(img) => img.as_span(),
            Self::Text(txt) => txt.as_span(),
            Self::Linebreak(lb) => lb.as_span(),
            Self::SoftLinebreak(slb) => slb.as_span(),
            Self::EOI => "EOI",
        }
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for Node<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let pair_as_str = value.as_str();

        match value.as_rule() {
            // Container nodes
            Rule::document => Ok(Node::Document(Document::try_from(value)?)),
            Rule::paragraph => Ok(Node::Paragraph(Paragraph::try_from(value)?)),
            Rule::verbatim => Ok(Node::BlockQuote(BlockQuote::try_from(value)?)),
            Rule::header => Ok(Node::Heading(Heading::try_from(value)?)),
            Rule::bullet_list | Rule::ordered_list => Ok(Node::List(List::try_from(value)?)),
            Rule::star_bullet_item_tight
            | Rule::star_bullet_item
            | Rule::dash_bullet_item_tight
            | Rule::dash_bullet_item
            | Rule::plus_bullet_item_tight
            | Rule::plus_bullet_item
            | Rule::period_ordered_list_item_tight
            | Rule::period_ordered_list_item
            | Rule::parenthesis_ordered_list_item_tight
            | Rule::parenthesis_ordered_list_item => Ok(Node::ListItem(ListItem::try_from(value)?)),
            Rule::fenced_codeblock | Rule::indented_codeblock => {
                Ok(Node::CodeBlock(CodeBlock::try_from(value)?))
            }
            Rule::emphasis => Ok(Node::Emphasis(Emphasis::try_from(value)?)),
            Rule::strong => Ok(Node::Strong(Strong::try_from(value)?)),
            Rule::label => Ok(Node::Label(Label::try_from(value)?)),
            Rule::link => Ok(Node::Link(Link::try_from(first_child!(
                value.into_inner()
            )?)?)),
            Rule::image => Ok(Node::Image(Image::try_from(first_child!(
                value.into_inner()
            )?)?)),
            Rule::code => Ok(Node::Code(Code::try_from(value)?)),
            // Leaf nodes
            Rule::str
            | Rule::space
            | Rule::symbol
            | Rule::escaped_special_char
            | Rule::source
            | Rule::linebreak_literal => Ok(Node::Text(Text::from(value))),
            Rule::linebreak => Ok(Node::Linebreak(Linebreak::from(value))),
            Rule::normal_endline | Rule::blockquote_linebreak => {
                Ok(Node::SoftLinebreak(SoftLinebreak::from(value)))
            }
            // End of input
            Rule::EOI => Ok(Node::EOI),
            // Error
            ty => Err(ParseError::SyntaxError(format!(
                r#"Failed to find a node to represent "{pair_as_str}" as a {ty:?}. Error occurred at: {location:?}"#
            ))),
        }
    }
}

// -----------------------------------------------------------------------
// While most of the types can have their TryFrom<Pair<'input, Rule>>
// implementations generated by the macro, the ones that need to parse out
// extra information like header level, ordered vs. bullet list, etc, need
// to have their  implementations defined manually.
// -----------------------------------------------------------------------
impl<'input> TryFrom<Pair<'input, Rule>> for List<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let span: &str = value.as_str();

        let ordered = match value.as_rule() {
            Rule::bullet_list => false,
            Rule::ordered_list => true,
            ty => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected a list node for "{span}", but got {ty:?}. Error occurred at {location:?}"#
                )))
            }
        };

        let list = first_child!(value.into_inner())?;
        let tight = match list.as_rule() {
            Rule::list_tight => true,
            Rule::list_loose => false,
            ty => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected a list node for "{span}", but got {ty:?}. Error occurred at {location:?}"#
                )))
            }
        };

        let children = Children::try_from(list)?;

        let start = match &children[0] {
            Node::ListItem(list_item) => std::cmp::max(1, list_item.index()),
            _ => 0,
        };

        Ok(Self {
            children,
            span,
            tight,
            ordered,
            start,
        })
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for ListItem<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_str();
        let location = value.line_col();

        let mut list_item_pairs = value.into_inner();
        let bullet_or_enumerator = match list_item_pairs.next() {
            Some(pair) => pair,
            None => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected a list marker in "{span}", found nothing. Error occurred at {location:?}"#
                )));
            }
        };

        let index = match bullet_or_enumerator.as_rule() {
            Rule::list_index => bullet_or_enumerator.as_str().parse()?,
            Rule::dash_bullet | Rule::star_bullet | Rule::plus_bullet => 1,
            other => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected a bullet or list index in {span}, but found {other:?}. Error occurred at {location:?}"#
                )));
            }
        };

        let children = match list_item_pairs.next() {
            Some(pair) => Children::try_from(pair)?,
            None => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected list contents in "{span}", but found none. Error occurred at {location:?}"#
                )));
            }
        };

        Ok(Self {
            children,
            span,
            index,
        })
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for CodeBlock<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let start_pos = value.as_span().start();
        let span = value.as_str();

        let mut pairs = match value.as_rule() {
            Rule::indented_codeblock => {
                return Ok(Self {
                    span,
                    language: None,
                    children: Children::try_from(value)?,
                })
            }
            Rule::fenced_codeblock => value.into_inner(),
            other => {
                return Err(ParseError::SyntaxError(format!(
                    r#"Expected a codeblock type in "{span}", but found {other:?}. Error occurred at {location:?}"#
                )))
            }
        };

        let language = match pairs.peek().map(|node| node.as_rule()) {
            Some(Rule::info_string_language) => Some(pairs.next().unwrap().as_str()),
            _ => None,
        };

        let children = Children::try_from_pairs(pairs, span, start_pos)?;

        Ok(Self {
            span,
            language,
            children,
        })
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for Heading<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let start_index = value.as_span().start();
        let span = value.as_str();

        let mut children = value.into_inner();
        let hashes = children.next().ok_or(ParseError::SyntaxError(format!(
            r#"No header markers found in "{span}". Error occurred at: {location:?}"#
        )))?;

        let level = hashes.as_str().len() as u8;
        let children = Children::try_from_pairs(children, span, start_index)?;

        Ok(Self {
            children,
            span,
            level,
        })
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for Link<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let span = value.as_str();

        let mut inner_nodes = value.into_inner();

        // All links have labels. Autolinks are a special case where their label
        // is the same as their source.
        let label_node = inner_nodes.next().ok_or(ParseError::SyntaxError(format!(
            r#"No label node found in "{span}". Error occurred at: {location:?}"#
        )))?;

        // If this is an autolink, there's no more children and so hence the source
        // is the text value of the label node. If this is a directed link or reference,
        // then the next node is the actual destination.
        let source = match inner_nodes.next() {
            Some(node) => node.as_str(),
            None => label_node.as_str(),
        };

        // Now we can do this since we've extracted the source as a str in the case where
        // the link is an autolink.
        let children = Children::try_from(label_node)?;

        Ok(Self {
            children,
            span,
            source,
        })
    }
}

impl<'input> TryFrom<Pair<'input, Rule>> for Image<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let link_as_str = value.as_str();

        let mut children = value.into_inner();
        let alt = children
            .next()
            .ok_or(ParseError::SyntaxError(format!(
                r#"No label node found in "{link_as_str}". Error occurred at: {location:?}"#
            )))?
            .as_str();
        let source = children
            .next()
            .ok_or(ParseError::SyntaxError(format!(
                r#"No source found for link in "{link_as_str}". Error occurred at: {location:?}"#
            )))?
            .as_str();

        Ok(Self {
            literal: alt,
            source,
        })
    }
}
