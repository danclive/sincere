//! Route
use std::collections::HashMap;

use hyper::Method;
use regex::Regex;

use super::Handle;
use super::context::Context;
use super::middleware::Middleware;

/// Route
pub struct Route {
    pattern: String,
    method: Method,
    handle: Box<Handle>,
    pub(crate) regex: Option<Regex>,
    paths: HashMap<String, usize>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
}

impl Route {
    pub fn new(method: Method, pattern: String, handle: Box<Handle>) -> Route {
        let mut route = Route {
            pattern: pattern,
            method: method,
            handle: handle,
            regex: None,
            paths: HashMap::new(),
            before: Vec::new(),
            after: Vec::new(),
        };

        route.re_connfigure();

        route
    }

    pub fn pattern(&self) -> &String {
        &self.pattern
    }

    pub fn method(&self) -> &Method {
        &self.method
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

    middleware!(before);
    middleware!(after);

    fn re_connfigure(&mut self) {
        if self.pattern.contains("{") {
            let (prce_pattern, route_paths) = extract_named_params(&self.pattern);

            self.paths = route_paths;

            let compilied_pattern = compile_pattern(prce_pattern);

            if compilied_pattern.contains("^") {
                match Regex::new(&compilied_pattern) {
                    Ok(regex) => {
                        self.regex = Some(regex);
                    }
                    Err(err) => {
                        panic!("Can't complie route path: {:?}, err: {:?}", self.pattern, err);
                    }
                }
            } else {
                panic!("Can't complie route path: {:?}", self.pattern);
            }
        }
    }
}

fn extract_named_params(pattern: &str) -> (String, HashMap<String, usize>) {
    
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
    let mut route = String::new();

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

                                if cursor_var == 0 && !((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')) {
                                    not_valid = true;
                                    break;
                                }

                                if (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch == '-' || ch == '_' || ch == ':' {
                                    if ch == ':' {
                                        // let (first, last) = item.split_at(cursor_var);
                                        // variable = first;
                                        // regexp = &last[1..];
                                        variable = &item[..cursor_var];
                                        regexp = &item[cursor_var + 1..];
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

    (route, matches)
}

fn compile_pattern(pattern: String) -> String {
    
    if pattern.contains("(") || pattern.contains("["){
        let mut tmp = String::new();

        tmp.push('^');
        tmp += &pattern;
        tmp.push('$');

        return tmp;
    }

    pattern
}

#[test]
fn compile() {
    let pattern = "{year:[0-9]{4}}/{title:[a-zA-Z\\-]+}";

    let (route, route_paths) = extract_named_params(pattern);
    assert_eq!(route, "([0-9]{4})/([a-zA-Z\\-]+)");
    let mut map: HashMap<String, usize> = HashMap::new();
    map.insert("title".to_string(), 2);
    map.insert("year".to_string(), 1);
    assert_eq!(route_paths, map);
}
