use crate::{Reflect, TypeInfo, TypePath, Typed};
use alloc::{boxed::Box, string::String};
use bevy_platform::{
    collections::HashMap,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use bevy_ptr::PtrMut;
use bevy_utils::TypeIdMap;
use core::{any::TypeId, fmt::Debug};
use downcast_rs::{impl_downcast, Downcast};

/// A registry of [reflected] types.
///
/// This struct is used as the central store for type information.
/// [Registering] a type will generate a new [`TypeRegistration`] entry in this store
/// using a type's [`GetTypeRegistration`] implementation
/// (which is automatically implemented when using [`#[derive(Reflect)]`](derive@crate::Reflect)).
///
/// See the [crate-level documentation] for more information.
///
/// [reflected]: crate
/// [Registering]: TypeRegistry::register
/// [crate-level documentation]: crate
pub struct TypeRegistry {
    registrations: TypeIdMap<TypeRegistration>,
    type_path_to_id: HashMap<&'static str, TypeId>,
}

// TODO:  remove this wrapper once we migrate to Atelier Assets and the Scene AssetLoader doesn't
// need a TypeRegistry ref
/// A synchronized wrapper around a [`TypeRegistry`].
#[derive(Clone, Default)]
pub struct TypeRegistryArc {
    /// The wrapped [`TypeRegistry`].
    pub internal: Arc<RwLock<TypeRegistry>>,
}

impl Debug for TypeRegistryArc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.internal
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .type_path_to_id
            .keys()
            .fmt(f)
    }
}

