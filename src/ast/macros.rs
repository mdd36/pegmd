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
        $value.next().ok_or(ParseError::SyntaxError(format!(
            "Missing required child in expression"
        )))
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
        #[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<'input> {
            children: Children<'input>,
            #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
            span: &'input str,
        }

        impl <'input> TryFrom<Pair<'input, Rule>> for $name<'input> {
            type Error = ParseError;

            fn try_from(value: Pair<'input, Rule>) -> Result<Self, Self::Error> {
                let span = value.as_str();
                let children = Children::try_from(value)?;
                Ok (Self { span, children })
            }
        }

        impl <'input> $name<'input> {
            #[allow(dead_code)]
            pub fn new(children: Children<'input>, span: &'input str) -> Self {
                Self {
                    children,
                    span,
                }
            }

            pub fn children(&self) -> &Children {
                &self.children
            }

            pub fn children_mut(&'input mut self) -> &mut Children {
                &mut self.children
            }

            pub fn as_span(&self) -> &str {
                self.span
            }
        }
    };

    ($name: ident $(, ($field_name: ident, $ty: ty))+) => {
        #[derive(std::fmt::Debug, PartialEq)]
        #[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<'input> {
            children: Children<'input>,
            #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
            span: &'input str,
            $($field_name: $ty,)+
        }

        impl <'input> $name<'input> {

            #[allow(dead_code)]
            pub fn new(
                children: Children<'input>,
                span: &'input str
                $(, $field_name: $ty)+
            ) -> Self {
                Self {
                    children,
                    span,
                    $($field_name,)+
                }
            }

            pub fn as_span(&self) -> &str {
                self.span
            }

            pub fn children(&self) -> &Children {
                &self.children
            }

            pub fn children_mut(&'input mut self) -> &mut Children {
                &mut self.children
            }

            $(pub fn $field_name(&self) -> $ty {
                self.$field_name
            })+
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
        #[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
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

        impl <'input> $name<'input> {
            pub fn as_span(&self) -> &'input str {
                &self.literal
            }
        }
    };

    ($name: ident $(, ($field_name: ident, $ty: ty))+) => {

        #[derive(std::fmt::Debug, PartialEq)]
        #[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
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
            pub fn as_span(&self) -> &'input str {
                &self.literal
            }

            $(
                pub fn $field_name(&self) -> $ty {
                    self.$field_name
                }
            )+
        }
    };
}
