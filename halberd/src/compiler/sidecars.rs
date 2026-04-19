// FIXME need to disambiguate the naming between
//       sidecars (the AST's implementation of the sidecars system)
//       sidecars (the compiler's sidecar types for use with that system)
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use crate::{scope::ScopeId, types::Type};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExprSidecarInner {
    scope: Option<ScopeId>,
    r#type: Option<Type>,
}

// FIXME rename
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ExprSidecar<S, T>(ExprSidecarInner, PhantomData<(S, T)>);

// FIXME names
pub(crate) trait ExprSidecarS<S> {
    fn scope(&self) -> S;
    fn scope_mut(&mut self) -> &mut S;
}
pub(crate) trait ExprSidecarT<T> {
    fn r#type(&self) -> &T;
    fn type_mut(&mut self) -> &mut T;
}

impl<S: Debug, T: Debug> Debug for ExprSidecar<S, T>
where Self: ExprSidecarS<S> + ExprSidecarT<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // FIXME oh shit wait is this making a local copy just to give the ref of it... should we
        //       add a `.scope_ref` or make `.scope` return a ref or smth? hmm
        f.debug_struct("ExprSidecar")
            .field("scope", &self.scope())
            .field("type", &self.r#type())
            .finish()
    }
}

macro_rules! esx {
    (@ess; $self:ident; $( $ty:ty { scope -> $body1:tt scope_mut -> $body2:tt } )* ) => {
        $( impl<T> ExprSidecarS<$ty> for ExprSidecar<$ty, T> {
            fn scope(&$self) -> $ty $body1
            fn scope_mut(&mut $self) -> &mut $ty $body2
        } )*
    };
    (@est; $self:ident; $( $ty:ty { r#type -> $body1:tt type_mut -> $body2:tt } )* ) => {
        $( impl<S> ExprSidecarT<$ty> for ExprSidecar<S, $ty> {
            fn r#type(&$self) -> &$ty $body1
            fn type_mut(&mut $self) -> &mut $ty $body2
        } )*
    };
}

esx! {@ess; self;
    () {
        scope -> {}
        // pretty sure this is ok - https://old.reddit.com/r/rust/comments/rjhiod/is_it_safe_to_create_references_to_zerosized/hp3oqr1/
        scope_mut -> { unsafe { &mut *std::ptr::dangling_mut::<()>() } }
    }
    Option<ScopeId> {
        scope -> { self.0.scope }
        scope_mut -> { &mut self.0.scope }
    }
    ScopeId {
        scope -> { unsafe { self.0.scope.unwrap_unchecked() } }
        scope_mut -> { unsafe { self.0.scope.as_mut().unwrap_unchecked() } }
    }
}
esx! {@est; self;
    () {
        r#type -> { unsafe { & *std::ptr::dangling_mut::<()>() } }
        // pretty sure this is ok - https://old.reddit.com/r/rust/comments/rjhiod/is_it_safe_to_create_references_to_zerosized/hp3oqr1/
        type_mut -> { unsafe { &mut *std::ptr::dangling_mut::<()>() } }
    }
    Option<Type> {
        r#type -> { &self.0.r#type }
        type_mut -> { &mut self.0.r#type }
    }
    Type {
        r#type -> { unsafe { self.0.r#type.as_ref().unwrap_unchecked() } }
        type_mut -> { unsafe { self.0.r#type.as_mut().unwrap_unchecked() } }
    }
}

impl Default for ExprSidecar<(), ()> {
    fn default() -> Self { Self(ExprSidecarInner { scope: None, r#type: None }, PhantomData) }
}

macro_rules! es_withs {
    (
        $self:ident, $s:ident, $t:ident;
        $($name:ident ($($arg:ident: $arg_ty:ty),*) -> <$rs:ty,$rt:ty> = $es:expr , $et:expr ; )*
    ) => {
        impl<$s, $t> ExprSidecar<$s, $t> {
            // FIXME wait this should be taking self as owned shouldn't it uh oh
            $(pub fn $name ($self $(,$arg:$arg_ty)*) -> ExprSidecar<$rs, $rt> {
                ExprSidecar(
                    ExprSidecarInner { scope: $es, r#type: $et },
                    PhantomData,
                )
            })*
        }
    };
}

es_withs! {self, S, T;
    with_scope_none() -> <Option<ScopeId>, T> = None, self.0.r#type;
    with_scope(s: ScopeId) -> <ScopeId, T> = Some(s), self.0.r#type;
    with_type_none() -> <S, Option<Type>> = self.0.scope, None;
    with_type(t: Type) -> <S, Type> = self.0.scope, Some(t);
}

impl<T> ExprSidecar<Option<ScopeId>, T> {
    pub fn try_with_scope_definitely(self) -> Option<ExprSidecar<ScopeId, T>> {
        if self.0.scope.is_none() {
            None
        } else {
            Some(ExprSidecar(self.0, PhantomData))
        }
    }
}

impl<S> ExprSidecar<S, Option<Type>> {
    pub fn try_with_type_definitely(self) -> Option<ExprSidecar<S, Type>> {
        if self.0.r#type.is_none() {
            None
        } else {
            Some(ExprSidecar(self.0, PhantomData))
        }
    }
}

impl<T> From<ExprSidecar<(), T>> for ExprSidecar<Option<ScopeId>, T> {
    fn from(value: ExprSidecar<(), T>) -> Self { Self(value.0, PhantomData) }
}

impl<T> TryFrom<ExprSidecar<Option<ScopeId>, T>> for ExprSidecar<ScopeId, T> {
    type Error = ();

    fn try_from(value: ExprSidecar<Option<ScopeId>, T>) -> Result<Self, Self::Error> {
        if value.scope().is_none() {
            Err(())
        } else {
            let inner = value.0;
            Ok(Self(inner, PhantomData))
        }
    }
}

impl<S> From<ExprSidecar<S, ()>> for ExprSidecar<S, Option<Type>> {
    fn from(value: ExprSidecar<S, ()>) -> Self { Self(value.0, PhantomData) }
}

impl<S> TryFrom<ExprSidecar<S, Option<Type>>> for ExprSidecar<S, Type> {
    type Error = ();

    fn try_from(value: ExprSidecar<S, Option<Type>>) -> Result<Self, Self::Error> {
        if value.r#type().is_none() {
            Err(())
        } else {
            let inner = value.0;
            Ok(Self(inner, PhantomData))
        }
    }
}