/// A trait which allows a type to generate its [`TypeRegistration`]
/// for registration into the [`TypeRegistry`].
///
/// This trait is automatically implemented for items using [`#[derive(Reflect)]`](derive@crate::Reflect).
/// The macro also allows [`TypeData`] to be more easily registered.
///
/// If you need to use this trait as a generic bound along with other reflection traits,
/// for your convenience, consider using [`Reflectable`] instead.
///
/// See the [crate-level documentation] for more information on type registration.
///
/// [`Reflectable`]: crate::Reflectable
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `GetTypeRegistration` so cannot provide type registration information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait GetTypeRegistration: 'static {
    /// Returns the default [`TypeRegistration`] for this type.
    fn get_type_registration() -> TypeRegistration;
    /// Registers other types needed by this type.
    ///
    /// This method is called by [`TypeRegistry::register`] to register any other required types.
    /// Often, this is done for fields of structs and enum variants to ensure all types are properly registered.
    fn register_type_dependencies(_registry: &mut TypeRegistry) {}
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a type registry with *no* registered types.
    pub fn empty() -> Self {
        Self {
            registrations: Default::default(),
            type_path_to_id: Default::default(),
        }
    }

    /// Create a type registry with default registrations for primitive types.
    pub fn new() -> Self {
        let mut registry = Self::empty();
        registry.register::<()>();
        registry.register::<bool>();
        registry.register::<char>();
        registry.register::<u8>();
        registry.register::<u16>();
        registry.register::<u32>();
        registry.register::<u64>();
        registry.register::<u128>();
        registry.register::<usize>();
        registry.register::<i8>();
        registry.register::<i16>();
        registry.register::<i32>();
        registry.register::<i64>();
        registry.register::<i128>();
        registry.register::<isize>();
        registry.register::<f32>();
        registry.register::<f64>();
        registry.register::<String>();
        registry
    }

    /// Attempts to register the type `T` if it has not yet been registered already.
    ///
    /// This will also recursively register any type dependencies as specified by [`GetTypeRegistration::register_type_dependencies`].
    /// When deriving `Reflect`, this will generally be all the fields of the struct or enum variant.
    /// As with any type registration, these type dependencies will not be registered more than once.
    ///
    /// If the registration for type `T` already exists, it will not be registered again and neither will its type dependencies.
    ///
    /// Additionally, this will add any reflect [type data](TypeData) as specified in the [`Reflect`] derive.
    ///
    /// # Example
    ///
    /// ```
    /// # use core::any::TypeId;
    /// # use bevy_reflect::{Reflect, TypeRegistry, std_traits::ReflectDefault};
    /// #[derive(Reflect, Default)]
    /// #[reflect(Default)]
    /// struct Foo {
    ///   name: Option<String>,
    ///   value: i32
    /// }
    ///
    /// let mut type_registry = TypeRegistry::default();
    ///
    /// type_registry.register::<Foo>();
    ///
    /// // The main type
    /// assert!(type_registry.contains(TypeId::of::<Foo>()));
    ///
    /// // Its type dependencies
    /// assert!(type_registry.contains(TypeId::of::<Option<String>>()));
    /// assert!(type_registry.contains(TypeId::of::<i32>()));
    ///
    /// // Its type data
    /// assert!(type_registry.get_type_data::<ReflectDefault>(TypeId::of::<Foo>()).is_some());
    /// ```
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        if self.register_internal(TypeId::of::<T>(), T::get_type_registration) {
            T::register_type_dependencies(self);
        }
    }

    /// Internal method to register a type with a given [`TypeId`] and [`TypeRegistration`].
    ///
    /// By using this method, we are able to reduce the number of `TypeId` hashes and lookups needed
    /// to register a type.
    ///
    /// This method is internal to prevent users from accidentally registering a type with a `TypeId`
    /// that does not match the type in the `TypeRegistration`.
    fn register_internal(
        &mut self,
        type_id: TypeId,
        get_registration: impl FnOnce() -> TypeRegistration,
    ) -> bool {
        use bevy_platform::collections::hash_map::Entry;

        match self.registrations.entry(type_id) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                let registration = get_registration();
                Self::update_registration_indices(&registration, &mut self.type_path_to_id);
                entry.insert(registration);
                true
            }
        }
    }

    /// Internal method to register additional lookups for a given [`TypeRegistration`].
    fn update_registration_indices(
        registration: &TypeRegistration,
        type_path_to_id: &mut HashMap<&'static str, TypeId>,
    ) {
        type_path_to_id.insert(registration.type_info().type_path(), registration.type_id());
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    #[inline]
    pub fn get(&self, type_id: TypeId) -> Option<&TypeRegistration> {
        self.registrations.get(&type_id)
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    pub fn get_mut(&mut self, type_id: TypeId) -> Option<&mut TypeRegistration> {
        self.registrations.get_mut(&type_id)
    }

    /// Returns a reference to the [`TypeData`] of type `T` associated with the given [`TypeId`].
    ///
    /// The returned value may be used to downcast [`Reflect`] trait objects to
    /// trait objects of the trait used to generate `T`, provided that the
    /// underlying reflected type has the proper `#[reflect(DoThing)]`
    /// attribute.
    ///
    /// If the specified type has not been registered, or if `T` is not present
    /// in its type registration, returns `None`.
    pub fn get_type_data<T: TypeData>(&self, type_id: TypeId) -> Option<&T> {
        self.get(type_id)
            .and_then(|registration| registration.data::<T>())
    }
}

impl TypeRegistryArc {
    /// Takes a read lock on the underlying [`TypeRegistry`].
    pub fn read(&self) -> RwLockReadGuard<'_, TypeRegistry> {
        self.internal.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes a write lock on the underlying [`TypeRegistry`].
    pub fn write(&self) -> RwLockWriteGuard<'_, TypeRegistry> {
        self.internal
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runtime storage for type metadata, registered into the [`TypeRegistry`].
///
/// An instance of `TypeRegistration` can be created using the [`TypeRegistration::of`] method,
/// but is more often automatically generated using [`#[derive(Reflect)]`](derive@crate::Reflect) which itself generates
/// an implementation of the [`GetTypeRegistration`] trait.
///
/// Along with the type's [`TypeInfo`],
/// this struct also contains a type's registered [`TypeData`].
///
/// See the [crate-level documentation] for more information on type registration.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{TypeRegistration, std_traits::ReflectDefault, FromType};
/// let mut registration = TypeRegistration::of::<Option<String>>();
///
/// assert_eq!("core::option::Option<alloc::string::String>", registration.type_info().type_path());
/// assert_eq!("Option<String>", registration.type_info().type_path_table().short_path());
///
/// registration.insert::<ReflectDefault>(FromType::<Option<String>>::from_type());
/// assert!(registration.data::<ReflectDefault>().is_some())
/// ```
///
/// [crate-level documentation]: crate
pub struct TypeRegistration {
    data: TypeIdMap<Box<dyn TypeData>>,
    type_info: &'static TypeInfo,
}

impl Debug for TypeRegistration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TypeRegistration")
            .field("type_info", &self.type_info)
            .finish()
    }
}

