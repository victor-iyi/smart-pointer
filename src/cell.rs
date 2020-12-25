//! Shareable mutable containers.
//!
//! Rust memory safety is based on this rule: Given an object `T`, it is only possible to have one of the following:
//! - Having several immutable references (`&T`) to the object (also known as **aliasing**).
//! - Having one mutable reference (`&mut T`) to the object (also known as **mutability).
//!
//! This is enforced by the Rust compiler. However, there are situations where this rule is not flexible engough.
//! Sometimes it is required to have multiple references to an object and yet mutate it.
//!

/// A mutable memory location.
///
/// # Examples
///
/// In this example, you can see that `Cell<T>` enables mutation inside an immutable struct.
/// In other words, it enables "interior mutability".
///
/// ```
/// use ptr::Cell;
///
/// struct SomeStruct {
///   regular_field: u8,
///   special_field: Cell<u8>,
/// }
///
/// let my_struct = SomeStruct {
///   regular_field: 0,
///   special_field: Cell::new(1),
/// };
///
/// let new_value = 10;
///
/// // ERROR:`my_struct` is immuatable.
/// // my_struct.regular_field = new_value;
///
/// // WORKS:Although `my_struct`is immutable, `spcial_field` is a `Cell`,
/// // which can always be mutated.
/// my_struct.special_field.set(new_value);
/// assert_eq!(my_struct.special_field.get(), new_value);
/// ```
///
pub struct Cell<T: ?Sized> {
    value: std::cell::UnsafeCell<T>,
}

unsafe impl<T> Send for Cell<T> where T: Send {}
// impl<T: ?Sized> !Sync for Cell<T> {}

impl<T: Default> Default for Cell<T> {
    /// Creates a `Cell<T>`, with `Default` for `T`.
    fn default() -> Cell<T> {
        Cell::new(Default::default())
    }
}

impl<T: Default> Cell<T> {
    /// Takes the value of a `Cell` leaving `Default::default()` in it's place.
    ///
    /// # Example
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    /// let five = c.take();
    ///
    /// assert_eq!(five, 5);
    /// assert_eq!(c.into_inner(), 0);
    /// ```
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

impl<T: PartialEq + Copy> PartialEq for Cell<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: PartialOrd + Copy> PartialOrd for Cell<T> {
    #[inline]
    fn partial_cmp(&self, other: &Cell<T>) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }

    #[inline]
    fn lt(&self, other: &Cell<T>) -> bool {
        self.get() < other.get()
    }

    #[inline]
    fn le(&self, other: &Cell<T>) -> bool {
        self.get() <= other.get()
    }

    #[inline]
    fn gt(&self, other: &Cell<T>) -> bool {
        self.get() > other.get()
    }

    #[inline]
    fn ge(&self, other: &Cell<T>) -> bool {
        self.get() >= other.get()
    }
}

impl<T> From<T> for Cell<T> {
    fn from(t: T) -> Cell<T> {
        Cell::new(t)
    }
}

// Nightly only: It is however implied by `UnsafeCell`.
// unsafe impl<T> !Sync for Cell<T> {}

