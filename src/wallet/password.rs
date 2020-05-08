use std::fmt;

#[derive(Clone)]
pub struct Password(String);

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Password(..)")
    }
}

impl Password {
    pub fn new(inner: String) -> Self {
        Password(inner)
    }

    pub unsafe fn take(self) -> String {
        self.0
    }
}
