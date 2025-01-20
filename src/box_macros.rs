/// Ghost box helper macro - trait variant.
/// Place this outside your trait definition.
///
/// # Example
///
/// ```
/// # use ghost_actor::*;
/// pub trait MyTrait {
///     // trait fns here...
///
///     ghost_box_trait_fns!(MyTrait);
/// }
/// ghost_box_trait!(MyTrait);
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// pub struct MyConcrete;
/// impl MyTrait for MyConcrete {
///     ghost_box_trait_impl_fns!(MyTrait);
/// }
///
/// pub struct BoxMyTrait(pub Box<dyn MyTrait>);
///
/// // BoxMyTrait will now implement `Debug + Clone + PartialEq + Eq + Hash`
/// ghost_box_new_type!(BoxMyTrait);
/// ```
#[macro_export]
macro_rules! ghost_box_trait {
    ($trait:ident) => {
        impl ::std::fmt::Debug for ::std::boxed::Box<dyn $trait> {
            fn fmt(
                &self,
                f: &mut ::std::fmt::Formatter<'_>,
            ) -> ::std::fmt::Result {
                self.__box_debug(f)
            }
        }

        impl ::std::clone::Clone for ::std::boxed::Box<dyn $trait> {
            fn clone(&self) -> Self {
                self.__box_clone()
            }
        }

        impl ::std::cmp::PartialEq for ::std::boxed::Box<dyn $trait> {
            fn eq(&self, o: &Self) -> bool {
                self.__box_eq(o)
            }
        }

        impl ::std::cmp::Eq for ::std::boxed::Box<dyn $trait> {}

        impl ::std::hash::Hash for ::std::boxed::Box<dyn $trait> {
            fn hash<Hasher: ::std::hash::Hasher>(&self, state: &mut Hasher) {
                self.__box_hash(state)
            }
        }
    };
}

/// Ghost box helper macro - trait fn variant.
/// Place this inside your trait definition.
///
/// # Example
///
/// ```
/// # use ghost_actor::*;
/// pub trait MyTrait {
///     // trait fns here...
///
///     ghost_box_trait_fns!(MyTrait);
/// }
/// ghost_box_trait!(MyTrait);
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// pub struct MyConcrete;
/// impl MyTrait for MyConcrete {
///     ghost_box_trait_impl_fns!(MyTrait);
/// }
///
/// pub struct BoxMyTrait(pub Box<dyn MyTrait>);
///
/// // BoxMyTrait will now implement `Debug + Clone + PartialEq + Eq + Hash`
/// ghost_box_new_type!(BoxMyTrait);
/// ```
#[macro_export]
macro_rules! ghost_box_trait_fns {
    ($trait:ident) => {
        /// Allows Debug from Box<dyn> trait objects.
        fn __box_debug(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> ::std::fmt::Result;

        /// Allows Clone from Box<dyn> trait objects.
        fn __box_clone(&self) -> ::std::boxed::Box<dyn $trait>;

        /// Allows PartialEq/Eq from Box<dyn> trait objects.
        fn __box_eq(&self, o: &dyn std::any::Any) -> bool;

        /// Allows Hash from Box<dyn> trait objects.
        fn __box_hash(&self, hasher: &mut dyn ::std::hash::Hasher);
    };
}

/// Ghost box helper macro - trait impl fn variant.
/// Place this inside your impl trait definition.
///
/// # Example
///
/// ```
/// # use ghost_actor::*;
/// pub trait MyTrait {
///     // trait fns here...
///
///     ghost_box_trait_fns!(MyTrait);
/// }
/// ghost_box_trait!(MyTrait);
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// pub struct MyConcrete;
/// impl MyTrait for MyConcrete {
///     ghost_box_trait_impl_fns!(MyTrait);
/// }
///
/// pub struct BoxMyTrait(pub Box<dyn MyTrait>);
///
/// // BoxMyTrait will now implement `Debug + Clone + PartialEq + Eq + Hash`
/// ghost_box_new_type!(BoxMyTrait);
/// ```
#[macro_export]
macro_rules! ghost_box_trait_impl_fns {
    ($trait:ident) => {
        #[inline]
        fn __box_debug(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> ::std::fmt::Result {
            ::std::fmt::Debug::fmt(self, f)
        }

        #[inline]
        fn __box_clone(&self) -> Box<dyn $trait> {
            ::std::boxed::Box::new(::std::clone::Clone::clone(&*self))
        }

        #[inline]
        fn __box_eq(&self, o: &dyn ::std::any::Any) -> bool {
            let c: &Self = match <dyn ::std::any::Any>::downcast_ref(o) {
                None => return false,
                Some(c) => c,
            };
            self == c
        }

        #[inline]
        fn __box_hash(&self, hasher: &mut dyn ::std::hash::Hasher) {
            ::std::hash::Hash::hash(self, &mut Box::new(hasher))
        }
    };
}

/// Ghost box helper macro - new type variant.
/// Place this outside your new type definition.
///
/// # Example
///
/// ```
/// # use ghost_actor::*;
/// pub trait MyTrait {
///     // trait fns here...
///
///     ghost_box_trait_fns!(MyTrait);
/// }
/// ghost_box_trait!(MyTrait);
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// pub struct MyConcrete;
/// impl MyTrait for MyConcrete {
///     ghost_box_trait_impl_fns!(MyTrait);
/// }
///
/// pub struct BoxMyTrait(pub Box<dyn MyTrait>);
///
/// // BoxMyTrait will now implement `Debug + Clone + PartialEq + Eq + Hash`
/// ghost_box_new_type!(BoxMyTrait);
/// ```
#[macro_export]
macro_rules! ghost_box_new_type {
    ($newtype:ident) => {
        impl ::std::fmt::Debug for $newtype {
            fn fmt(
                &self,
                f: &mut ::std::fmt::Formatter<'_>,
            ) -> ::std::fmt::Result {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }

        impl ::std::clone::Clone for $newtype {
            fn clone(&self) -> Self {
                Self(::std::clone::Clone::clone(&self.0))
            }
        }

        impl ::std::cmp::PartialEq for $newtype {
            fn eq(&self, o: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.0, &o.0)
            }
        }

        impl ::std::cmp::Eq for $newtype {}

        impl ::std::hash::Hash for $newtype {
            fn hash<Hasher: ::std::hash::Hasher>(&self, state: &mut Hasher) {
                ::std::hash::Hash::hash(&self.0, state)
            }
        }
    };
}