impl TypeRegistration {
    /// Creates type registration information for `T`.
    pub fn of<T: Reflect + Typed + TypePath>() -> Self {
        Self {
            data: Default::default(),
            type_info: T::type_info(),
        }
    }

    /// Returns the [`TypeId`] of the type.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_info.type_id()
    }

    /// Returns a reference to the registration's [`TypeInfo`]
    pub fn type_info(&self) -> &'static TypeInfo {
        self.type_info
    }

    /// Inserts an instance of `T` into this registration's [type data].
    ///
    /// If another instance of `T` was previously inserted, it is replaced.
    ///
    /// [type data]: TypeData
    pub fn insert<T: TypeData>(&mut self, data: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Inserts the [`TypeData`] instance of `T` created for `V`, and inserts any
    /// [`TypeData`] dependencies for that combination of `T` and `V`.
    #[inline]
    pub fn register_type_data<T: TypeData + FromType<V>, V>(&mut self) {
        self.insert(T::from_type());
        T::insert_dependencies(self);
    }

    /// Returns a reference to the value of type `T` in this registration's
    /// [type data].
    ///
    /// Returns `None` if no such value exists.
    ///
    /// [type data]: TypeData
    pub fn data<T: TypeData>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }
}

impl Clone for TypeRegistration {
    fn clone(&self) -> Self {
        let mut data = TypeIdMap::default();
        for (id, type_data) in &self.data {
            data.insert(*id, (*type_data).clone_type_data());
        }

        TypeRegistration {
            data,
            type_info: self.type_info,
        }
    }
}

/// A trait used to type-erase type metadata.
///
/// Type data can be registered to the [`TypeRegistry`] and stored on a type's [`TypeRegistration`].
///
/// Almost any type that implements [`Clone`] can be considered "type data".
/// This is because it has a blanket implementation over all `T` where `T: Clone + Send + Sync + 'static`.
///
/// See the [crate-level documentation] for more information on type data and type registration.
///
/// [crate-level documentation]: crate
pub trait TypeData: Downcast + Send + Sync {
    /// Creates a type-erased clone of this value.
    fn clone_type_data(&self) -> Box<dyn TypeData>;
}

impl_downcast!(TypeData);

impl<T: 'static + Send + Sync> TypeData for T
where
    T: Clone,
{
    fn clone_type_data(&self) -> Box<dyn TypeData> {
        Box::new(self.clone())
    }
}

/// Trait used to generate [`TypeData`] for trait reflection.
///
/// This is used by the `#[derive(Reflect)]` macro to generate an implementation
/// of [`TypeData`] to pass to [`TypeRegistration::insert`].
pub trait FromType<T> {
    /// Creates an instance of `Self` for type `T`.
    fn from_type() -> Self;
    /// Inserts [`TypeData`] dependencies of this [`TypeData`].
    /// This is especially useful for trait [`TypeData`] that has a supertrait (ex: `A: B`).
    /// When the [`TypeData`] for `A` is inserted, the `B` [`TypeData`] will also be inserted.
    fn insert_dependencies(_type_registration: &mut TypeRegistration) {}
}

