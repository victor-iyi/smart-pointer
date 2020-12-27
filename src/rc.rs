#![allow(dead_code)]

//! Single-threaded reference-counting pointers. 'Rc' stands for 'Reference Counted'.
//!
//! The type [`Rc<T>`][Rc] provides shared ownership of a value of type `T`, allocated in the heap.
//! Invoking [`clone`][clone] on [`Rc`] produces a new pointer to the same allocation in the heap.
//! When the last [`Rc`] pointer to a given allocation is destroyed, the value stored in that allocation (often referred to as "inner value") is also dropped.
//!
//! Shared references in Rust disallow mutation by default, and [`Rc`] is no exception:
//! you cannot generally obtain a mutable reference to something inside an [`Rc`].
//! If you need mutability, put a [`Cell`] or [`RefCell`] inside the [`Rc`]; see [an example of mutability inside an `Rc`][mutability].
//!
//! [`Rc`] uses non-atomic reference counting. This means that overhead is very low, but an [`Rc`] cannot
//! be sent between threads, and consequently [`Rc`] does not implement [`Send`][send]. As a result, the
//! Rust compiler will check *at compile time* that you are not sending [`Rc`]s between threads.
//! If you need multi-threaded, atomic reference counting, use [`std::sync::Arc`][arc].
//!
//! The [`downgrade`][downgrade] method can be used to create a non-owning [`Weak`] pointer.
//! A [`Weak`] pointer can be [`upgrade`][upgrade]d to an [`Rc`], but this will return [`None`]
//!  if the value stored in the allocation has already been dropped.
//! In other words, `Weak` pointers do not keep the value inside the allocation alive;
//! however, they *do* keep the allocation (the backing store for the inner value) alive.
//!
//! A cycle between [`Rc`] pointers will never be deallocated. For this reason,
//! [`Weak`] is used to break cycles. For example, a tree could have strong [`Rc`] pointers
//! from parent nodes to children, and [`Weak`] pointers from children back to their parents.
//!
//! `Rc<T>` automatically dereferences to `T` (via the [`Deref`] trait), so you can call `T`'s
//! methods on a value of type [`Rc<T>`][`Rc`]. To avoid name clashes with `T`'s methods,
//! the methods of [`Rc<T>`][`Rc`] itself are associated functions, called using function-like syntax:
//!
//! ```
//! use std::rc::Rc;
//!
//! let my_rc = Rc::new(());
//!
//! Rc::downgrade(&my_rc);
//! ```
//!
//! [`Weak<T>`][`Weak`] does not auto-dereference to `T`, because the inner value may have
//! already been dropped.
//!
//! # Cloning references
//!
//! Creating a new reference to the same allocation as an existing reference counted pointer is done
//! using the `Clone` trait implemented for [`Rc<T>`][`Rc`] and [`Weak<T>`][`Weak`].
//!
//! ```
//! use std::rc::Rc;
//!
//! let foo = Rc::new(vec![1.0, 2.0, 3.0]);
//!
//! // The two syntaxes below are equivalent.
//! let a = foo.clone();
//! let b = Rc::clone(&foo);
//!
//! // a and b both point to the same memory as foo.
//! ```
//!
//! The `Rc::clone(&from)` syntax is the most idiomatic because it connveys more explicitly the meaning of the code.
//! In the example above, this syntax makes it easier to see that this code is creating a new reference rather than copying the whole content of `foo`.
//!
//! # Examples
//!
//! Consider a scenario where a set of `Gadget`s are owned by a given `Owner`.
//! We want to have our `Gadget`s point to their `Owner`. We can't do this with unique ownership,
//! because more than one gadget may belong to the same `Owner`. [`Rc`] allows us to share an `Owner` between multiple `Gadget`s, and have the `Owner` remain allocated as long as any `Gadget` points at it.
//!
//! ```
//! use std::rc::Rc;
//!
//! struct Owner {
//!   name: String,
//!   // ...other fields
//! }
//!
//! struct Gadget {
//!   id: i32,
//!   owner: Rc<Owner>,
//!   // ...other fields
//! }
//! # #[allow(clippy::needless_doctest_main)]
//! fn main() {
//!   // Create a reference-counted `Owner`.
//!   let gadget_owner: Rc<Owner> = Rc::new(
//!     Owner {
//!       name: "Gadget Man".to_string(),
//!     }
//!  );
//!
//!   // Create `Gadget`s belonging to `gadget_owner`. Cloning the `Rc<Owner>`
//!   // gives us a new pointer to the same `Owner` allocation, incrementing
//!   // the reference count in the process.
//!   let gadget1 = Gadget {
//!     id: 1,
//!     owner: Rc::clone(&gadget_owner),
//!   };
//!   let gadget2 = Gadget {
//!     id: 2,
//!     owner: Rc::clone(&gadget_owner),
//!   };
//!
//!   // Dispose of our local variable `gadget_owner`.
//!   drop(gadget_owner);
//!
//!   // Despite dropping `gadget_owner`, we're still able to print out the name
//!   // of the `Owner` of the `Gadget`s. This is because we've only dropped a
//!   // single `Rc<Owner>`, not the `Owner` it points to. As long as there are
//!   // other `Rc<Owner>` pointing at the same `Owner` allocation, it will remain
//!   // live. The field projection `gadget1.owner.name` works because
//!   // `Rc<Owner>` automatically dereferences to `Owner`.
//!   println!("Gadget {} owned by {}", gadget1.id, gadget1.owner.name);
//!   println!("Gadget {} owned by {}", gadget2.id, gadget2.owner.name);
//!
//!   // At the end of the function, `gadget1` and `gadget2` are destroyed,
//!   // with them the last counted references to our `Owner`. Gadget Man now
//!   // gets destroyed as well.
//! }
//! ```
//!
//! If our requirements change, and we also need to be able to traverse from `Owner` to `Gadget`, we will run into problems.
//! An [`Rc`] pointer from `Owner` to `Gadget` indroduces a cycle. This means that their reference counts can never reach 0,
//! and the allocaltion will never be destroyed: a memory leak. In order to get around this, we can use [`Weak`] pointers.
//!
//! Rust actually makes it somewhat difficule to produce this loop in the first place. In order to end up with two values that point at each other,
//! one of them needs to be mutable. This is difficult because [`Rc`] enforces memory safety by only giving out shared references to the value it wraps, and these doesn't allow direct mutation.
//! We need to wrap the part of the value we wish to mutate in a [`RefCell`], which provides *interior mutability*:
//! a method to achive mutability through a shared reference. [`RefCell`] enforces Rust's borrowing rules at runtime.
//!
//! ```
//! use std::rc::{Rc, Weak};
//! use ptr::RefCell;
//!
//! struct Owner {
//!   name: String,
//!   gadgets: RefCell<Vec<Weak<Gadget>>>,
//!   // ...other fields
//! }
//!
//! struct Gadget {
//!   id: u32,
//!   owner: Rc<Owner>,
//!   //  ...other fields
//! }
//!
//! # #[allow(clippy::needless_doctest_main)]
//! fn main() {
//!   // Create a reference-counted `Owner`. Note that we've put the `Owner`'s
//!   // vector of `Gadget`s inside a `RefCell` so that we can mutate it through
//!   // a shared reference.
//!   let gadget_owner: Rc<Owner> = Rc::new(
//!     Owner {
//!       name: "Gadget Man".to_string(),
//!       gadgets: RefCell::new(vec![]),
//!     }
//!   );
//!
//!   // Create `Gadget`s belonging to `gadget_owner`, as before.
//!   let gadget1 = Rc::new(
//!     Gadget {
//!       id: 1,
//!       owner: Rc::clone(&gadget_owner),
//!     }
//!   );
//!   let gadget2 = Rc::new(
//!     Gadget {
//!       id: 2,
//!       owner: Rc::clone(&gadget_owner),
//!     }
//!   );
//!
//!   // Add the `Gadget`s to their Owner.
//!   {
//!     let mut gadgets = gadget_owner.gadgets.borrow_mut();
//!     gadgets.push(Rc::downgrade(&gadget1));
//!     gadgets.push(Rc::downgrade(&gadget2));
//!
//!     // `RefCell` dynamic borrow ends here.
//!    }
//!
//!   // Iterate over our `Gadget`s, printing their details out.
//!   for gadget_weak in gadget_owner.gadgets.borrow().iter() {
//!     // `gadget_weak` is a `Weak<Gadget>`. Sinc `Weak` pointers can't
//!     // guarantee the allocation still exists, we need to call
//!     //
//!     // In this case we know the allocation still exists, so we simply
//!     // `unwrap` the `Option`. In more complicated program, you might
//!     // need graceful error handling for a `None` result.
//!
//!     let gadget = gadget_weak.upgrade().unwrap();
//!     println!("Gadget {} owned by {}", gadget.id, gadget.owner.name);
//!   }
//!
//!   // At the end of the function, `gadget_owner`, `gadget1` and `gadget2`
//!   // are destroyed. There are now strong (`Rc`) pointers to the
//!   // gadgets, so they are destroyed. This zeros the reference count on
//!   // Gadget Man, so he gets destroyed as well.
//! }
//! ```
//! [clone]: Clone::clone
//! [`Cell`]: crate::Cell
//! [`RefCell`]: crate::RefCell
//! [send]: std::marker::Send
//! [arc]: std::sync::Arc
//! [`Deref`]: std::ops::Deref
//! [downgrade]: Rc::downgrade
//! [upgrade]: Weak::upgrade
//! [mutability]: crate#introducing-mutability-inside-of-something-immutable

