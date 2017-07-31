#[macro_export]
macro_rules! route {
    ($func_name:ident) => (
        pub fn $func_name<H>(&mut self, pattern: &str, handle: H) -> &mut Route
            where H: Fn(&mut Context) + Send + Sync + 'static
        {
            self.add(&stringify!($func_name).to_uppercase(), pattern, handle)
        }
    )
}

#[macro_export]
macro_rules! middleware {
    ($func_name:ident) => (
        pub fn $func_name<H>(&mut self, handle: H) -> &mut Self
            where H: Fn(&mut Context) + Send + Sync + 'static
        {
            self.$func_name.push(Middleware {
                inner: Box::new(handle),
            });

            self
        }
    )
}
