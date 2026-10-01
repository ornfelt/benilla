use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_veclike;

impl_reflect_for_veclike!(
    ::alloc::collections::VecDeque<T>,
    ::alloc::collections::VecDeque::insert,
    ::alloc::collections::VecDeque::remove,
    ::alloc::collections::VecDeque::push_back,
    ::alloc::collections::VecDeque::pop_back,
    ::alloc::collections::VecDeque::<T>
);
impl_type_path!(::alloc::collections::VecDeque<T>);
