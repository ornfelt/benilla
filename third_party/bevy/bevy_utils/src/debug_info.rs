use alloc::{borrow::Cow, fmt, string::String};
use core::any::type_name;
use core::ops::Deref;
use disqualified::ShortName;

/// Wrapper to help debugging ECS issues. This is used to display the names of systems, components, ...
#[derive(Clone, PartialEq, Eq)]
pub struct DebugName {
    name: Cow<'static, str>,
}

impl fmt::Display for DebugName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", &**self)
    }
}

impl fmt::Debug for DebugName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &**self)
    }
}

impl DebugName {
    /// Create a new `DebugName` from a `&str`
    pub const fn borrowed(value: &'static str) -> Self {
        DebugName {
            name: Cow::Borrowed(value),
        }
    }

    /// Create a new `DebugName` from a `String`
    pub fn owned(value: String) -> Self {
        DebugName {
            name: Cow::Owned(value),
        }
    }

    /// Create a new `DebugName` from a type by using its [`core::any::type_name`]
    pub fn type_name<T>() -> Self {
        DebugName {
            name: Cow::Borrowed(type_name::<T>()),
        }
    }

    /// Get the [`ShortName`] corresponding to this debug name
    pub fn shortname(&self) -> ShortName<'_> {
        ShortName(self.name.as_ref())
    }
}

impl Deref for DebugName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl From<Cow<'static, str>> for DebugName {
    fn from(value: Cow<'static, str>) -> Self {
        Self { name: value }
    }
}

impl From<String> for DebugName {
    fn from(value: String) -> Self {
        Self::owned(value)
    }
}

impl From<DebugName> for Cow<'static, str> {
    fn from(value: DebugName) -> Self {
        value.name
    }
}

impl From<&'static str> for DebugName {
    fn from(value: &'static str) -> Self {
        Self::borrowed(value)
    }
}
