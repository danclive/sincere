use std::collections::HashMap;
use std::ascii::AsciiExt;

use http::Method;
use http::Request;
use http::Response;
use super::Handle;

pub struct Route {
    pub pattern: String,
    pub method: Method,
    pub handle: Box<Handle>,
    pub compilied_pattern: String,
    pub paths: HashMap<String, usize>,
}

impl Route {
    pub fn new(method: Method, pattern: String, handle: Box<Handle>) -> Route {
        let mut route = Route {
            pattern: pattern.clone(),
            method: method,
            handle: handle,
            compilied_pattern: String::default(),
            paths: HashMap::new(),
        };

        route.re_connfigure(pattern);

        route
    }

    pub fn name(&mut self, name: &str) {
        println!("{:?}", name);
        println!("{:?}", self.method);
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

pub struct Group {
    pub routes: Vec<Route>,
    prefix: String,
}

impl Group {
    pub fn new(prefix: &str) -> Group {
        Group {
            routes: Vec::new(),
            prefix: prefix.to_owned(),
        }
    }
    fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(), 
            self.prefix.clone() + pattern, 
            Box::new(handle),
        );

        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    pub fn get<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("GET", pattern, handle)
    }

    pub fn post<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("POST", pattern, handle)
    }

    pub fn put<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("PUT", pattern, handle)
    }

    pub fn delete<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("DELETE", pattern, handle)
    }

    pub fn option<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("OPTION", pattern, handle)
    }

    pub fn head<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("HEAD", pattern, handle)
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
