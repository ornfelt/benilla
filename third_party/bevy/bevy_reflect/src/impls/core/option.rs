#![expect(
    unused_qualifications,
    reason = "the macro uses `MyEnum::Variant` which is generally unnecessary for `Option`"
)]

use bevy_reflect_derive::impl_reflect;

impl_reflect! {
    #[type_path = "core::option"]
    enum Option<T> {
        None,
        Some(T),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Enum, FromReflect, PartialReflect, VariantType};
    use bevy_reflect_derive::Reflect;
    use static_assertions::assert_impl_all;

    #[test]
    fn should_partial_eq_option() {
        let a: &dyn PartialReflect = &Some(123);
        let b: &dyn PartialReflect = &Some(123);
        assert_eq!(Some(true), a.reflect_partial_eq(b));
    }

    #[test]
    fn option_should_impl_enum() {
        assert_impl_all!(Option<()>: Enum);

        let mut value = Some(123usize);

        assert!(value
            .reflect_partial_eq(&Some(123usize))
            .unwrap_or_default());
        assert!(!value
            .reflect_partial_eq(&Some(321usize))
            .unwrap_or_default());

        assert_eq!("Some", value.variant_name());
        assert_eq!("core::option::Option<usize>::Some", value.variant_path());

        if value.is_variant(VariantType::Tuple) {
            if let Some(field) = value
                .field_at_mut(0)
                .and_then(|field| field.try_downcast_mut::<usize>())
            {
                *field = 321;
            }
        } else {
            panic!("expected `VariantType::Tuple`");
        }

        assert_eq!(Some(321), value);
    }

    #[test]
    fn option_should_from_reflect() {
        #[derive(Reflect, PartialEq, Debug)]
        struct Foo(usize);

        let expected = Some(Foo(123));
        let output = <Option<Foo> as FromReflect>::from_reflect(&expected).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn option_should_apply() {
        #[derive(Reflect, PartialEq, Debug)]
        struct Foo(usize);

        // === None on None === //
        let patch = None::<Foo>;
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto None");

        // === Some on None === //
        let patch = Some(Foo(123));
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto None");

        // === None on Some === //
        let patch = None::<Foo>;
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto Some");

        // === Some on Some === //
        let patch = Some(Foo(123));
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto Some");
    }
}
