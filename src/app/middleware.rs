use super::context::Context;
use super::Handle;

pub struct Middleware {
    pub inner: Box<Handle>,
}

impl Middleware {
    pub fn execute(&self, context: &mut Context) {
        if context.next() {
            (self.inner)(context);
        }
    }

    pub fn execute_always(&self, context: &mut Context) {
        (self.inner)(context);
    }
}
