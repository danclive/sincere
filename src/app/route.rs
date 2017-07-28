use std::collections::HashMap;
use std::ascii::AsciiExt;

use http::Method;
use super::Handle;
use super::context::Context;
use super::middleware::Middleware;

pub struct Route {
    pattern: String,
    method: Method,
    handle: Box<Handle>,
    compilied_pattern: String,
    paths: HashMap<String, usize>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
}

impl Route {
    pub fn new(method: Method, pattern: String, handle: Box<Handle>) -> Route {
        let mut route = Route {
            pattern: pattern.clone(),
            method: method,
            handle: handle,
            compilied_pattern: String::default(),
            paths: HashMap::new(),
            before: Vec::new(),
            after: Vec::new(),
        };

        route.re_connfigure(pattern);

        route
    }

    pub fn pattern(&self) -> &String {
        &self.pattern
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn compilied_pattern(&self) -> String {
        self.compilied_pattern.clone()
    }

    pub fn path(&self) -> HashMap<String, usize> {
        self.paths.clone()
    }

    pub fn execute(&self, context: &mut Context) {
        for before in self.before.iter() {
            before.execute(context);
        }

        if context.next() {
            (self.handle)(context);
        }

        for after in self.after.iter() {
            after.execute(context);
        }
    }

    pub fn before<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.before.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn after<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.after.push(Middleware {
            inner: Box::new(handle),
        });
    }

    fn re_connfigure(&mut self, pattern: String) {
        
        let prce_pattern;

        if pattern.contains("{") {
            let (route, route_paths) = extract_named_params(&pattern).unwrap();
            self.paths = route_paths;

            prce_pattern = route;
        } else {
            prce_pattern = pattern;
        }

        self.compilied_pattern = compile_pattern(prce_pattern);
    }
}

fn extract_named_params(pattern: &str) -> Result<(String, HashMap<String, usize>), ()> {
    
    let mut parenthese_count = 0;
    let mut bracket_count = 0;
    let mut intermediate = 0;
    let mut marker = 0;  
    let mut number_matches = 0;
    let mut tmp;
    let mut found_pattern;
    
    let mut prev_ch = '\0';
    let mut variable;
    let mut regexp;
    let mut item;
    let mut route = "".to_string();

    let mut not_valid = false;

    let mut matches = HashMap::new();

    if !pattern.is_ascii() {
        panic!("{:?}", "The ruote pattern must be an ascii");
    }
    
    for (cursor, ch) in pattern.chars().enumerate() {
        if parenthese_count == 0 {
            if ch == '{' {
                if bracket_count == 0 {
                    marker = cursor + 1;
                    intermediate = 0;
                    not_valid = false;
                }

                bracket_count += 1;
            } else {
                if ch == '}' {
                    bracket_count -= 1;
                    if intermediate > 0 {
                        if bracket_count == 0 {

                            number_matches += 1;
                            variable = "";
                            regexp = "";
                            item = &pattern[marker..cursor];

                            for (cursor_var, ch) in item.chars().enumerate() {
                                if ch == '\0' {
                                    break;
                                }

                                if cursor_var == 0 && !( (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')) {
                                    not_valid = true;
                                    break;
                                }

                                if (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch == '-' || ch == '_' || ch == ':' {
                                    if ch == ':' {
                                        let (first, last) = item.split_at(cursor_var);
                                        variable = first;
                                        regexp = &last[1..];
                                        break;
                                    }
                                } else {
                                    not_valid = true;
                                    break;
                                }
                            }

                            if !not_valid {
                                tmp = number_matches;
                                if !variable.is_empty() && !regexp.is_empty() {

                                    found_pattern = 0;
                                    for regexp_ch in regexp.chars() {
                                        if regexp_ch == '\0' {
                                            break;
                                        }

                                        if found_pattern == 0 {
                                            if regexp_ch == '(' {
                                                found_pattern = 1;
                                            }
                                        } else {
                                            if regexp_ch == ')' {
                                                found_pattern = 2;
                                                break;
                                            }
                                        }
                                    }

                                    if found_pattern != 2 {
                                        route.push('(');
                                        route += regexp;
                                        route.push(')');
                                    } else {
                                        route += regexp;
                                    }
                                    matches.insert(variable.to_string(), tmp);
                                } else {
                                    route += "([^/]*)";
                                    matches.insert(item.to_string(), tmp);
                                }
                                
                            } else {
                                route.push('{');
                                route += item;
                                route.push('}');
                            }
                            continue;
                        }
                    }

                }
            }
        }

        if bracket_count == 0 {
            if ch == '(' {
                parenthese_count += 1;
            } else {
                if ch == ')' {
                    parenthese_count -= 1;
                    if parenthese_count == 0 {
                        number_matches += 1;
                    }
                }
            }
        }

        if bracket_count > 0 {
            intermediate += 1;
        } else {
            if parenthese_count == 0 && prev_ch != '\\' {
                if ch == '.' || ch == '+' || ch == '|' || ch == '#' {
                    route = route + "\\";
                }
            }
            route.push(ch);
            prev_ch = ch;
        }
    }

    Ok((route, matches))
}

fn compile_pattern(pattern: String) -> String {
    
    let mut tmp = String::default();
    
    if pattern.contains("(") || pattern.contains("["){
        tmp.push('^');
        tmp += &pattern;
        tmp.push('$');

        return tmp;
    }

    pattern
}
