use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_veclike;

impl_reflect_for_veclike!(
    ::alloc::vec::Vec<T>,
    ::alloc::vec::Vec::insert,
    ::alloc::vec::Vec::remove,
    ::alloc::vec::Vec::push,
    ::alloc::vec::Vec::pop,
    [T]
);
impl_type_path!(::alloc::vec::Vec<T>);

#[cfg(test)]
mod tests {
    use alloc::vec;
    use bevy_reflect::PartialReflect;

    #[test]
    fn should_partial_eq_vec() {
        let a: &dyn PartialReflect = &vec![1, 2, 3];
        let b: &dyn PartialReflect = &vec![1, 2, 3];
        let c: &dyn PartialReflect = &vec![3, 2, 1];
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }
}
