use crate::cell::Cell;

/// An error returned by [`RefCell::try_borrow`](struct@RefCell.html#method.try_borrow).
pub struct BorrowError;

impl std::fmt::Debug for BorrowError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.debug_struct("BorrowError").finish()
  }
}

impl std::fmt::Display for BorrowError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.write_str("already mutably borrowed")
  }
}

/// An error returned by [`RefCell::try_borrow`](struct@RefCell.html#method.try_borrow_mut).
pub struct BorrowMutError;

impl std::fmt::Debug for BorrowMutError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.debug_struct("BorrowMutError").finish()
  }
}

impl std::fmt::Display for BorrowMutError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.write_str("already borrowed")
  }
}

/// State which`RefCell` can occur.
#[derive(Debug, Copy, Clone)]
enum RefState {
  /// UnShared state. We do not have any Shared or exclusive references.
  UnShared,

  /// If we have shared references, we cannot have exlusive references.
  /// We can have `n` number of references we've given out.
  Shared(usize),

  /// If we have exclusive refenrece, we cannot have shared reference(s).
  /// We can only have a single exlusive refenrece at a time.
  Exclusive,
}

/// Ref represents the reference to the value of `RefCell`.
pub struct Ref<'refcell, T> {
  refcell: &'refcell RefCell<T>,
}

impl<T> Drop for Ref<'_, T> {
  fn drop(&mut self) {
    match self.refcell.state.get() {
      RefState::UnShared | RefState::Exclusive => unreachable!(),
      RefState::Shared(1) => self.refcell.state.set(RefState::UnShared),
      RefState::Shared(n) => self.refcell.state.set(RefState::Shared(n - 1)),
    }
  }
}

impl<T> std::ops::Deref for Ref<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    // SAFETY NOTE:
    // A Ref is only created if no exlusive reference have been given out.
    // once it's given out state is set to Shared, so no exclusive refs are given out.
    // so dereferencing into a shred ref is fine.
    unsafe { &*self.refcell.value.get() }
  }
}

/// RefMut represents a mutable reference to the value of `RefCell`.
pub struct RefMut<'refcell, T> {
  refcell: &'refcell RefCell<T>,
}

impl<T> Drop for RefMut<'_, T> {
  fn drop(&mut self) {
    match self.refcell.state.get() {
      RefState::UnShared | RefState::Shared(_) => unreachable!(),
      RefState::Exclusive => self.refcell.state.set(RefState::UnShared),
    }
  }
}
impl<T> std::ops::Deref for RefMut<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    // SAFETY NOTE:
    // See DerefMut
    unsafe { &*self.refcell.value.get() }
  }
}

impl<T> std::ops::DerefMut for RefMut<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY NOTE:
    // A RefMut is only created if no other references have been given out.
    // once it's given out state is set to Exlusive, so no future refs are given out.
    // so we have an exclusive lease on the inner value, so mutably dereferencing is fine.
    unsafe { &mut *self.refcell.value.get() }
  }
}

/// Reference cell.
pub struct RefCell<T> {
  value: std::cell::UnsafeCell<T>,
  state: Cell<RefState>,
}

impl<T> RefCell<T> {
  /// Creates a new `RefCell` containing `value`.
  ///
  /// # Examples
  /// ```
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  /// ```
  pub fn new(value: T) -> Self {
    Self {
      value: std::cell::UnsafeCell::new(value),
      state: Cell::new(RefState::UnShared),
    }
  }

