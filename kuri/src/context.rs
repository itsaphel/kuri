use serde::{de, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
    sync::Arc,
};

/// Registry of types that may be injected in MCPService tool handlers. Any state in the Context is
/// global: it is shared and persisted throughout requests, *not* transient for the lifetime of a
/// single request.
///
/// Types must be registered when the MCPService is built. Afterwards, this HashMap cannot be modified.
#[derive(Default)]
pub struct Context {
    /// A map from type to the injected tool.
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    /// Register a type T in the server's context.
    pub fn insert<T: 'static>(&mut self, state: Inject<T>) {
        self.map.insert(TypeId::of::<Inject<T>>(), Box::new(state));
    }

    /// Get a reference to a type T from the context.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }
}

/// A trait to go from a Context to a type T.
///
/// Implementing this for a type allows it to be directly injected into tool handlers as a parameter.
pub trait FromContext {
    fn from_context(ctx: &Context) -> Self;
}

/// Inject wraps types that can be injected into tool handler functions. These allow tool handlers
/// to have side effects and access shared state, outside of the handler's parameters.
/// The type must be registered in the MCPService's context to be injected.
///
/// # Examples
///
/// ```no_run
/// # use kuri::context::Inject;
/// # use kuri_macros::tool;
/// # use kuri::ToolError;
/// # use std::sync::atomic::{AtomicI32, Ordering};
/// struct MyState { counter: AtomicI32 }
///
/// #[tool]
/// async fn my_tool(my_state: Inject<MyState>) -> Result<(), ToolError> {
///    // MyState is injected from the MCPService's context
///   my_state.counter.fetch_add(1, Ordering::SeqCst);
///   println!("{}", my_state.counter.load(Ordering::SeqCst));
///   Ok(())
/// }
/// ```
pub struct Inject<T: ?Sized>(Arc<T>);

impl<T> Inject<T> {
    pub fn new(state: T) -> Inject<T> {
        Inject(Arc::new(state))
    }
}

// Pass function calls through to the inner object
impl<T: ?Sized> Deref for Inject<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> Clone for Inject<T> {
    fn clone(&self) -> Inject<T> {
        Inject(Arc::clone(&self.0))
    }
}

impl<T: Default> Default for Inject<T> {
    fn default() -> Self {
        Inject::new(T::default())
    }
}

impl<T> Serialize for Inject<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de, T> de::Deserialize<'de> for Inject<T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Ok(Inject::new(T::deserialize(deserializer)?))
    }
}

/// Implement ability to get a `Inject<T>` from the server's context
impl<T: 'static> FromContext for Inject<T> {
    fn from_context(ctx: &Context) -> Self {
        if let Some(obj) = ctx.get::<Inject<T>>() {
            obj.clone()
        } else {
            panic!("Tried to inject an object not in the MCPService's state!")
        }
    }
}
