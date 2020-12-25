use crate::cell::Cell;

/// A mutable memory location with dynamically checked borrow rules.
///
/// See the [module-level documentation](index.html) for more.
pub struct RefCell<T> {
    /// Protected value that can be borrowed with dynamically checked rules.
    value: std::cell::UnsafeCell<T>,
    /// Borrow rulues for `value`.
    state: Cell<Borrow>,
}

/// `Borrow` represents the different states/rules which we can borrow [`RefCell`](struct.RefCell)
#[derive(Copy, Clone, PartialEq)]
enum Borrow {
    /// Shared state with ref count - We have `n` borrows but *NO* exclusive borrow.
    Shared(usize),

    /// Unshared state - We don't have any borrows to `T` or haven't given out any yet.
    UnShared,

    /// Exclusive state - Giving out a *(SINGLE)* mutable borrow to `T`.
    Exclusive,
}

impl<T> RefCell<T> {
    /// Creates a new `RefCell` containing `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::RefCell;
    ///
    /// let c = RefCell::new(5);
    /// ```
    #[inline]
    pub const fn new(value: T) -> RefCell<T> {
        RefCell {
            value: std::cell::UnsafeCell::new(value),
            state: Cell::new(Borrow::UnShared),
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
    #[inline]
    pub fn into_inner(self) -> T {
        // Since this function takes `self` (the `RefCell`) by value, the
        // compiler statically verifies that it is not currently borrowed.
        // Therefore the following assertion is just a `debug_assert!`.
        debug_assert!(self.state.get() == Borrow::UnShared);
        self.value.into_inner()
    }

    /// Replace the wrapped value with a new one, returning the old value, without deinitializing either one.
    ///
    /// This function corresponds to [`std::mem::replace`](std/mem/fn.replace.html).
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
    /// assert_eq!(old_value, 5);
    /// assert!(cell == RefCell::new(6));
    /// ```
    #[inline]
    pub fn replace(&self, val: T) -> T {
        std::mem::replace(&mut *self.borrow_mut(), val)
    }

    /// Replaces the wrapped value with a new one computed from `f`, returning the old value,
    /// without deinitializing either one.
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
    /// let old_value = cell.replace_with(|&mut old| old + 1);
    ///
    /// assert_eq!(old_value, 5);
    /// assert!(cell ==  RefCell::new(6));
    /// ```
    #[inline]
    pub fn replace_with(&self, f: impl FnOnce(&mut T) -> T) -> T {
        let mut_borrow = &mut *self.borrow_mut();

        // Get new replacement value.
        let new_value = f(mut_borrow);

        // Replace & return old value.
        std::mem::replace(mut_borrow, new_value)
    }

    /// Swap the wrapped value of `self` with the wrapped value of `other`,
    /// without deinitializing either one.
    ///
    /// This function corresponds to the [`std::mem::swap`](std/mem/fn.swap.html).
    ///
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::RefCell;
    ///
    /// let cell = RefCell::new(5);
    /// let dest = RefCell::new(6);
    ///
    /// cell.swap(&dest);
    ///
    /// assert!(cell == RefCell::new(6));
    /// assert!(dest == RefCell::new(5));
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        std::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T> RefCell<T> {
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
    /// use ptr::RefCell;
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
    /// use ptr::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let m = c.borrow_mut();
    /// let b = c.borrow(); // this causes a panic
    /// ```
    pub fn borrow(&self) -> Ref<'_, T> {
        self.try_borrow()
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
    /// use ptr::RefCell;
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
        // Shared borrow.
        match self.state.get() {
            Borrow::UnShared => {
                self.state.set(Borrow::Shared(1));
                // SAFETY: No data reace when called from separate threads because `!Sync`.
                // Also, `RefCell` guarantees no `&mut T`, so we can have as many `T` as we want.
                Ok(Ref { cell: self })
            }
            Borrow::Shared(n) => {
                self.state.set(Borrow::Shared(n + 1));
                // SAFETY: No data reace when called from separate threads because `!Sync`.
                // Also, `RefCell` guarantees no `&mut T`, so we can have as many `T` as we want.
                Ok(Ref { cell: self })
            }
            Borrow::Exclusive => Err(BorrowError),
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
    /// use ptr::RefCell;
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
    /// use ptr::RefCell;
    ///
    /// let c = RefCell::new(5);
    /// let m = c.borrow();
    ///
    /// let b = c.borrow_mut();  //this causes a panic.
    /// ````
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.try_borrow_mut()
            .unwrap_or_else(|_| panic!("{}", BorrowMutError))
    }

    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowError> {
        // We want exclusive access to modify T.
        match self.state.get() {
            Borrow::Exclusive | Borrow::Shared(_) => Err(BorrowError),
            Borrow::UnShared => {
                self.state.set(Borrow::Exclusive);
                // SAFETY: No data race when called from spearate threads because `!Sync`,
                // in addition, `RefCell` gurantees no other borrow to T.
                Ok(RefMut { cell: self })
            }
        }
    }

    /// Returns a raw pointer to the underlying data in this cell
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let ptr = c.as_ptr();
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borros `RefCell` mutably (at compile-time) so there is no
    /// need for dynamic checks.
    ///
    /// However be cautious: this method expects `self` to be mutable, which is
    /// generally not the case when using a `RefCell`. Take a look at the
    /// [`borrow_mut`] method insted if `self` isn't mutable.
    ///
    /// [`borrow_mut`]: #method.borrow_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::RefCell;
    ///
    /// let mut c = RefCell::new(5);
    /// *c.get_mut() += 1;
    ///
    /// assert_eq!(*c.borrow(), 6);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: This can cause data race when called from separate threads,
        // but `Cell` is `!Sync`,  so it won't happen and `&mut` guarantees unique access.
        unsafe { &mut *self.value.get() }
    }
}

impl<T: Default> RefCell<T> {
    /// Takes the wrapped value, leaving `Default::default()` in its place.
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
    /// let c = RefCell::new(5);
    /// let five = c.take();
    ///
    /// assert_eq!(five, 5);
    /// assert_eq!(c.into_inner(), 0);
    /// ```
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

unsafe impl<T> Send for RefCell<T> where T: Send {}

impl<T: Clone> Clone for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn clone(&self) -> RefCell<T> {
        RefCell::new(self.borrow().clone())
    }
}

impl<T: Default> Default for RefCell<T> {
    /// Creates a `RefCell<T>`, with the `Default` value for `T`.
    #[inline]
    fn default() -> RefCell<T> {
        RefCell::new(Default::default())
    }
}

impl<T: PartialEq> PartialEq for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: Eq> Eq for RefCell<T> {}

impl<T: PartialOrd> PartialOrd for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn partial_cmp(&self, other: &RefCell<T>) -> Option<std::cmp::Ordering> {
        self.borrow().partial_cmp(&*other.borrow())
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn lt(&self, other: &RefCell<T>) -> bool {
        *self.borrow() < *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn le(&self, other: &RefCell<T>) -> bool {
        *self.borrow() <= *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn gt(&self, other: &RefCell<T>) -> bool {
        *self.borrow() > *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn ge(&self, other: &RefCell<T>) -> bool {
        *self.borrow() >= *other.borrow()
    }
}

impl<T: Ord> Ord for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn cmp(&self, other: &RefCell<T>) -> std::cmp::Ordering {
        self.borrow().cmp(&*other.borrow())
    }
}

impl<T> From<T> for RefCell<T> {
    fn from(t: T) -> RefCell<T> {
        RefCell::new(t)
    }
}

// impl<T: std::ops::CoerceUnsized<U>, U> std::ops::CoerceUnsized<RefCell<U>> for RefCell<T> {}

/// Wraps a borrowed reference to a value in a `RefCell` box.
/// A wrapper type for an immutably borrowed value from a [`RefCell<T>`](struct.RefCell.html).
pub struct Ref<'r, T> {
    cell: &'r RefCell<T>,
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        match self.cell.state.get() {
            Borrow::Exclusive | Borrow::UnShared => unreachable!(),
            Borrow::Shared(1) => self.cell.state.set(Borrow::UnShared),
            Borrow::Shared(n) => self.cell.state.set(Borrow::Shared(n - 1)),
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

/// A wrapper type for mutably borrowed value from a [`RefCell<T>`](struct.RefCell.html).
pub struct RefMut<'r, T> {
    cell: &'r RefCell<T>,
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        match self.cell.state.get() {
            Borrow::UnShared | Borrow::Shared(_) => unreachable!(),
            Borrow::Exclusive => {
                self.cell.state.set(Borrow::UnShared);
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

/// An error returned by [`RefCell::try_borrow`](struct.RefCell.html#method.try_borrow)
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

/// An error returned by [`RefCell::try_borrow_mut`](struct.RefCell.html#method.try_borrow_mut).
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

    #[test]
    fn as_ptr() {
        let c = RefCell::new(5);
        let _ptr = c.as_ptr();
    }

    #[test]
    fn replace() {
        let cell = RefCell::new(5);
        let old_value = cell.replace(6);

        assert_eq!(old_value, 5);
        assert_eq!(*cell.borrow(), 6);
    }

    #[test]
    fn replace_with() {
        let cell = RefCell::new(5);
        let old_value = cell.replace_with(|&mut old| old + 1);

        assert_eq!(old_value, 5);
        assert_eq!(*cell.borrow(), 6);
    }

    #[test]
    fn swap() {
        let cell = RefCell::new(5);
        let dest = RefCell::new(6);

        cell.swap(&dest);

        assert_eq!(*cell.borrow(), 6);
        assert_eq!(*dest.borrow(), 5);
    }

    #[test]
    fn borrow() {
        let c = RefCell::new(5);

        let borrowed_five = c.borrow();
        let borrowed_five2 = c.borrow();

        assert_eq!(*borrowed_five, *borrowed_five2);
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

    #[test]
    fn partial_cmp() {
        assert!(RefCell::new(5) == RefCell::new(5));
    }
}