impl<T> Cell<T> {
    /// Creates a new `Cell` containing the given `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    /// ```
    // TODO: Learn more about `const fn`.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            value: std::cell::UnsafeCell::new(value),
        }
    }

    /// Sets the contained value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    ///
    /// c.set(10);
    /// ```
    #[inline]
    pub fn set(&self, val: T) {
        let old = self.replace(val);
        drop(old);
    }

    /// Swaps the value of two `Cell`s.
    /// Difference between `std::mem::swap` is that this does not require `&mut` reference.
    ///
    /// # Example
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c1 = Cell::new(5i32);
    /// let c2 = Cell::new(10i32);
    ///
    /// c1.swap(&c2);
    /// assert_eq!(c1.get(), 10);
    /// assert_eq!(c2.get(), 5);
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        // Pointing to the same object.
        if std::ptr::eq(self, other) {
            return;
        }
        // SAFETY: Could be risky when called from separate threads, but `Cell` impl `!Sync`, so this won't happen,
        // This also won't invalidate pointers since `Cell` makes sure nothing else will be pointing into either `Cell`s.
        unsafe {
            std::ptr::swap(self.value.get(), other.value.get());
        }
    }

    /// Replaces the contained value, and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let cell = Cell::new(5);
    ///
    /// assert_eq!(cell.get(), 5);
    /// assert_eq!(cell.replace(10), 5); // returns the replaced value.
    /// assert_eq!(cell.get(), 10);
    /// ```
    pub fn replace(&self, val: T) -> T {
        // SAFETY: This can cause data reaces if called from a separate threads,
        // but Cell is `!Sync`, so this won't happen.
        std::mem::replace(unsafe { &mut *self.value.get() }, val)
    }

    /// Unwraps the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    /// let five = c.into_inner();
    ///
    /// assert_eq!(five, 5);
    /// ```
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: Copy> Cell<T> {
    /// Update the contained value using a function and returns the new value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    /// let new = c.update(|x| x + 1);
    ///
    /// assert_eq!(new, 6);
    /// assert_eq!(c.get(), 6);
    /// ```
    #[inline]
    pub fn update(&self, f: impl FnOnce(T) -> T) -> T {
        let old = self.get();
        let new = f(old);
        self.set(new);
        new
    }

    /// Returns a copy of the contained value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    ///
    /// let five = c.get();
    /// ```
    #[inline]
    pub fn get(&self) -> T {
        // SAFETY: This could cause data races but `Cell` is `!Sync`.
        // We know no one else is modifying this value, since only this thread can mutate. (because `!Sync`).
        // and executing only this function. i.e. not mutating the value.
        unsafe { *self.value.get() }
    }
}

impl<T: ?Sized> Cell<T> {
    /// Returns a raw pointer to the underlying data of in this Cell.
    ///
    /// # Example
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let c = Cell::new(5);
    /// let p = c.as_ptr();
    /// ```
    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borrows `Cell` mutably (at compile-time) which guarantees
    /// that we possess the only reference.
    ///
    /// # Example
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let mut c = Cell::new(5);
    /// *c.get_mut() += 1;
    ///
    /// assert_eq!(c.get(), 6);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: This can cause data race when called from separate threads, but `Cell` is `!Sync`,
        // so it won't happen and `&mut` guarantees unique access.
        unsafe { &mut *self.value.get() }
    }

    /// Returns a`&Cell<T>` from `&mut T`.
    ///
    /// # Example
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let slice: &mut [i32] = &mut [1, 2, 3];
    ///let cell_slice: &Cell<[i32]> = Cell::from_mut(slice);
    ///let slice_cell: &[Cell<i32>] = cell_slice.as_slice_of_cells();
    ///
    /// assert_eq!(slice_cell.len(), 3);
    /// ```
    ///
    /// See also [`as_slice_of_cells`](#method.as_slice_of_cells)
    #[inline]
    pub fn from_mut(t: &mut T) -> &Cell<T> {
        // SAFETY: `&mut` ensures unique access.
        unsafe { &*(t as *mut T as *const Cell<T>) }
    }
}

