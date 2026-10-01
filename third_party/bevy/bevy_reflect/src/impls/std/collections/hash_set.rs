use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_hashset;

impl_reflect_for_hashset!(::std::collections::HashSet<V,S>);
impl_type_path!(::std::collections::HashSet<V, S>);
