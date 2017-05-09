use darling::{self, FromMetaItem};
use syn;

use derive_builder_core::{Block};

mod field_options;
mod struct_options;

pub use self::field_options::FieldOptions;
pub use self::struct_options::StructOptions;

static VIS_INHERITED: syn::Visibility = syn::Visibility::Inherited;
static VIS_PUBLIC: syn::Visibility = syn::Visibility::Public;

/// Handler for old-style visibility declarations using `public` and `private` words in
/// attribute. 
///
/// There isn't static support for requiring only one appear, so this trait handles the collision case.
trait LegacyVis {
    /// Whether the word `public` appeared in the meta item.
    fn declared_public(&self) -> bool;

    /// Whether the word `private` appeared in the meta item.
    fn declared_private(&self) -> bool;

    /// To the declared visibility, if any.
    ///
    /// # Panics
    /// This function panics if the item was declared both public and private.
    fn to_visibility(&self) -> Option<&'static syn::Visibility> {
        match (self.declared_public(), self.declared_private()) {
            (true, true) => panic!("A field cannot be both private and public"),
            (true, false) => Some(&VIS_PUBLIC),
            (false, true) => Some(&VIS_INHERITED),
            (false, false) => None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, FromMetaItem)]
#[darling(default)]
pub struct FieldItem {
    pub public: Option<()>,
    pub private: Option<()>,
}

impl FieldItem {
    /// Helper method which converts into a Visibility if one was explicitly declared.
    pub fn take_vis(self) -> Option<syn::Visibility> {
        self.to_visibility().map(|v| v.clone())
    }
}

impl LegacyVis for FieldItem {
    fn declared_private(&self) -> bool {
        self.private.is_some()
    }

    fn declared_public(&self) -> bool {
        self.public.is_some()
    }
}

/// A `DefaultExpression` can be either explicit or refer to the canonical trait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultExpression {
    Explicit(String),
    Trait,
}

impl FromMetaItem for DefaultExpression {
    fn from_word() -> darling::Result<Self> {
        Ok(DefaultExpression::Trait)
    }

    fn from_string(s: &str) -> darling::Result<Self> {
        Ok(DefaultExpression::Explicit(s.to_string()))
    }
}

impl DefaultExpression {
    pub fn parse_block(&self) -> Block {
        let expr = match *self {
            DefaultExpression::Explicit(ref s) => {
                if s.is_empty() {
                    panic!(r#"Empty default expressions `default=""` are not supported."#);
                }
                s
            },
            DefaultExpression::Trait => "::derive_builder::export::Default::default()",
        };

        expr.parse().expect(&format!("Couldn't parse default expression `{:?}`", self))
    }
}