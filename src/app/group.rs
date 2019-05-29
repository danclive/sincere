/// Route group
use std::collections::HashMap;

use super::context::Context;
use super::middleware::Middleware;
use super::route::Route;
use crate::http::Method;

// use hyper::Method;

/// Route group
///
/// # Examples
///
/// ```
/// use sincere::App;
/// use sincere::app::Group;
///
/// let mut group = Group::new("/app");
///
/// group.get("/", |context| {
///     context.response.from_text("Hello world!").unwrap();
/// });
///
/// group.post("/", |context| {
///     context.response.from_text("Hello world!").unwrap();
/// });
///
/// let mut app = App::new();
///
/// app.mount_group(group);
/// ```
/// or
///
/// ```
/// use sincere::App;
///
/// let mut app = App::new();
///
/// app.mount("/app", |group| {
///
///     group.get("/", |context| {
///         context.response.from_text("Get method!").unwrap();
///     });
///
///     group.post("/", |context| {
///         context.response.from_text("Post method!").unwrap();
///     });
///
/// });
/// ```
pub struct Group {
    pub routes: HashMap<Method, Vec<Route>>,
    prefix: String,
    pub before: Vec<Middleware>,
    pub after: Vec<Middleware>,
}

impl Group {
    /// Create a route group.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::app::Group;
    ///
    /// let group = Group::new("/app");
    /// ```
    ///
    pub fn new(prefix: &str) -> Group {
        Group {
            routes: HashMap::new(),
            prefix: prefix.to_owned(),
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    /// Add route handle to group.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::app::Group;
    /// use sincere::http::Method;
    ///
    /// let mut group = Group::new("/app");
    ///
    /// group.add(Method::GET, "/", |context| {
    ///     context.response.from_text("Get method!").unwrap();
    /// });
    /// ```
    pub fn add<H>(&mut self, method: Method, pattern: &str, handle: H) -> &mut Route
    where
        H: Fn(&mut Context) + Send + Sync + 'static,
    {
        let route = Route::new(
            method.clone(),
            self.prefix.clone() + pattern,
            Box::new(handle),
        );

        let routes = self.routes.entry(method).or_insert(Vec::new());
        routes.push(route);
        routes.last_mut().unwrap()
    }

    route!(
        /// Add route handle to group with GET method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.get("/", |context| {
        ///     context.response.from_text("Get method!").unwrap();
        /// });
        /// ```
        get
    );

    route!(
        /// Add route handle to group with PUT method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.put("/", |context| {
        ///     context.response.from_text("Put method!").unwrap();
        /// });
        /// ```
        put
    );

    route!(
        /// Add route handle to group with POST method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.post("/", |context| {
        ///     context.response.from_text("Post method!").unwrap();
        /// });
        /// ```
        post
    );

    route!(
        /// Add route handle to group with HEAD method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.head("/", |context| {
        ///     // context.response.from_text("Head method!").unwrap();
        /// });
        /// ```
        head
    );

    route!(
        /// Add route handle to group with PATCH method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.patch("/", |context| {
        ///     context.response.from_text("Patch method!").unwrap();
        /// });
        /// ```
        patch
    );

    route!(
        /// Add route handle to group with TRACE method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.trace("/", |context| {
        ///     context.response.from_text("Trace method!").unwrap();
        /// });
        /// ```
        trace
    );

    route!(
        /// Add route handle to group with DELETE method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.delete("/", |context| {
        ///     context.response.from_text("Delete method!").unwrap();
        /// });
        /// ```
        delete
    );

    route!(
        /// Add route handle to group with OPTIONS method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.options("/", |context| {
        ///     context.response.from_text("Options method!").unwrap();
        /// });
        /// ```
        options
    );

    route!(
        /// Add route handle to group with CONNECT method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/group");
        ///
        /// group.connect("/", |context| {
        ///     context.response.from_text("Connect method!").unwrap();
        /// });
        /// ```
        connect
    );

    middleware!(
        /// Add `before handle` to group.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/app");
        ///
        /// group.before(|context| {
        ///     context.response.from_text("before!").unwrap();
        /// });
        /// ```
        before
    );

    middleware!(
        /// Add `after handle` to group.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::app::Group;
        ///
        /// let mut group = Group::new("/app");
        ///
        /// group.after(|context| {
        ///     context.response.from_text("after!").unwrap();
        /// });
        /// ```
        after
    );
}
