//! see: https://rust-lang.github.io/rfcs/0445-extension-trait-conventions.html

pub trait StringExt {
    fn pop_if_is(&mut self, c: char) -> bool;
}

impl StringExt for String {
    /// removes the last char of the string if its a specific char,
    ///
    /// returns a bool indicating if a char was removed
    fn pop_if_is(&mut self, c: char) -> bool {
        if self.ends_with(c) {
            self.pop();

            return true;
        }

        false
    }
}
