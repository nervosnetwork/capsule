#[derive(Clone)]
pub struct Password(String);

impl Password {
    pub fn new(inner: String) -> Self {
        Password(inner)
    }

    pub unsafe fn take(self) -> String {
        self.0
    }
}
