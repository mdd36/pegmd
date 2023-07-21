use pest::iterators::Pair;

use crate::{container_type, leaf_type, parser::Rule, error::ParseError, first_child};

/// A NodeType wraps containers and leafs into a single type so that other
/// components can differentiate their behavior based on the whether the
/// node is a leaf, a container, or the end of the input. EOI could arguably 
/// be a leaf, but for greater explicitly it's modeled separately.
/// 
/// ### Lifetime Parameters
/// 
/// * `'input` - The lifetime is constrained to the lifetime of the input to the parser
///              since leaf nodes contain a string slice from the original input.
pub enum NodeType<'input> {
    Leaf,
    Container(&'input Vec<Node<'input>>),
    EOI,
}

/// A union type over all the different nodes. Each variant contains the struct representation
/// for that type, except for EOI since EOI contains nothing by definition.
/// 
/// ### Lifetime Parameters
/// 
/// * `'input` - The lifetime is constrained to the lifetime of the input to the parser
///              since leaf nodes like Text contain a string slice from the original input.
#[derive(Debug, PartialEq)]
pub enum Node<'input> {
    // Containers
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
    Code(Code<'input>),
    // End of input
    EOI
}

impl <'input> Node<'input> {
    pub fn inner(&'input self) -> NodeType<'input> {
        match self {
            Self::Document(c) => NodeType::Container(c.as_ref()),
            Self::Paragraph(p) => NodeType::Container(p.as_ref()), 
            Self::BlockQuote(bq) => NodeType::Container(bq.as_ref()),
            Self::Heading(h) => NodeType::Container(h.as_ref()),
            Self::List(l) => NodeType::Container(l.as_ref()),
            Self::ListItem(li) => NodeType::Container(li.as_ref()),
            Self::CodeBlock(cb) => NodeType::Container(cb.as_ref()),
            Self::Emphasis(emp) => NodeType::Container(emp.as_ref()),
            Self::Strong(strong) => NodeType::Container(strong.as_ref()),
            Self::Label(l) => NodeType::Container(l.as_ref()),
            Self::Link(l) => NodeType::Container(l.as_ref()),
            Self::Image(_) => NodeType::Leaf,
            Self::Text(_) => NodeType::Leaf,
            Self::Linebreak(_) => NodeType::Leaf,
            Self::Code(_) => NodeType::Leaf,
            Self::EOI => NodeType::EOI,
        }
    }
}

impl <'input> TryFrom<Pair<'input, Rule>> for Node<'input> {
    type Error = ParseError;

    fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let pair_as_str = value.as_str();

        match value.as_rule() {
            // Container nodes
            Rule::document => Ok(Node::Paragraph(Paragraph::try_from(value)?)),
            Rule::paragraph => Ok(Node::Paragraph(Paragraph::try_from(value)?)),
            Rule::verbatim => Ok(Node::BlockQuote(BlockQuote::try_from(value)?)),
            Rule::header=> Ok(Node::Heading(Heading::try_from(value)?)),
            Rule::bullet_list | Rule::ordered_list => Ok(Node::List(List::try_from(value)?)),
            Rule::list_item | Rule::list_item_tight => Ok(Node::ListItem(ListItem::try_from(value)?)),
            Rule::codeblock => Ok(Node::CodeBlock(CodeBlock::try_from(value)?)),
            Rule::emphasis => Ok(Node::Emphasis(Emphasis::try_from(value)?)),
            Rule::strong => Ok(Node::Strong(Strong::try_from(value)?)),
            Rule::label => Ok(Node::Label(Label::try_from(value)?)),
            Rule::link => Ok(Node::Link(Link::try_from(first_child!(value)?)?)),
            Rule::image => Ok(Node::Image(Image::try_from(first_child!(value)?)?)),
            // Leaf nodes
            Rule::str | Rule::space | Rule::symbol | Rule::escaped_special_char => Ok(Node::Text(Text::from(value))),
            Rule::linebreak => Ok(Node::Linebreak(Linebreak::from(value))),
            Rule::code => Ok(Node::Code(Code::from(value))),
            // End of input
            Rule::EOI => Ok(Node::EOI),
            // Error
            ty => Err(ParseError::SyntaxError(
                format!(r#"Failed to find a node to represent "{pair_as_str}" as a {ty:?}. Error occurred at: {location:?}"#)
            ))
        }
    }
}

