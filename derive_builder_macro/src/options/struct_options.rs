use darling::util::IdentList;
use syn::{self, Attribute, Ident};

use derive_builder_core::{Bindings, Builder, BuilderPattern, BuildMethod, DeprecationNotes};
use options::{DefaultExpression, LegacyVis, Locate, FieldItem};

/// Container for struct-level options encountered while parsing input to the
/// `Builder` proc macro.
///
/// # Darling Config
/// 1. `from_ident` specifies that a conversion from `syn::Ident` exists. This makes all other fields optional.
/// 1. `attributes(builder)` specifies that non-reserved properties will be sought after in `builder` attributes.
/// 1. `forward_attrs(allow, cfg, doc)` specifies that attributes using those terms will be preserved in the parse
///    container.
#[derive(Debug, Clone, PartialEq, Eq, FromDeriveInput)]
#[darling(from_ident, attributes(builder), forward_attrs(allow, cfg, doc), map = "Self::finish")]
pub struct StructOptions {
    // The first four properties come from the struct itself. Their names are defined
    // by `darling`.

    pub ident: Ident,
    pub vis: syn::Visibility,
    pub generics: syn::Generics,
    pub attrs: Vec<Attribute>,

    // These properties may appear in `#[builder(...)]` attributes on the struct.

    pub pattern: BuilderPattern,
    pub derive: IdentList,
    pub name: Option<Ident>,
    pub build_fn: BuildFnOptions,
    pub setter: Option<SetterOptions>,
    pub try_setter: Option<bool>,
    pub default: Option<DefaultExpression>,

    pub public: Option<()>,
    pub private: Option<()>,

    #[darling(map = "FieldItem::take_vis")]
    pub field: Option<syn::Visibility>,
    
    #[darling(rename = "no_std", map = "no_std_to_bindings")]
    pub bindings: Bindings,
    
    // This property is set by `StructOptions::finish`.

    /// Any deprecation notes that should be included in the generated output.
    #[darling(skip)]
    pub deprecation_notes: DeprecationNotes,
}

impl StructOptions {
    /// Compute the builder identity.
    fn builder_ident(&self) -> Ident {
        self.name
            .clone()
            .unwrap_or_else(|| format!("{}Builder", self.ident).into())
    }

    pub fn as_builder<'a>(&'a self) -> Builder<'a> {
        Builder {
            enabled: true,
            ident: self.builder_ident(),
            pattern: self.pattern,
            derives: self.derive.as_slice(),
            generics: Some(&self.generics),
            visibility: self.to_visibility().unwrap_or(&self.vis),
            fields: vec![],
            functions: vec![],
            doc_comment: None,
            bindings: Default::default(),
            deprecation_notes: self.deprecation_notes.clone(),
        }
    }

    pub fn as_build_method<'a>(&'a self) -> BuildMethod<'a> {
        let (_, ty_generics, _) = self.generics.split_for_impl();
        BuildMethod {
            enabled: !self.build_fn.skip,
            ident: &self.build_fn.name,
            visibility: &self.vis,
            pattern: self.pattern,
            target_ty: &self.ident,
            target_ty_generics: Some(ty_generics),
            initializers: vec![],
            doc_comment: None,
            bindings: self.bindings,
            default_struct: self.default
                .as_ref()
                .map(DefaultExpression::parse_block),
            validate_fn: self.build_fn.validate.as_ref(),
        }
    }

    /// Scan options for deprecation warnings.
    fn finish(mut self) -> Self {
        if !cfg!(feature = "struct_default") && self.default.is_some() {
            let location = self.locate();
            self.deprecation_notes.push(format!(
                   "the meaning of `#[builder(default)]` on the struct level (found {}) will \
                    change in the next version (see \
                    https://github.com/colin-kiegel/rust-derive-builder/issues/61 for \
                    more details). To squelch this message and adopt the new behavior now, \
                    compile `derive_builder` with `--features \"struct_default\"`.",
                    location));
        }

        if !cfg!(feature = "private_fields") && self.field.is_none() {
            let location = self.locate();
            self.deprecation_notes.push(format!(
                "Builder fields will be private by default starting in the next version. \
                (see https://github.com/colin-kiegel/rust-derive-builder/issues/86 for \
                more details). To squelch this message and adopt the new behavior now, \
                compile `derive_builder` with `--features \"private_fields\"` or add \
                `field(<vis>)` to the builder attribute on the struct. (Found {})",
                location));
        }

        self
    }
}

