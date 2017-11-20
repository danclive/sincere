# Sincere

Sincere is http server and micro web framework for Rust(stable) based on epoll and multithreadind. Here is an example of a simple application:

```rust
extern crate sincere;

use sincere::App;

fn main() {
    let mut app = App::new();

    app.get("/", |context| {
        context.response.from_text("Hello world!").unwrap();
    });

    app.run("127.0.0.1:8000");
}
```
Don't forget add this to your `Cargo.toml`:

```
[dependencies]
sincere = "0.4"
```
Build and run, then, visiting `http://127.0.0.1:8000/`, you will see `Hello world` on the screen.
