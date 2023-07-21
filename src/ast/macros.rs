/// Sometimes, we need to extract the singleton child from a [`pest::iterator::Pair`] and fail if it's
/// missing. This is normally to make the rule processing less verbose. For example, we can match on 
/// the `link` rule, then extract the specific link variant for processing, and since we know that
/// there must be a link variant as the only child of the node, we can raise an error if it's missing.
/// 
/// # Parameters
/// 
/// - `$value`: A [`pest::iterator::Pair`]. This value is consumed in the macro.
/// 
/// # Returns
/// 
/// A Result<pest::iterator::Pair, ParseError>. Will be the Err variant only if the value was missing.
#[macro_export]
macro_rules! first_child {
  ($value: expr) => {
      $value.into_inner()
          .next()
          .ok_or(ParseError::SyntaxError(format!("Missing required child in expression")))
  };
}

/// Creates a struct to represent a container node, along with some trait implementations.
/// Always requires an identifier for the name of the generated struct, and optionally accepts
/// one more more tuples of (identifier, type) to add additional fields to the struct.
/// 
/// This macro will always define the [`AsRef<Vec<Node<'input>>>`] trait for the generated struct,
/// but will only create an implementation for [`TryFrom<Pair<'input, Rule>>`] if no extra fields 
/// are specified since there's no general way to parse those fields from the Pair.
/// 
/// If extra fields are provided, the macro will create a non-mutating getter method for each field.
#[macro_export]
macro_rules! container_type {
  ($name:ident) => {
      #[derive(std::fmt::Debug, PartialEq)]
      pub struct $name<'input> {
          children: Vec<Node<'input>>
      }

      impl <'input> TryFrom<Pair<'input, Rule>> for $name<'input> {
          type Error = ParseError;

          fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
                let children: Result<Vec<Node>, ParseError> = value.into_inner()
                    .map(|child| Node::try_from(child))
                    .collect();
              Ok (Self { children: children? })
          }
      }

      impl <'input> AsRef<Vec<Node<'input>>> for $name<'input> {
        fn as_ref(&self) -> &Vec<Node<'input>> {
            &self.children
        }
      }

      impl <'input> $name<'input> {
        #[allow(dead_code)]
          pub fn new(children: Vec<Node<'input>> ) -> Self {
            Self {
              children,
            }
          }
      }
  };

  ($name: ident $(, ($field_name: ident, $ty: ty))+) => {
      #[derive(std::fmt::Debug, PartialEq)]
      pub struct $name<'input> {
          children: Vec<Node<'input>>,
          $($field_name: $ty,)+
      }

      impl <'input> AsRef<Vec<Node<'input>>> for $name<'input> {
        fn as_ref(&self) -> &Vec<Node<'input>> {
            &self.children
        }
      }

      impl <'input> $name<'input> {

          #[allow(dead_code)]
          pub fn new(children: Vec<Node<'input>> $(, $field_name: $ty)+) -> Self {
            Self {
              children,
              $($field_name,)+
            }
          }

          $(
              pub fn $field_name(&self) -> $ty {
                  self.$field_name
              }
          )+
      }
  };
}

/// Creates a struct to represent a leaf node, along with some trait implementations.
/// Always requires an identifier for the name of the generated struct, and optionally accepts
/// one more more tuples of (identifier, type) to add additional fields to the struct.
/// 
/// This macro will always define the [`AsRef<str>`] trait for the generated struct,
/// but will only create an implementation for [`TryFrom<Pair<'input, Rule>>`] if no extra fields 
/// are specified since there's no general way to parse those fields from the Pair.
/// 
/// If extra fields are provided, the macro will create a non-mutating getter method for each field.
#[macro_export]
macro_rules! leaf_type {
  ($name: ident) => {
      
        #[derive(std::fmt::Debug, PartialEq)]
        pub struct $name<'input> {
            literal: &'input str,
        }

      impl <'input> From<Pair<'input, Rule>> for $name<'input> {
          fn from(value: Pair<'input, Rule>) -> Self {
              Self { literal: value.as_str() }
          }
      }

    impl <'input> AsRef<str> for $name<'input> {
        fn as_ref(&self) -> &str {
            &self.literal
        }
    }
  };

  ($name: ident $(, ($field_name: ident, $ty: ty))+) => {
      
      #[derive(std::fmt::Debug, PartialEq)]
      pub struct $name<'input> {
          literal: &'input str,
          $($field_name: $ty,)+
      }

      impl <'input> AsRef<str> for $name<'input> {
          fn as_ref(&self) -> &str {
              &self.literal
          }
      }

      impl <'input> $name<'input> {
          $(
              pub fn $field_name(&self) -> $ty {
                  self.$field_name
              }
          )+
      }
  };
}