use std::cell::UnsafeCell;

/// A mutable memory location.
pub struct Cell<T> {
  value: UnsafeCell<T>,
}

// Nightly only: It is however implied by `UnsafeCell`.
// unsafe impl<T> Sync for Cell<T> {}

impl<T> Cell<T> {
  /// Creates a new Cell containing the given value.
  ///
  /// # Examples
  ///
  /// ```
  ///
  /// use ptr::cell::Cell;
  ///
  /// let c = Cell::new(5);
  /// ```
  pub fn new(value: T) -> Self {
    Self {
      value: UnsafeCell::new(value),
    }
  }

  /// Sets the contained value.
  ///
  /// # Examples
  /// ```
  ///
  /// use ptr::cell::Cell;
  ///
  /// let c = Cell::new(5);
  ///
  /// c.set(10);
  /// ```
  pub fn set(&self, value: T) {
    // SAFETY NOTE:
    // We know no one else is concurrently mutating `self.value` (because of `!Sync`).
    // We know we're not invalidating any references, because we never give any out.
    unsafe {
      *self.value.get() = value;
    }
  }

  /// Returns a copy of the contained value.
  ///
  /// # Examples
  ///
  /// ```
  ///
  /// use ptr::cell::Cell;
  ///
  /// let c = Cell::new(5);
  /// assert_eq!(c.get(), 5);
  /// ```
  pub fn get(&self) -> T
  where
    T: Copy,
  {
    // SAFETY NOTE;
    // We know no one else is modifying this value, since only this thread can mutate. (because `!Sync`).
    // and executing only this function. i.e. not mutating the value.
    unsafe { *self.value.get() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
