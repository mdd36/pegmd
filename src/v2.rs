use std::ops::Deref;
use pest::{Parser, iterators::Pair};
use crate::{ParseError, parser::{MarkdownParser, Rule}};

macro_rules! first_child {
    ($value: expr) => {
        $value.into_inner()
            .next()
            .ok_or(ParseError::SyntaxError(format!("Missing required child in expression")))?
    };
}

#[derive(Debug)]
pub struct Leaf<'a> {
    literal: &'a str,
}

impl <'a> Deref for Leaf<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.literal
    }
}

impl <'a> From<Pair<'a, Rule>> for Leaf<'a> {
    fn from(value: Pair<'a, Rule>) -> Self  {
        Self { literal: value.as_str() }
    }
}

#[derive(Debug)]
pub struct Container<'a> {
    children: Vec<Node<'a>>,
}

impl <'a> Deref for Container<'a> {
    type Target = Vec<Node<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.children
    }
}

impl <'a> TryFrom<Pair<'a, Rule>> for Container<'a> {
    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let children: Result<Vec<Node>, ParseError> = value.into_inner()
            .map(|child| Node::try_from(child))
            .collect();

        Ok( Self { children: children? } )
    }
}

impl <'a> Container<'a> {
    pub fn iter(&self) -> impl Iterator<Item=&Node<'a>> {
        self.children.iter()
    }
}

pub enum NodeType<'a> {
    Leaf(&'a Leaf<'a>),
    Container(&'a Container<'a>),
    EOI,
}

impl <'a> From<&'a Leaf<'a>> for NodeType<'a> {
    fn from(value: &'a Leaf<'a>) -> Self {
        NodeType::Leaf(value)
    }
}

impl <'a> From<&'a Container<'a>> for NodeType<'a> {
    fn from(value: &'a Container<'a>) -> Self {
        NodeType::Container(value)
    }
}

macro_rules! container_type {
    ($name:ident) => {
        #[derive(std::fmt::Debug)]
        pub struct $name<'a> {
            container: Container<'a>
        }

        impl <'a> From<&'a $name<'a>> for NodeType<'a> {
            fn from(value: &'a $name<'a>) -> NodeType<'a> {
                NodeType::from(&value.container)
            }
        }

        impl <'a> TryFrom<Pair<'a, Rule>> for $name<'a> {
            type Error = ParseError;

            fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
                Ok (Self { container: Container::try_from(value)? })
            }
        }

        impl <'a> Deref for $name<'a> {
            type Target = Container<'a>;

            fn deref(&self) -> &Self::Target {
                &self.container
            }
        }
    };

    ($name: ident $(, ($field_name: ident, $ty: ty))+) => {
        #[derive(std::fmt::Debug)]
        pub struct $name<'a> {
            container: Container<'a>,
            $($field_name: $ty,)+
        }

        impl <'a> From<&'a $name<'a>> for NodeType<'a> {
            fn from(value: &'a $name<'a>) -> NodeType<'a> {
                NodeType::from(&value.container)
            }
        }

        impl <'a> Deref for $name<'a> {
            type Target = Container<'a>;

            fn deref(&self) -> &Self::Target {
                &self.container
            }
        }

        impl <'a> $name<'a> {
            $(
                pub fn $field_name(&self) -> $ty {
                    self.$field_name
                }
            )+
        }
    };
}