/// [`Reflect`] values are commonly used in situations where the actual types of values
/// are not known at runtime. In such situations you might have access to a `*const ()` pointer
/// that you know implements [`Reflect`], but have no way of turning it into a `&dyn Reflect`.
///
/// This is where [`ReflectFromPtr`] comes in, when creating a [`ReflectFromPtr`] for a given type `T: Reflect`.
/// Internally, this saves a concrete function `*const T -> const dyn Reflect` which lets you create a trait object of [`Reflect`]
/// from a pointer.
///
/// # Example
/// ```
/// use bevy_reflect::{TypeRegistry, Reflect, ReflectFromPtr};
/// use bevy_ptr::PtrMut;
///
/// #[derive(Reflect)]
/// struct Reflected(String);
///
/// let mut type_registry = TypeRegistry::default();
/// type_registry.register::<Reflected>();
///
/// let mut value = Reflected("Hello world!".to_string());
/// let value = PtrMut::from(&mut value);
///
/// let reflect_data = type_registry.get(core::any::TypeId::of::<Reflected>()).unwrap();
/// let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
/// // SAFE: `value` is of type `Reflected`, which the `ReflectFromPtr` was created for
/// let value = unsafe { reflect_from_ptr.as_reflect_mut(value) };
///
/// assert_eq!(value.downcast_ref::<Reflected>().unwrap().0, "Hello world!");
/// ```
#[derive(Clone)]
pub struct ReflectFromPtr {
    from_ptr_mut: unsafe fn(PtrMut) -> &mut dyn Reflect,
}

#[expect(
    unsafe_code,
    reason = "We must interact with pointers here, which are inherently unsafe."
)]
impl ReflectFromPtr {
    /// Convert `PtrMut` into `&mut dyn Reflect`.
    ///
    /// # Safety
    ///
    /// `val` must be a pointer to a value of the type that the [`ReflectFromPtr`] was constructed for.
    pub unsafe fn as_reflect_mut<'a>(&self, val: PtrMut<'a>) -> &'a mut dyn Reflect {
        // SAFETY: contract uphold by the caller.
        unsafe { (self.from_ptr_mut)(val) }
    }
}

#[expect(
    unsafe_code,
    reason = "We must interact with pointers here, which are inherently unsafe."
)]
impl<T: Reflect> FromType<T> for ReflectFromPtr {
    fn from_type() -> Self {
        ReflectFromPtr {
            from_ptr_mut: |ptr| {
                // SAFETY: `from_ptr_mut` is only called in `ReflectFromPtr::as_reflect_mut`, which
                // lays out the invariants required by `deref_mut`
                unsafe { ptr.deref_mut::<T>() as &mut dyn Reflect }
            },
        }
    }
}

#[cfg(test)]
#[expect(
    unsafe_code,
    reason = "We must interact with pointers here, which are inherently unsafe."
)]
mod test {
    use super::*;

    #[test]
    fn test_reflect_from_ptr() {
        #[derive(Reflect)]
        struct Foo {
            a: f32,
        }

        let foo_registration = <Foo as GetTypeRegistration>::get_type_registration();
        let reflect_from_ptr = foo_registration.data::<ReflectFromPtr>().unwrap();

        let mut value = Foo { a: 1.0 };
        {
            let value = PtrMut::from(&mut value);
            // SAFETY: reflect_from_ptr was constructed for the correct type
            let dyn_reflect = unsafe { reflect_from_ptr.as_reflect_mut(value) };
            match dyn_reflect.reflect_mut() {
                bevy_reflect::ReflectMut::Struct(strukt) => {
                    strukt.field_mut("a").unwrap().apply(&2.0f32);
                }
                _ => panic!("invalid reflection"),
            }
        }

        assert_eq!(value.a, 2.0);
    }
}
