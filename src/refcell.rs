use crate::cell::Cell;

/// A mutable memory location with dynamically checked borrow rules.
///
/// See the [module-level documentation](index.html) for more.
pub struct RefCell<T> {
  // Protected value that can be borrowed with dynamically checked rules.
  value: std::cell::UnsafeCell<T>,
  // Shared state of `value`.
  state: Cell<RefState>,
}

/// `RefState` represents the different state which we can borrow [`RefCell`](struct.RefCell)
#[derive(Copy, Clone)]
enum RefState {
  /// Shared state with reference count - We have `n` references but *NO* exclusive reference.
  Shared(usize),

  /// Unshared state - We don't have any references to `T` or haven't given out any yet.
  UnShared,

  /// Exclusive state - Given out a *(SINGLE)* mutable reference to `T`.
  Exclusive,
}

impl<T> RefCell<T> {
  /// Creates a new `RefCell` containing `value`.
  ///
  /// # Example
  ///
  /// ```
  /// use ptr::RefCell;
  ///
  /// let c = RefCell::new(5);
  /// ```
  pub const fn new(value: T) -> Self {
    Self {
      value: std::cell::UnsafeCell::new(value),
      state: Cell::new(RefState::UnShared),
    }
  }

  /// Consules the `RefCell`, returning the wrapped value.
  ///
  /// # Examples
  ///
  /// ```
  /// use ptr::RefCell;
  ///
  /// let c  = RefCell::new(5);
  ///
  /// let five = c.into_inner();
  /// ```
  pub fn into_inner(self) -> T {
    self.value.into_inner()
  }

  /// Replace the wrapped value with a new one, returning the old value, without deinitializing either one.
  ///
  /// This function corresponds to [`std::mem::replace`](std.mem.replace).
  ///
  /// # Panics
  ///
  /// Panics if the value is currently borrowed.
  ///
  /// # Examples
  ///
  /// ```
  /// use ptr::RefCell;
  ///
  /// let cell = RefCell::new(5);
  /// let old_value = cell.replace(6);
  ///
  /// // assert_eq!(old_value, 5);
  /// // assert_eq!(cell, RefCell::new(6));
  /// ```
  pub fn replace(&self, _val: T) -> Option<T> {
    // TODO: Use `borrow_mut`.
    // std::mem::replace(&mut *self.borrow_mut(), val)
    None
  }

  /// Replaces the wrapped value with a new one computed from `f`, returning the old value, without deinitializing either one.
  ///
  /// # Panics
  ///
  /// Panics if the value is currently borrowed.
  ///
  /// # Examples
  ///
  /// ```
  /// use ptr::RefCell;
  ///
  /// let cell = RefCell::new(5);
  /// let old_value = cell.replace_ewith(|&mut old| old + 1);
  ///
  /// // assert_eq!(old_value, 5);
  /// // assert_eq!(cell, RefCell::new(6));
  /// ```
  pub fn replace_with(&self, _f: impl FnOnce(&mut T) -> T) -> Option<T> {
    None
  }
}

impl<T: Copy> RefCell<T> {
  pub fn borrow(&self) -> Ref<'_, T> {
    self
      .try_borrow()
      .unwrap_or_else(|_| panic!("{}", BorrowError))
  }

  pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
    // Shared borrow.
    match self.state.get() {
      RefState::UnShared => {
        self.state.set(RefState::Shared(1));
        // SAFETY: No data reace when called from separate threads because `!Sync`.
        // Also, `RefCell` guarantees no `&mut T`, so we can have as many `T` as we want.
        Ok(Ref { cell: self })
      }
      RefState::Shared(n) => {
        self.state.set(RefState::Shared(n + 1));
        // SAFETY: No data reace when called from separate threads because `!Sync`.
        // Also, `RefCell` guarantees no `&mut T`, so we can have as many `T` as we want.
        Ok(Ref { cell: self })
      }
      RefState::Exclusive => Err(BorrowError),
    }
  }

  pub fn borrow_mut(&self) -> RefMut<'_, T> {
    self
      .try_borrow_mut()
      .unwrap_or_else(|_| panic!("{}", BorrowMutError))
  }

  pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowError> {
    // We want exclusive access to modify T.
    match self.state.get() {
      RefState::Exclusive | RefState::Shared(_) => Err(BorrowError),
      RefState::UnShared => {
        self.state.set(RefState::Exclusive);
        // SAFETY: No data race when called from spearate threads because `!Sync`,
        // in addition, `RefCell` gurantees no other reference to T.
        Ok(RefMut { cell: self })
      }
    }
  }
}

pub struct Ref<'r, T> {
  cell: &'r RefCell<T>,
}

impl<T> Drop for Ref<'_, T> {
  fn drop(&mut self) {
    match self.cell.state.get() {
      RefState::Exclusive | RefState::UnShared => unreachable!(),
      RefState::Shared(1) => self.cell.state.set(RefState::UnShared),
      RefState::Shared(n) => self.cell.state.set(RefState::Shared(n - 1)),
    }
  }
}

impl<T> std::ops::Deref for Ref<'_, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    // SAEFTY: A `Ref` is only created if no exlusive reference have been given out.
    // once it's given out state is set to Shared, so no exclusive refs are given out.
    // so dereferencing into a shred ref is fine.
    unsafe { &*self.cell.value.get() }
  }
}

pub struct RefMut<'r, T> {
  cell: &'r RefCell<T>,
}

impl<T> Drop for RefMut<'_, T> {
  fn drop(&mut self) {
    match self.cell.state.get() {
      RefState::UnShared | RefState::Shared(_) => unreachable!(),
      RefState::Exclusive => {
        self.cell.state.set(RefState::UnShared);
      }
    }
  }
}

impl<T> std::ops::Deref for RefMut<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    // SAFETY: See `deref_mut`.
    unsafe { &*self.cell.value.get() }
  }
}

impl<T> std::ops::DerefMut for RefMut<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: A `RefMut` is only created if no other references have been given out.
    // once it's given out state is set to Exlusive, so no future refs are given out.
    // so we have an exclusive lease on the inner value, so mutably dereferencing is fine.
    unsafe { &mut *self.cell.value.get() }
  }
}

pub struct BorrowError;

impl std::fmt::Debug for BorrowError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BorrowError").finish()
  }
}

impl std::fmt::Display for BorrowError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("already mutably borrowed")
  }
}

pub struct BorrowMutError;

impl std::fmt::Debug for BorrowMutError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BorrowMutError").finish()
  }
}

impl std::fmt::Display for BorrowMutError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("already borrowed")
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn new() {
    let _c = RefCell::new(5);
  }

  #[test]
  fn into_inner() {
    let c = RefCell::new(5);

    let _five = c.into_inner();
  }
}
