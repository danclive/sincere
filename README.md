# Sincere

[![crates.io](https://meritbadge.herokuapp.com/sincere)](https://crates.io/crates/sincere)
[![Build Status](https://travis-ci.org/mitum/sincere.svg?branch=master)](https://travis-ci.org/mitum/sincere)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Sincere is a micro web framework for Rust(stable) based on hyper and multithreadind. Style like koa. The same, which aims to be a smaller, more expressive, and more robust foundation for web applications and APIs. Sincere does not bundle any middleware within core, and provides an elegant suite of methods that make writing servers fast and enjoyable.
Here is an example of a simple application:

```rust
extern crate sincere;

use sincere::App;

fn main() {
    let mut app = App::new();

    app.get("/", |context| {
        context.response.from_text("Hello world!").unwrap();
    });

    app.run("127.0.0.1:8000", 20).unwrap();
}
```
Don't forget add this to your `Cargo.toml`:

```toml
[dependencies]
sincere = "0.5.8"
```
Build and run, then, visiting `http://127.0.0.1:8000/`, you will see `Hello world` on the screen.

## API documentation: 

- [master](https://docs.rs/sincere)

## Guide

### Routing

```rust
app.add("GET", "/user", ...);

app.get("/user", ...);

app.get("/user/{id:[0-9]+}", ...);

app.post("/user", ...);

app.put("/user/{id:[0-9]+}", ...);

app.delete("/user/{id:[0-9]+}", ...);

app.options("/", ...);

app.connect("/", ...);

app.head("/", ...);

```

### Route Group

```rust
use sincere::App;
use sincere::Group;

fn main() {
    let mut app = App::new();

    app.get("/", |context| {
        context.response.from_text("Hello world!").unwrap();
    });

    let mut user_group = Group::new("/user");

    // /user
    user_group.get("/", ...);
    // /user/123
    app.get("/{id:[0-9]+}", ...);

    app.mount_group(user_group::handle);

    app.run("127.0.0.1:8000");
}
```

```rust
use sincere::App;
use sincere::Group;
use sincere::Context;

pub struct User;

impl User {
    pub fn list(mut context: &mut Context) {
        ...
    }

    pub fn detail(mut context: &mut Context) {
        ...
    }

    pub fn handle() -> Group {
        let mut group = Group::new("/user");

        group.get("/", Self::list);
        group.get("/{id:[0-9]+}", Self::detail);

        group
    }
}

fn main() {
    let mut app = App::new();

    app.get("/", |context| {
        context.response.from_text("Hello world!").unwrap();
    });

    app.mount(User::handle());

    app.run("127.0.0.1:8000");
}
```

### Middleware

```rust
use sincere::App;

fn main() {
    let mut app = App::new();

    app.begin(|context| {
        ...
    });

    app.before(|context| {
        ...
    });

    app.after(|context| {
        ...
    });

    app.finish(|context| {
        ...
    });


    app.get("/", |context| {
        context.response.from_text("Hello world!").unwrap();
    });

    app.run("127.0.0.1:8000");
}
```

```rust
...
app.begin(...).begin(...);

app.begin(...).finish(...);

app.begin(...).before(...).after(...).finish(...);
...
```

```rust
app.get("/", |context| {
    context.response.from_text("Hello world!").unwrap();
}).before(...).after(...);
```

```rust
let mut group = Group::new("/user");

group.before(...).after(...);

group.get("/", |context| {
    context.response.from_text("Hello world!").unwrap();
}).before(...).after(...);
```

```rust
pub fn auth(context: &mut Context) {
    if let Some(token) = context.request.get_header("Token") {
        match token::verify_token(token) {
            Ok(id) => {
                context.contexts.insert("id".to_owned(), Value::String(id));
            },
            Err(err) => {
                context.response.from_text("").unwrap();
                context.stop();
            }
        }
    } else {
        context.response.status_code(401);
        context.stop();
    }
}

app.post("/article", ...).before(auth);

group.post("/", ...).before(auth);

```

```rust
pub fn cors(app: &mut App) {

    app.begin(move |context| {
        if context.request.method() == &Method::Options {
            context.response
            .status_code(204)
            .header(("Access-Control-Allow-Methods", "GET,HEAD,PUT,PATCH,POST,DELETE,OPTIONS"));

            context.stop();
        }
    });

    app.finish(move |context| {
        context.response
        .header(("Access-Control-Allow-Origin", "*"))
        .header(("Access-Control-Allow-Headers", "content-type, token"));
    });
}

app.use_middleware(cors);
```

### Path Parameters

```rust
app.get("/user/{id}", |context| {
    let id = context.request.param("id").unwrap();
});

app.get("/user/{id:[0-9]+}", |context| {
    let id = context.request.param("id").unwrap();
});

app.get("/user/{id:[a-z0-9]{24}}", |context| {
    let id = context.request.param("id").unwrap();
});
```

### Query Parameters

`/article?per_page=10&page=1`

```rust
app.get("/article", |content| {
    let page = context.request.query("page").unwrap_or("1");
    let per_page = context.request.query("per_page").unwrap_or("10");
});
```

### Bind Json

```toml
serde_derive = "1.0"

[dependencies.serde_json]
version = "1.0"
features = ["preserve_order"]
```

```rust
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
```

```rust
app.post("/article", |content| {

    #[derive(Deserialize, Debug)]
    struct New {
        title: String,
        image: Vec<String>,
        content: String
    }

    let new_json = context.request.bind_json::<New>().unwrap();

    // some code

    let return_json = json!({
        "article_id": 123
    });

    context.response.from_json(return_json).unwrap();
});
```

### Get and set headers, http status code

```rust
app.get("/", |context| {
    let token = context.request.header("Token").unwrap_or("none".to_owned());

    context.response.from_text("Hello world!").status_code(200).header(("Hello", "World")).unwrap();
});
```
