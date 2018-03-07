#[macro_export]
macro_rules! route {
    ($(#[$meta:meta])* $func_name:ident) => (
        $(#[$meta])*
        pub fn $func_name<H>(&mut self, pattern: &str, handle: H) -> &mut Route
            where H: Fn(&mut Context) + Send + Sync + 'static
        {
            self.add(stringify!($func_name).to_uppercase().parse().unwrap(), pattern, handle)
        }
    )
}

#[macro_export]
macro_rules! middleware {
    ($(#[$meta:meta])* $func_name:ident) => (
        $(#[$meta])*
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