  /// Immutably borrows the wrapped value.
  /// The borrow lasts until the returned `Ref` exits scope.
  /// Multiple immutable borrows can be taken out at the same time.
  ///
  /// We can borrow shared reference(s). We can have as many shared references has we want,
  /// as long as we do not have exclusive references
  ///
  /// # Panics
  ///
  /// Panics if the value is currently mutably borrowed.
  /// For a non-panicking variant, use [`try_borrow`](#method.try_borrow).
  ///
  /// # Example
  ///
  /// ```
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  ///
  /// let borrowed_five = c.borrow();
  /// let borrowed_five2 = c.borrow();
  ///
  /// ```
  ///
  /// An example of panic:
  ///
  /// ```should_panic
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  ///
  /// let m = c.borrow_mut();
  /// let b = c.borrow(); // this causes a panic
  /// ```
  pub fn borrow(&self) -> Ref<'_, T> {
    self
      .try_borrow()
      .unwrap_or_else(|_| panic!("{}", BorrowError))
  }

  /// Immutably borrows the wrapped value, returning an error if the value is currently mutably borrowed.
  ///
  /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be taken out at the same time.
  ///
  /// This is the non-panicking variant of [`borrow`](#method.borrow).
  ///
  /// # Example
  ///
  /// ```
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  ///
  /// {
  ///    let m = c.borrow_mut();
  ///    assert!(c.try_borrow().is_err());
  /// }
  ///
  /// {
  ///    let m = c.borrow();
  ///    assert!(c.try_borrow().is_ok());
  /// }
  /// ```
  pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
    match self.state.get() {
      RefState::UnShared => {
        // We don't have any shared reference yet, this is the first time we're sharing a reference.
        self.state.set(RefState::Shared(1));
        // SAFETY NOTE:
        // No exclusive refs have been given out since state would be Exclusive.
        Ok(Ref { refcell: self })
      }
      RefState::Shared(n) => {
        // We can safetly share multiple references, becuse we have no exlusive reference.
        self.state.set(RefState::Shared(n + 1));
        // SAFETY NOTE:
        // No exclusive refs have been given out since state would be Exclusive.
        Ok(Ref { refcell: self })
      }
      // We cannot have exclusive reference.
      RefState::Exclusive => Err(BorrowError),
    }
  }

  /// Mutably borrows the wrapped value.
  ///
  /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived from it exit scope.
  /// The value cannot be borrowed while this borrow is active.
  ///
  /// This is an exclusive reference one at a time as long as we don't have any shared references.
  ///
  /// # Panics
  ///
  /// Panics if the value is currently borrowed.
  /// For a non-panicking variant, use [`try_borrow_mut`](#method.try_borrow_mut).
  ///
  /// # Example
  ///
  /// ```
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new("hello".to_owned());
  ///
  /// *c.borrow_mut() = "bonjour".to_owned();
  // assert_eq!(&*c.borrow(), Some("bonjour"));
  /// ```
  ///
  /// An example of panic:
  ///
  /// ```should_panic
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  /// let m = c.borrow();
  ///
  /// let b = c.borrow_mut();  //this causes a panic.
  /// ````
  pub fn borrow_mut(&self) -> RefMut<'_, T> {
    self
      .try_borrow_mut()
      .unwrap_or_else(|_| panic!("{}", BorrowMutError))
  }

  /// Mutably borrows the wrapped value, returning an error if the value is currently borrowed.
  /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived from it exit scope.
  /// The value cannot be borrowed while this borrow is active.
  ///
  /// This is the non-panicking variant of [`borrow_mut`](#method.borrow_mut).
  ///
  /// # Examples
  ///
  /// ```
  /// use ptr::refcell::RefCell;
  ///
  /// let c = RefCell::new(5);
  ///
  /// {
  ///   let m = c.borrow();
  ///   assert!(c.try_borrow_mut().is_err());
  /// }
  ///
  /// assert!(c.try_borrow_mut().is_ok());
  /// ```
  pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
    if let RefState::UnShared = self.state.get() {
      self.state.set(RefState::Exclusive);
      // SAFETY NOTE:
      // No other references have been given out since state would have been Shared or Exclusive.
      Ok(RefMut { refcell: self })
    } else {
      Err(BorrowMutError)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::ops::Deref;

  #[test]
  fn new_refcell() {
    let _c = RefCell::new(5);
  }

  #[test]
  fn borrow() {
    let c = RefCell::new(5);

    let borrowed_five = c.borrow();
    let borrowed_five2 = c.borrow();

    assert_eq!(borrowed_five.deref(), borrowed_five2.deref());
  }

  #[test]
  #[should_panic(expected = "already mutably borrowed")]
  fn panic_borrow() {
    let c = RefCell::new(5);

    let _m = c.borrow_mut();
    let _b = c.borrow(); // this causes a panic
  }

  #[test]
  fn try_borrow() {
    let c = RefCell::new(5);

    {
      let _m = c.borrow_mut();
      assert!(c.try_borrow().is_err());
    }

    {
      let _m = c.borrow();
      assert!(c.try_borrow().is_ok());
    }
  }

  #[test]
  fn borrow_mut() {
    let c = RefCell::new("hello".to_owned());

    *c.borrow_mut() = "bonjour".to_owned();
  }

  #[test]
  #[should_panic(expected = "already borrowed")]
  fn panic_borrow_mut() {
    let c = RefCell::new(5);
    let _m = c.borrow();

    let _b = c.borrow_mut(); //this causes a panic.
  }

  #[test]
  fn try_borrow_mut() {
    let c = RefCell::new(5);

    {
      let _m = c.borrow();
      assert!(c.try_borrow_mut().is_err());
    }

    assert!(c.try_borrow_mut().is_ok());
  }
}
