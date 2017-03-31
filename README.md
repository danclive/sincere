# Sincere

Sincere is http server and micro web framework for Rust(stable) based on epoll, kqueue and multithreadind. Here is an example of a simple application:

```rust
extern crate sincere;

use sincere::Micro;

fn main() {
    let mut app = Micro::new();

    app.get("/", |request, response| {
        println!("{:?}", request.headers);
        response.from_text("Hello Sincere");
    });

    app.run("127.0.0.1:8000");
}
```
Don't forget add this to your `Cargo.toml`:

```
[dependencies]
sincere = { git = "https://github.com/dangcheng/sincere" }
```
Build and run, then, visiting `http://127.0.0.1:8000/`, you will see `Hello Sincere` on the screen.