// Create all the different AST node types. See the macros.rs file for how they're defined
// and what traits are automatically implemented.
container_type!(Document);
container_type!(Paragraph);
container_type!(BlockQuote);
container_type!(ListItem);
container_type!(CodeBlock);
container_type!(Emphasis);
container_type!(Strong);
container_type!(Label);
container_type!(List, (tight, bool), (ordered, bool));
container_type!(Heading, (level, u8));
container_type!(Link, (source, &'input str));
leaf_type!(Text);
leaf_type!(Linebreak);
leaf_type!(Code);
leaf_type!(Image, (source, &'input str));

// -----------------------------------------------------------------------
// While most of the types can have their TryFrom<Pair<'input, Rule>> 
// implementations generated by the macro, the ones that need to parse out 
// extra information like header level, ordered vs. bullet list, etc, need 
// to have their  implementations defined manually. 
// -----------------------------------------------------------------------
impl <'input> TryFrom<Pair<'input, Rule>> for List<'input> {
  type Error = ParseError;

  fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
      let list_as_str = value.as_str();
      let location = value.line_col();

      let ordered = match value.as_rule() {
          Rule::bullet_list => false,
          Rule::ordered_list => true,
          ty => return Err(ParseError::SyntaxError(
              format!(r#"Expected a list node for "{list_as_str}", but got {ty:?}. Error occurred at {location:?}"#)
          ))
      };

      let list = first_child!(value)?;
      let tight = match list.as_rule() {
          Rule::list_tight => true,
          Rule::list_loose => false,
          ty => return Err(ParseError::SyntaxError(
              format!(r#"Expected a list node for "{list_as_str}", but got {ty:?}. Error occurred at {location:?}"#)
          ))
      };

      let list_items: Result<Vec<Node<'input>>, ParseError> = list.into_inner()
        .map(Node::try_from)
        .collect();

      Ok( Self { children: list_items?, tight, ordered } )
  }
}

impl <'input> TryFrom<Pair<'input, Rule>> for Heading<'input> {

  type Error = ParseError;

  fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
      let location = value.line_col();
      let header_as_str = value.as_str();

      let mut children = value.into_inner();
      let hashes = children.next()
          .ok_or(ParseError::SyntaxError(format!(r#"No header markers found in "{header_as_str}". Error occurred at: {location:?}"#)))?;
      let title = children.next()
          .ok_or(ParseError::SyntaxError(format!(r#"No title found in "{header_as_str}". Error occurred at: {location:?}"#)))?;
      
      let level = hashes.as_str().len() as u8;
      let children = vec![Node::try_from(title)?];
      
      Ok(Self { children, level })
  }

}

impl <'input> TryFrom<Pair<'input, Rule>> for Link<'input> {
  type Error = ParseError;

  fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
      let location = value.line_col();
      let link_as_str = value.as_str();
      let link_type = value.as_rule();
      let mut children = value.into_inner();

      let (label, source) = match link_type {
          Rule::directed_link | Rule::full_reference_link | Rule::shortcut_reference_link => {
              let label = children.next()
                  .ok_or(ParseError::SyntaxError(format!(r#"No label node found in "{link_as_str}". Error occurred at: {location:?}"#)))?
                  .try_into()?;
              let source = children.next()
                  .ok_or(ParseError::SyntaxError(format!(r#"No source found for link in "{link_as_str}". Error occurred at: {location:?}"#)))?
                  .as_str();
              (label, source)
          }
          Rule::autolink => {
              let source = children.next()
                  .ok_or(ParseError::SyntaxError(format!(r#"No source found for link in "{link_as_str}". Error occurred at: {location:?}"#)))?;
              let source_str = source.as_str();
              (source.try_into()?, source_str)
          }
          ty => return Err(ParseError::SyntaxError(
              format!(r#"Expected source "{link_as_str}" to be a link type, but was {ty:?}. Error occurred at: {location:?}"#)
          ))
      };

      Ok( Self { children: vec![ label ], source } )
      
  }
} 

impl <'input> TryFrom<Pair<'input, Rule>> for Image<'input> {
  type Error = ParseError;

  fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
      let location = value.line_col();
      let link_as_str = value.as_str();

      let mut children = value.into_inner();
      let alt = children.next()
          .ok_or(ParseError::SyntaxError(format!(r#"No label node found in "{link_as_str}". Error occurred at: {location:?}"#)))?
          .as_str();
      let source = children.next()
          .ok_or(ParseError::SyntaxError(format!(r#"No source found for link in "{link_as_str}". Error occurred at: {location:?}"#)))?
          .as_str();

      Ok( Self { literal: alt, source } )
  }
}