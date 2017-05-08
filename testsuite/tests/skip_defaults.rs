#[macro_use]
extern crate derive_builder;

#[derive(Debug, PartialEq, Eq, Builder)]
#[builder(default)]
struct Lorem {
    #[builder(setter(skip))]
    foo: u8,
    bar: String,
}

impl Default for Lorem {
    fn default() -> Self {
        Lorem {
            foo: 10,
            bar: Default::default(),
        }
    }
}

#[test]
fn skipped_field_gets_struct_default() {
    assert_eq!(LoremBuilder::default().bar("hello".to_string()).build().unwrap(), Lorem {
        foo: 10,
        bar: "hello".to_string(),
    });
}