use crate::cell::Cell;

// This is repr(C) to future-proof against possible field-reodering, which would
// interface with otherwise safe [into|from]_raw() of transmutable inner types.
#[repr(C)]
struct RcBox<T: ?Sized> {
  strong: Cell<usize>,
  weak: Cell<usize>,
  value: T,
}

/// A single-threaded reference-counting pointer. 'Rc' stands for 'Reference Counted.'
///
/// See the [module-level documentation](./index.html) for more details.
///
/// The inherent methods of `Rc` are all associated functions, which means that you have to call them as
/// e.g., [`Rc::get_mut(&mut value)`][get_mut] instead of `value.get_mut()`. This avoids conflicts with
/// methods of the inner type `T`.
///
/// [get_mut]: #method.get_mut
pub struct Rc<T: ?Sized> {
  ptr: std::ptr::NonNull<RcBox<T>>,
  phantom: std::marker::PhantomData<RcBox<T>>,
}

// impl<T: ?Sized> !std::marker::Send for Rc<T> {}
// impl<T: ?Sized> !std::marker::Sync for Rc<T> {}

// impl<T: ?Sized + std::marker::Unsize<U>, U: ?Sized> std::ops::CoerceUnsized<Rc<U>> for Rc<T> {}
// impl<T: ?Sized + std::marker::Unsize<U>, U: ?Sized> std::ops::DispatchFromDyn<Rc<U>> for Rc<T> {}

