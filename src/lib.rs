//! Shareable mutable containers.
//!
//! Rust memory safety is based on this rule: Given an object `T`, it is only possible to have one of the following:
//!
//! - Having several immutable references (`&T`) to the object (also known as **aliasing**).
//! - Having one mutable reference (`&mut T`) to the object (also known as **mutability**).
//!
//! This is enforced by the Rust compiler. However, there are situations where this rule is not flexible enough.
//! Sometimes it is required to have multiple references to an object and yet mutate it.
//!
//! Shareable mutable containers exist to permit mutability in a controlled manner, even in the presence of aliasing.
//! Both [`Cell<T>`][`Cell`] and [`RefCell<T>`][`RefCell`] allow doing this in a single-threaded way.
//! However, neither [`Cell<T>`][`Cell`] nor [`RefCell<T>`][`RefCell`]
//! are thread safe (they do not implement [`Sync`]). If you need to do aliasing and mutation between multiple threads it is
//! possible to use [`Mutex`], [`RwLock`] or [atomic types][atomic].
//!
//! Values of the [`Cell<T>`][`Cell`] and [`RefCell<T>`][`RefCell`] types may be mutated through shared references (i.e. the common `&T` type),
//! whereas most Rust types can only be mutated through unique (`&mut T`) references. We say that [`Cell<T>`][`Cell`] and [`RefCell<T>`][`RefCell`]
//! provide 'interior mutability', in contrast with typical Rust types that exhibit 'inherited mutability'.
//!
//! Cell types come in two flavors: [`Cell<T>`][`Cell`] and [`RefCell<T>`][`RefCell`]. [`Cell<T>`][`Cell`] implements interior mutability by moving values in and out of the [`Cell<T>`][`Cell`].
//! To use references instead of values, one must use the [`RefCell<T>`][`RefCell`] type, acquiring a write lock before mutating.
//! [`Cell<T>`][`Cell`] provides methods to retrieve and change the current interior value:
//!
//! - For types that implement [`Copy`], the [`get`] method retrieves the current interior value.
//! - For types that implement [`Default`], the [`take`] method replaces the current interior value with [`Default::default()`][`default`] and returns the replaced value.
//! - For all types, the [`replace`] method replaces the current interior value and returns the replaced value and the [`into_inner`] method consumes the [`Cell<T>`][`Cell`] and returns the interior value.
//! Additionally, the [`set`] method replaces the interior value, dropping the replaced value.
//!
//! [`RefCell<T>`][`RefCell`] uses Rust's lifetimes to implement 'dynamic borrowing', a process whereby one can claim temporary, exclusive, mutable access to the inner value.
//! Borrows for [`RefCell<T>`][`RefCell`]s are tracked 'at runtime', unlike Rust's native reference types which are entirely tracked statically, at compile time.
//! Because [`RefCell<T>`][`RefCell`] borrows are dynamic it is possible to attempt to borrow a value that is already mutably borrowed; when this happens it results in thread panic.
//!
//! # When to choose interior mutability
//!
//! The more common inherited mutability, where one must have unique access to mutate a value, is one of the key language elements that enables Rust to reason strongly about pointer aliasing, statically preventing crash bugs. Because of that, inherited mutability is preferred, and interior mutability is something of a last resort. Since cell types enable mutation where it would otherwise be disallowed though, there are occasions when interior mutability might be appropriate, or even must be used, e.g.
//!
//! - Introducing mutability 'inside' of something immutable
//! - Implementation details of logically-immutable methods.
//! - Mutating implementations of [`Clone`].
//!
//! # Introducing mutability 'inside' of something immutable
//!
//! Many shared smart pointer types, including [`Rc<T>`][`Rc`] and [`Arc<T>`][`Arc`], provide containers that can be cloned and shared between multiple parties.
//! Because the contained values may be multiply-aliased, they can only be borrowed with `&`, not `&mut`.
//! Without cells it would be impossible to mutate data inside of these smart pointers at all.
//!
//! It's very common then to put a [`RefCell<T>`][`RefCell`] inside shared pointer types to reintroduce mutability:
//!
//! ```
//! use std::collections::HashMap;
//! use std::rc::Rc;
//!
//! use pointer::{RefCell, RefMut};
//!
//! # #[allow(clippy::needless_doctest_main)]
//! fn main() {
//!     let shared_map: Rc<RefCell<_>> = Rc::new(RefCell::new(HashMap::new()));
//!     // Create a new block to limit the scope of the dynamic borrow
//!     {
//!         let mut map: RefMut<_> = shared_map.borrow_mut();
//!         map.insert("africa", 92388);
//!         map.insert("kyoto", 11837);
//!         map.insert("piccadilly", 11826);
//!         map.insert("marbles", 38);
//!     }
//!
//!     // Note that if we had not let the previous borrow of the cache fall out
//!     // of scope then the subsequent borrow would cause a dynamic thread panic.
//!     // This is the major hazard of using `RefCell`.
//!     let total: i32 = shared_map.borrow().values().sum();
//!     println!("{}", total);
//! }
//! ```
//!
//! Note that this example uses [`Rc<T>`][`Rc`] and not [`Arc<T>`][`Arc`]. [`RefCell<T>`][`RefCell`]s are for single-threaded scenarios.
//! Consider using [`RwLock<T>`][`RwLock`] or [`Mutex<T>`][`Mutex`] if you need shared mutability in a multi-threaded situation.
//!
//! # Implementation details of logically-immutable methods
//!
//! Occasionally it may be desirable not to expose in an API that there is mutation happening "under the hood".
//! his may be because logically the operation is immutable, but e.g., caching forces the implementation to perform mutation;
//! or because you must employ mutation to implement a trait method that was originally defined to take `&self`.
//!
//! ```
//! use pointer::RefCell;
//!
//! struct Graph {
//!     edges: Vec<(i32, i32)>,
//!     span_tree_cache: RefCell<Option<Vec<(i32, i32)>>>
//! }
//!
//! impl Graph {
//!     fn minimum_spanning_tree(&self) -> Vec<(i32, i32)> {
//!         self.span_tree_cache.borrow_mut()
//!             .get_or_insert_with(|| self.calc_span_tree())
//!             .clone()
//!     }
//!
//!     fn calc_span_tree(&self) -> Vec<(i32, i32)> {
//!         // Expensive computation goes here
//!         vec![]
//!     }
//! }
//! ```
//!
//!
//! # Mutating implementations of [`Clone`]
//!
//! This is simply a special - but common - case of the previous: hiding mutability for operations that appear to be immutable.
//! The [`clone`] method is expected to not change the source value, and is declared to take `&self`, not `&mut self`.
//! Therefore, any mutation that happens in the [`clone`] method must use [cell types][cells].
//! For example, [`Rc<T>`][`Rc`] maintains its reference counts within a [`Cell<T>`][`Cell`].
//!
//! ```
//! use pointer::Cell;
//!
//! use std::ptr::NonNull;
//! use std::process::abort;
//! use std::marker::PhantomData;
//!
//! struct Rc<T: ?Sized> {
//!     ptr: NonNull<RcBox<T>>,
//!     phantom: PhantomData<RcBox<T>>,
//! }
//!
//! struct RcBox<T: ?Sized> {
//!     strong: Cell<usize>,
//!     refcount: Cell<usize>,
//!     value: T,
//! }
//!
//! impl<T: ?Sized> Clone for Rc<T> {
//!     fn clone(&self) -> Rc<T> {
//!         self.inc_strong();
//!         Rc {
//!             ptr: self.ptr,
//!             phantom: PhantomData,
//!         }
//!     }
//! }
//!
//! trait RcBoxPtr<T: ?Sized> {
//!
//!     fn inner(&self) -> &RcBox<T>;
//!
//!     fn strong(&self) -> usize {
//!         self.inner().strong.get()
//!     }
//!
//!     fn inc_strong(&self) {
//!         self.inner()
//!             .strong
//!             .set(self.strong()
//!                      .checked_add(1)
//!                      .unwrap_or_else(|| abort() ));
//!     }
//! }
//!
//! impl<T: ?Sized> RcBoxPtr<T> for Rc<T> {
//!    fn inner(&self) -> &RcBox<T> {
//!        unsafe {
//!            self.ptr.as_ref()
//!        }
//!    }
//! }
//! ```
//!
//! [`RefCell`]: crate::refcell::RefCell
//! [`Cell`]: crate::cell::Cell
//! [cells]: crate::cell
//! [`Rc`]: crate::rc::Rc
//! [`get`]: crate::cell::Cell::get
//! [`set`]: crate::cell::Cell::set
//! [`take`]: crate::cell::Cell::take
//! [`replace`]: crate::cell::Cell::replace
//! [`into_inner`]: crate::cell::Cell::into_inner
//! [`Default`]: std::default::Default
//! [`default`]: std::default::Default::default
//! [`Clone`]: Clone
//! [`clone`]: Clone::clone
//! [`Copy`]: std::marker::Copy
//! [`Sync`]: std::marker::Sync
//! [`Mutex`]: std::sync::Mutex
//! [`RwLock`]: std::sync::RwLock
//! [`Arc`]: std::sync::Arc
//! [atomic]: std::sync::atomic

pub mod cell;
pub mod rc;
pub mod refcell;

pub use cell::Cell;
pub use rc::{Rc, Weak};
pub use refcell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};
