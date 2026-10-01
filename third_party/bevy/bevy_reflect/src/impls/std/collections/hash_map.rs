use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_hashmap;

impl_reflect_for_hashmap!(::std::collections::HashMap<K, V, S>);
impl_type_path!(::std::collections::hash_map::RandomState);
impl_type_path!(::std::collections::HashMap<K, V, S>);

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use static_assertions::assert_impl_all;

    #[test]
    fn should_reflect_hashmaps() {
        assert_impl_all!(std::collections::HashMap<u32, f32>: Reflect);
    }
}