macro_rules! leaf_type {
    ($name: ident) => {
        
        #[derive(std::fmt::Debug)]
        pub struct $name<'a> {
            leaf: Leaf<'a>,
        }

        impl <'a> From<&'a $name<'a>> for NodeType<'a> {
            fn from(value: &'a $name<'a>) -> NodeType<'a> {
                NodeType::from(&value.leaf)
            }
        }

        impl <'a> From<Pair<'a, Rule>> for $name<'a> {
            fn from(value: Pair<'a, Rule>) -> Self {
                Self { leaf: Leaf::from(value) }
            }
        }

        impl <'a> AsRef<str> for $name<'a> {
            fn as_ref(&self) -> &str {
                &self.leaf
            }
        }

        impl <'a> Deref for $name<'a> {
            type Target = Leaf<'a>;

            fn deref(&self) -> &Self::Target {
                &self.leaf
            }
        }
    };

    ($name: ident $(, ($field_name: ident, $ty: ty))+) => {
        
        #[derive(std::fmt::Debug)]
        pub struct $name<'a> {
            leaf: Leaf<'a>,
            $($field_name: $ty,)+
        }

        impl <'a> From<&'a $name<'a>> for NodeType<'a> {
            fn from(value: &'a $name<'a>) -> NodeType<'a> {
                NodeType::from(&value.leaf)
            }
        }

        impl <'a> AsRef<str> for $name<'a> {
            fn as_ref(&self) -> &str {
                &self.leaf
            }
        }

        impl <'a> Deref for $name<'a> {
            type Target = Leaf<'a>;

            fn deref(&self) -> &Self::Target {
                &self.leaf
            }
        }

        impl <'a> $name<'a> {
            $(
                pub fn $field_name(&self) -> $ty {
                    self.$field_name
                }
            )+
        }
    };
}

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
container_type!(Link, (source, &'a str));

impl <'a> TryFrom<Pair<'a, Rule>> for List<'a> {
    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let list_as_str = value.as_str();
        let location = value.line_col();

        let ordered = match value.as_rule() {
            Rule::bullet_list => false,
            Rule::ordered_list => true,
            ty => return Err(ParseError::SyntaxError(
                format!(r#"Expected a list node for "{list_as_str}", but got {ty:?}. Error occurred at {location:?}"#)
            ))
        };

        let list = first_child!(value);
        let tight = match list.as_rule() {
            Rule::list_tight => true,
            Rule::list_loose => false,
            ty => return Err(ParseError::SyntaxError(
                format!(r#"Expected a list node for "{list_as_str}", but got {ty:?}. Error occurred at {location:?}"#)
            ))
        };

        Ok( Self { container: Container::try_from(list)?, tight, ordered } )
    }
}

impl <'a> TryFrom<Pair<'a, Rule>> for Heading<'a> {

    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let header_as_str = value.as_str();

        let mut children = value.into_inner();
        let hashes = children.next()
            .ok_or(ParseError::SyntaxError(format!(r#"No header markers found in "{header_as_str}". Error occurred at: {location:?}"#)))?;
        let title = children.next()
            .ok_or(ParseError::SyntaxError(format!(r#"No title found in "{header_as_str}". Error occurred at: {location:?}"#)))?;
        
        let level = hashes.as_str().len() as u8;
        let container = Container::try_from(title)?;
        
        Ok(Self { container, level })
    }

}

impl <'a> TryFrom<Pair<'a, Rule>> for Link<'a> {
    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
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

        Ok( Self { container: label, source } )
        
    }
} 


leaf_type!(Text);
leaf_type!(Linebreak);
leaf_type!(Code);
leaf_type!(Image, (source, &'a str));

impl <'a> TryFrom<Pair<'a, Rule>> for Image<'a> {
    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let location = value.line_col();
        let link_as_str = value.as_str();

        let mut children = value.into_inner();
        let leaf = children.next()
            .ok_or(ParseError::SyntaxError(format!(r#"No label node found in "{link_as_str}". Error occurred at: {location:?}"#)))?
            .into();
        let source = children.next()
            .ok_or(ParseError::SyntaxError(format!(r#"No source found for link in "{link_as_str}". Error occurred at: {location:?}"#)))?
            .as_str();

        Ok( Self { leaf, source } )
        
    }
}

#[derive(Debug)]
pub enum Node<'a> {
    Document(Document<'a>),
    Paragraph(Paragraph<'a>),
    BlockQuote(BlockQuote<'a>),
    Heading(Heading<'a>),
    List(List<'a>),
    ListItem(ListItem<'a>),
    CodeBlock(CodeBlock<'a>),
    Emphasis(Emphasis<'a>),
    Strong(Strong<'a>),
    Label(Label<'a>),
    Link(Link<'a>),
    
    Image(Image<'a>),
    Text(Text<'a>), 
    Linebreak(Linebreak<'a>),
    Code(Code<'a>),

    EOI
}

impl <'a> Node<'a> {

    pub fn inner(&'a self) -> NodeType<'a> {
        match self {
            Self::Document(c) => NodeType::Container(c),
            Self::Paragraph(p) => NodeType::Container(p), 
            Self::BlockQuote(bq) => NodeType::Container(bq),
            Self::Heading(h) => NodeType::Container(h),
            Self::List(l) => NodeType::Container(l),
            Self::ListItem(li) => NodeType::Container(li),
            Self::CodeBlock(cb) => NodeType::Container(cb),
            Self::Emphasis(emp) => NodeType::Container(emp),
            Self::Strong(strong) => NodeType::Container(strong),
            Self::Label(l) => NodeType::Container(l),
            Self::Link(l) => NodeType::Container(l),
            Self::Image(i) => NodeType::Leaf(i),
            Self::Text(txt) => NodeType::Leaf(txt),
            Self::Linebreak(lb) => NodeType::Leaf(lb),
            Self::Code(c) => NodeType::Leaf(c),
            Self::EOI => NodeType::EOI,
        }
    }
}

impl <'a> TryFrom<Pair<'a, Rule>> for Node<'a> {
    type Error = ParseError;

    fn try_from(value: Pair<'a, Rule>) -> Result<Self, Self::Error> {
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
            Rule::italic=> Ok(Node::Emphasis(Emphasis::try_from(value)?)),
            Rule::strong => Ok(Node::Strong(Strong::try_from(value)?)),
            Rule::label => Ok(Node::Label(Label::try_from(value)?)),
            Rule::link => Ok(Node::Link(Link::try_from(first_child!(value))?)),
            Rule::image => Ok(Node::Image(Image::try_from(first_child!(value))?)),
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

pub fn parse_document<'a>(input: &'a str) -> Result<Node<'a>, ParseError> { 
    let raw_tokens = MarkdownParser::parse(Rule::document, input)?;
    let blocks: Result<Vec<Node>, ParseError> = raw_tokens.into_iter()
        .map(|block| Node::try_from(block))
        .collect();
    Ok(Node::Document(Document { container: Container { children: blocks? } })) // TODO clean this up
}

// ------------ Walk the tree ------------

pub enum WalkAction {
    Enter,
    Leave,
}

pub enum NextAction {
    GotoNext,
    SkipChildren,
    End
}

pub trait Visitor {
    fn visit(&self, node: &Node, action: WalkAction) -> NextAction;
}

pub fn walk(root: &Node, visitor: &impl Visitor) -> NextAction {
    let container = match root.inner() {
        NodeType::Container(c) => c,
        NodeType::Leaf(_) | NodeType::EOI => return visitor.visit(root, WalkAction::Enter), 
    };

    match visitor.visit(root, WalkAction::Enter) {
        NextAction::GotoNext => {
            // Visit the children, stopping early if one of them says to end the traversal
            for child in container.iter() {
                if let NextAction::End = walk(child, visitor) {
                    return NextAction::End
                }
            }
            visitor.visit(root, WalkAction::Leave)
        }
        NextAction::SkipChildren => {
            // Give the container its exit visit since we're not visiting any children
            visitor.visit(root, WalkAction::Leave)
        }
        NextAction::End => {
            // Give the container its exit visit before stopping the traversal
            let _ = visitor.visit(root, WalkAction::Leave);
            NextAction::End
        }
    }
}



#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    pub fn basic_test() {
        let input = "![a link](g.co)";
        println!("{:?}", MarkdownParser::parse(Rule::document, input));
        println!("--------------");
        let document = parse_document(input);

        println!("{document:?}");
    }

    #[test]
    pub fn complex_test() {
        let contents = std::fs::read_to_string("src/test.md")
            .unwrap_or_else(|e| panic!("Failed to open file: {e:?}"));

        println!("{:#?}", MarkdownParser::parse(Rule::document, &contents));
        println!("--------------");
        let document = parse_document(&contents);

        println!("{document:#?}");
    }
}
