use darling::util::Override;
use syn;

use derive_builder_core::{Bindings, BuilderField, BuilderPattern, DeprecationNotes, Initializer, Setter};

use options::{DefaultExpression, FieldItem, LegacyVis, StructOptions};
use super::struct_options::SetterOptions as StructSetterOptions;

/// Options for a builder field.
#[derive(Debug, Clone, FromField, PartialEq, Eq)]
#[darling(attributes(builder), forward_attrs(allow, cfg, doc))]
pub struct FieldOptions {
    /// The identifier of the field.
    pub ident: syn::Ident,

    /// The declared type of the field.
    pub ty: syn::Ty,

    /// The declared visibility of the field.
    pub vis: syn::Visibility,

    /// The declared attributes on the field.
    pub attrs: Vec<syn::Attribute>,

    /// Setter options declared on the field in `#[builder(setter(...))]`.
    #[darling(default, map = "SetterOptions::from_override")]
    pub setter: SetterOptions,

    /// Value for `#[builder(try_setter)]`.
    #[darling(default)]
    pub try_setter: Option<bool>,

    /// Value for `#[builder(default)]` or `#[builder(default = "BLOCK")]`.
    #[darling(default)]
    pub default: Option<DefaultExpression>,

    /// The pattern of builder being used.
    #[darling(default)]
    pub pattern: Option<BuilderPattern>,

    #[darling(default, map="FieldItem::take_vis")]
    pub field: Option<syn::Visibility>,

    #[darling(default)]
    pub private: Option<()>,

    #[darling(default)]
    pub public: Option<()>,

    /// The std/core bindings used.
    /// This must be inherited from the struct.
    #[darling(skip)]
    pub bindings: Bindings,

    #[darling(skip)]
    pub deprecation_notes: DeprecationNotes,

    /// Whether or not the field should get a default from the struct.
    /// This cannot be set directly via attribute.
    #[darling(skip)]
    pub use_default_struct: bool,
}

impl FieldOptions {
    /// Applies struct-level defaults to the field-level options.
    pub fn with_defaults(&mut self, parent: &StructOptions) {
        // `try_setter` at the struct level is inherited at the field level
        // barring explicit opt-out.
        if self.try_setter.is_none() && parent.try_setter.is_some() {
            self.try_setter = parent.try_setter.clone();
        }

        // If there isn't a local default, but the parent has a default then
        // we should reference that struct in the initializer.
        if self.default.is_none() && parent.default.is_some() {
            self.use_default_struct = true;
        }

        if self.pattern.is_none() {
            self.pattern = Some(parent.pattern);
        }

        self.setter.with_defaults(parent.setter.as_ref(), &self.ident);

        // These fields can't be set at the field level, and will always have
        // a value at the struct level, so we inherit them here.
        self.bindings = parent.bindings;
    }

    /// Returns a `BuilderField` according to the options.
    pub fn as_builder_field<'a>(&'a self) -> BuilderField<'a> {
        BuilderField {
            field_ident: &self.ident,
            field_type: &self.ty,
            setter_enabled: !self.setter.skip.unwrap_or_default(),
            field_visibility: self.field.as_ref().unwrap_or(&self.vis),
            attrs: &self.attrs,
            bindings: self.bindings,
        }
    }

    /// Returns a `Setter` according to the options.
    pub fn as_setter<'a>(&'a self) -> Setter<'a> {
        Setter {
            enabled: !self.setter.skip.unwrap_or_default(),
            try_setter: self.try_setter.unwrap_or_default(),
            visibility: self.to_visibility().unwrap_or(&self.vis),
            pattern: self.pattern.expect("Field-level builder pattern should either have been set or inherited"),
            attrs: &self.attrs,
            ident: &self.setter_name(),
            field_ident: &self.ident,
            field_type: &self.ty,
            generic_into: self.setter.into.unwrap_or_default(),
            deprecation_notes: &self.deprecation_notes,
            bindings: self.bindings,
        }
    }

    /// Returns an `Initializer` according to the options.
    ///
    /// # Panics
    ///
    /// if `default_expression` can not be parsed as `Block`.
    pub fn as_initializer<'a>(&'a self) -> Initializer<'a> {
        Initializer {
            setter_enabled: !self.setter.skip.unwrap_or_default(),
            field_ident: &self.ident,
            builder_pattern: self.pattern.expect("Field-level builder pattern should either have been set or inherited"),
            default_value: self.default
                .as_ref()
                .map(DefaultExpression::parse_block),
            use_default_struct: self.use_default_struct,
            bindings: self.bindings,
        }
    }

    /// Gets the name to use for the setter; if an override isn't
    /// provided, it will fall back to the field name.
    fn setter_name<'a>(&'a self) -> &'a syn::Ident {
        self.setter.name.as_ref().unwrap_or(&self.ident)
    }
}