impl<T> Cell<[T]> {
    /// Returns`&[Cell<T>]` from `&Cell<[T]>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ptr::Cell;
    ///
    /// let slice: &mut [i32] = &mut [1, 2, 3];
    /// let cell_slice: &Cell<[i32]> = Cell::from_mut(slice);
    /// let slice_cell: &[Cell<i32>] = cell_slice.as_slice_of_cells();
    ///
    /// assert_eq!(slice_cell.len(), 3);
    /// ```
    ///
    /// See also [`from_mut`](#method.from_mut)
    pub fn as_slice_of_cells(&self) -> &[Cell<T>] {
        // SAFETY: `Cell<T>` has memory layout as `T`.
        unsafe { &*(self as *const Cell<[T]> as *const [Cell<T>]) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let _c = Cell::new(5);
    }

    #[test]
    fn set() {
        let c = Cell::new(5);
        c.set(10);
    }

    #[test]
    fn swap() {
        let c1 = Cell::new(5i32);
        let c2 = Cell::new(10i32);

        c1.swap(&c2);

        assert_eq!(10, c1.get());
        assert_eq!(5, c2.get());
    }

    #[test]
    fn replace() {
        let cell = Cell::new(5);

        assert_eq!(cell.get(), 5);
        assert_eq!(cell.replace(10), 5); // returns old value.
        assert_eq!(cell.get(), 10);
    }

    #[test]
    fn into_inner() {
        let c = Cell::new(5);
        let five = c.into_inner();

        assert_eq!(five, 5);
    }

    #[test]
    fn get() {
        let c = Cell::new(5);

        let five = c.get();
        assert_eq!(five, 5);
    }

    #[test]
    fn update() {
        let c = Cell::new(5);
        let new = c.update(|x| x + 1);

        assert_eq!(new, 6);
        assert_eq!(c.get(), 6);
    }

    #[test]
    fn as_ptr() {
        let c = Cell::new(5);

        let ptr = c.as_ptr();
        // SAFETY: `Cell` ensures unique access.
        assert_eq!(unsafe { *ptr }, 5);
        assert_eq!(c.get(), 5);
    }

    #[test]
    fn get_mut() {
        let mut c = Cell::new(5);

        *c.get_mut() += 1;

        assert_eq!(c.get(), 6);
    }

    #[test]
    fn from_mut() {
        let slice: &mut [i32] = &mut [1, 2, 3];
        let cell_slice: &Cell<[i32]> = Cell::from_mut(slice);
        let slice_cell: &[Cell<i32>] = cell_slice.as_slice_of_cells();

        assert_eq!(slice_cell.len(), 3);
    }

    #[test]
    fn take() {
        let c = Cell::new(5);
        let five = c.take();

        assert_eq!(five, 5);
        assert_eq!(c.into_inner(), 0);
    }

    #[test]
    fn as_slice_of_cells() {
        let slice: &mut [i32] = &mut [1, 2, 3];
        let cell_slice: &Cell<[i32]> = Cell::from_mut(slice);
        let slice_cell: &[Cell<i32>] = cell_slice.as_slice_of_cells();

        assert_eq!(slice_cell.len(), 3);
    }

    #[test]
    fn cell_str() {
        let cell = Cell::new("John Doe");
        assert_eq!(cell.get(), "John Doe");

        cell.set("Jane Doe");
        assert_eq!(cell.get(), "Jane Doe");
    }

    #[test]
    fn cell_number() {
        let cell = Cell::new(10);
        assert_eq!(cell.get(), 10);

        cell.set(20);
        assert_eq!(cell.get(), 20);
    }

    #[test]
    fn cell_obj() {
        #[derive(Debug, Copy, Clone, PartialEq)]
        struct Color(u8, u8, u8, u8);

        let color = Cell::new(Color(0, 0, 0, 0));
        assert_eq!(color.get(), Color(0, 0, 0, 0));

        color.set(Color(128, 128, 128, 1));
        assert_eq!(color.get(), Color(128, 128, 128, 1));
    }

    // #[test]
    // #[should_panic(expected = "Cell is not thread safe.")]
    // fn it_does_not_work() {
    //   use std::sync::Arc;

    //   let arc = Arc::new(Cell::new(42));

    //   let another = Arc::clone(&arc);
    //   std::thread::spawn(move || {
    //     another.set(43);
    //   });

    //   let other = Arc::clone(&arc);
    //   std::thread::spawn(move || {
    //     other.set(44);
    //   });

    //   eprintln!("{:?}", arc.get());
    // }

    // #[test]
    // #[should_panic(expected = "Dangling reference.")]
    // fn should_not_work() {
    //   let cell = Cell::new(String::from("Hello"));
    //   let hello = cell.get(); // pointer still valid.
    //   cell.set(String::new()); // memory should be cleared here.
    //   cell.set(String::from("world")); // memory should be cleared here.
    //   eprintln!("{}", hello);
    // }
}