impl Locate for StructOptions {
    fn locate(&self) -> String {
        format!("on struct {}", self.ident)
    }
}

impl LegacyVis for StructOptions {
    fn declared_private(&self) -> bool {
        self.private.is_some()
    }

    fn declared_public(&self) -> bool {
        self.public.is_some()
    }
}

impl From<Ident> for StructOptions {
    fn from(ident: Ident) -> Self {
        StructOptions {
            ident,
            vis: syn::Visibility::Inherited,
            generics: Default::default(),
            attrs: Default::default(),
            pattern: Default::default(),
            derive: Default::default(),
            name: Default::default(),
            build_fn: Default::default(),
            setter: Default::default(),
            try_setter: Default::default(),
            default: Default::default(),
            field: Default::default(),
            private: Default::default(),
            public: Default::default(),
            bindings: Default::default(),
            deprecation_notes: Default::default(),
        }
    }
}

/// The `setter` word in the `#[builder(...)]` attribute at the struct level.
///
/// There is another struct - `options::field_options::SetterOptions` - which corresponds
/// to the settings under this name in field context.
#[derive(Default, Debug, PartialEq, Eq, Clone, FromMetaItem)]
#[darling(default)]
pub struct SetterOptions {
    pub prefix: Option<Ident>,
    pub into: bool,
    pub skip: bool,
    private: Option<()>,
    public: Option<()>,
}

impl LegacyVis for SetterOptions {
    fn declared_private(&self) -> bool {
        self.private.is_some()
    }

    fn declared_public(&self) -> bool {
        self.public.is_some()
    }
}

/// Struct-level control over the generated build function.
#[derive(Debug, Clone, PartialEq, Eq, FromMetaItem)]
#[darling(default)]
pub struct BuildFnOptions {
    /// Whether or not the build function should be skipped.
    pub skip: bool,

    /// The name of the build function to generate, if one is being generated.
    pub name: Ident,

    /// The path to the pre-build validation function that should be used, if any.
    pub validate: Option<syn::Path>,
}

impl Default for BuildFnOptions {
    fn default() -> Self {
        BuildFnOptions {
            name: Ident::new("build"),
            skip: Default::default(),
            validate: Default::default(),
        }
    }
}



/// Converts the presence of the `no_std` field to a `Bindings` instance.
fn no_std_to_bindings(b: bool) -> Bindings {
    if b { Bindings::NoStd } else { Bindings::Std }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromDeriveInput;
    use derive_builder_core::Bindings;

    #[test]
    fn setter_into() {
        let di = syn::parse_derive_input(r#"
            #[derive(Builder)]
            #[builder(setter(into))]
            struct Bar {
                foo: u8,
                bar: String,
            }
        "#).unwrap();

        assert_eq!(StructOptions::from_derive_input(&di).unwrap(), StructOptions {
            setter: Some(SetterOptions {
                into: true,
                ..Default::default()
            }),
            ..Ident::new("Bar").into()
        });
    }

    #[test]
    fn full_struct() {
        let di = syn::parse_derive_input(r#"
            #[derive(Default, Builder)]
            #[builder(no_std, setter(into), default, name = "BarBaz", build_fn(skip))]
            pub struct Bar<T> {
                foo: u8,
                bar: T,
            }
        "#).unwrap();

        assert_eq!(StructOptions::from_derive_input(&di).unwrap(), StructOptions {
            setter: Some(SetterOptions {
                into: true,
                ..Default::default()
            }),
            build_fn: BuildFnOptions {
                skip: true,
                ..Default::default()
            },
            generics: syn::Generics {
                ty_params: vec![syn::Ident::new("T").into()],
                ..syn::Generics::default()
            },
            vis: syn::Visibility::Public,
            name: Ident::new("BarBaz").into(),
            default: Some(DefaultExpression::Trait),
            bindings: Bindings::NoStd,
            ..Ident::new("Bar").into()
        });
    }
}