impl LegacyVis for FieldOptions {
    fn declared_private(&self) -> bool {
        self.private.is_some()
    }

    fn declared_public(&self) -> bool {
        self.public.is_some()
    }
}

impl From<(syn::Ident, syn::Ty)> for FieldOptions {
    fn from((ident, ty): (syn::Ident, syn::Ty)) -> Self {
        FieldOptions {
            ident,
            ty,
            vis: syn::Visibility::Inherited,
            setter: Default::default(),
            try_setter: Default::default(),
            default: Default::default(),
            attrs: Default::default(),
            deprecation_notes: Default::default(),
            private: Default::default(),
            public: Default::default(),
            bindings: Default::default(),
            pattern: Default::default(),
            use_default_struct: Default::default(),
            field: Default::default(),
        }
    }
}

#[derive(Default, Debug, Clone, FromMetaItem, PartialEq, Eq)]
#[darling(default)]
pub struct SetterOptions {
    pub name: Option<syn::Ident>,
    pub prefix: Option<syn::Ident>,
    pub skip: Option<bool>,
    pub into: Option<bool>,
}

impl SetterOptions {
    /// Applies defaults from the parent struct to these setter options.
    ///
    /// 1. Inherits `into` unless a local option was set.
    /// 1. Generates the prefixed name for the setter, if a parent prefix was provided AND no
    ///    local name override was used.
    fn with_defaults(&mut self, parent: Option<&StructSetterOptions>, field_ident: &syn::Ident) {
        if self.name.is_none() {
            if let Some(ref prefix) = self.prefix.as_ref() {
                self.name = Some(format!("{}_{}", prefix, field_ident).into());
            }
        }

        if let Some(p) = parent {
            if self.into.is_none() {
                self.into = Some(p.into);
            }

            if self.name.is_none() {
                if let Some(ref prefix) = p.prefix.as_ref() {
                    self.name = Some(format!("{}_{}", prefix, field_ident).into());
                }
            }
        }
    }

    /// The presence of the `setter` word should enable a setter even if the struct-level says
    /// to skip.
    fn presence_enables(mut self) -> Self {
        if self.skip.is_none() {
            self.skip = Some(false);
        }

        self
    }

    /// Convert from `setter` word to SetterOptions.
    fn from_override(v: Override<SetterOptions>) -> Self {
        match v {
            Override::Explicit(val) => val.presence_enables(),
            Override::Inherit => SetterOptions {
                skip: Some(false),
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use darling::FromField;
    use syn;

    #[test]
    fn simple() {
        let di = syn::parse_derive_input(r#"
            pub struct Foo {
                #[builder(setter(into))]
                foo: String,
            }
        "#).unwrap();

        if let syn::Body::Struct(syn::VariantData::Struct(fields)) = di.body {
            assert_eq!(FieldOptions::from_field(&fields[0]).unwrap(), 
                FieldOptions {
                    setter: SetterOptions {
                        into: Some(true),
                        skip: None,
                        name: None,
                    },
                    ..FieldOptions::from((syn::Ident::new("foo"), syn::parse_type("String").unwrap()))
                });
        } else {
            panic!("Didn't read the struct correctly");
        }
    }

    #[test]
    fn rename() {
        let di = syn::parse_derive_input(r#"
            pub struct Foo {
                #[builder(setter(name = "baz"))]
                bar: String,
            }
        "#).unwrap();

        if let syn::Body::Struct(syn::VariantData::Struct(fields)) = di.body {
            assert_eq!(FieldOptions::from_field(&fields[0]).unwrap(), 
                FieldOptions {
                    setter: SetterOptions {
                        name: Some("baz".into()),
                        ..Default::default()
                    },
                    ..FieldOptions::from(("bar".into(), syn::parse_type("String").unwrap()))
                });
        } else {
            panic!("Didn't read the struct correctly")
        }
    }
}