/// `Weak` is a version of [`Rc`] that holds a non-owning reference to managed allocation.
/// The allocation is accessed by calling [`upgrade`] on the [`Weak`] pointer, which returns an [`Option`]`<`[`Rc`]`<T>>`.
///
/// Since a `Weak` reference does not count towards ownership, it will not prevent the value stored in the
/// allocation from being dropped, and `Weak` itself make no guarantees about the value still being present.
/// Thus it may return [`None`] when [`upgrade`]d. Note however that a `Weak` reference *does* prevent the
/// allocation itself (the backing store) from being deallocated.
///
/// A `Weak` pointer is useful for keeping a temporary reference to the allocation managed by [`Rc`] without
/// preventing its inner value from being dropped. It is also used to prevent circular references between
/// [`Rc`] pointers, since mutual owning references would never allow either [`Rc`] to be dropped. For example,
/// a tree could have strong [`Rc`] pointers from parent nodes to children, and `Weak` pointers from children back to their parents.
///
/// The typical way to obtain a `Weak` pointer is to call [`Rc::downgrade`].
///
/// [`upgrade`]: Weak::upgrade
pub struct Weak<T> {
  // This is a `NonNull` to allow optimizing the size of this type in enums,
  // but it is not necessarily a valid pointer.
  // `Weak::new` sets this to `usize::MAX` so that it doesn't need
  // to allocate space on the heap. That's not a value a real pointer
  // will ever have because RcBox has alignment at least 2.
  // This is only possible when `T: Sized`; unsized `T` never dangle.
  ptr: std::ptr::NonNull<RcBox<T>>,
}

// impl<T: ?Sized> !std::marker::Send for Weak<T> {}
// impl<T: ?Sized> !std::marker::Sync for Weak<T> {}
// impl<T: ?Sized + std::marker::Unsize<U>, U: ?Sized> std::ops::CoerceUnsized<Weak<U>> for Weak<T> {}
// impl<T: std::marker::Unsize<U>, U: ?Sized> std::ops::DispatchFromDyn<Weak<U>> for Weak<T> {}
