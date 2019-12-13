use sincere::App;

fn main() {
    let mut app = App::new();

    app.get("/", |context| {

        println!("{:?}", context.request);
        context.response.from_text("Hello world!").unwrap();
    });

    app.run("0.0.0.0:10001").unwrap();
}
