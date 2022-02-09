use super::ast;

use std::rc::Rc;
use std::collections::HashMap;
use std::fmt;

pub enum ValRef {
    None,
    Number(i32),
    String(Rc<String>),
    Quote(Rc<Vec<ast::Expression>>),
    List(Rc<Vec<ValRef>>),
    Func(&'static dyn Fn(&Vec<ValRef>) -> ValRef),
}

impl Clone for ValRef {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Number(num) => Self::Number(*num),
            Self::String(s) => Self::String(s.clone()),
            Self::Quote(q) => Self::Quote(q.clone()),
            Self::List(l) => Self::List(l.clone()),
            Self::Func(f) => Self::Func(*f),
        }
    }
}

impl fmt::Display for ValRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Number(num) => write!(f, "{}", num),
            Self::String(s) => write!(f, "{}", s),
            Self::Quote(q) => write!(f, "{:?}", q),
            Self::List(l) => {
                write!(f, "[")?;
                let vec = l.as_ref();
                for idx in 0..vec.len() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", vec[idx])?;
                }
                write!(f, "]")
            }
            Self::Func(_) => write!(f, "(func)"),
        }
    }
}

pub struct Scope {
    parent: Option<Rc<Scope>>,
    map: HashMap<String, ValRef>,
}

impl Scope {
    pub fn new(parent: Option<Rc<Scope>>) -> Self {
        Self {
            parent,
            map: HashMap::new(),
        }
    }

    fn lookup(&self, name: &String) -> Result<ValRef, String> {
        match self.map.get(name) {
            Some(r) => Ok(r.clone()),
            None => match &self.parent {
                Some(parent) => parent.lookup(name),
                None => Err(format!("Variable '{}' doesn't exist", name)),
            }
        }
    }

    pub fn insert(&mut self, name: String, val: ValRef) {
        self.map.insert(name, val);
    }
}

fn call(exprs: &Vec<ast::Expression>, scope: &Scope) -> Result<ValRef, String> {
    if exprs.len() < 1 {
        return Err("Call list has no elements".to_string());
    }

    let mut args: Vec<ValRef> = Vec::new();
    args.reserve(exprs.len() - 1);
    for idx in 1..exprs.len() {
        args.push(eval(&exprs[idx], scope)?);
    }

    let func = eval(&exprs[0], scope)?;
    match func {
        ValRef::Func(func) => Ok(func(&args)),
        ValRef::Quote(exprs) => {
            let mut retval = ValRef::None;
            for expr in exprs.as_ref() {
                retval = eval(expr, scope)?;
            }

            Ok(retval)
        }
        _ => Err("Attempt to call non-function".to_string()),
    }
}

pub fn eval(expr: &ast::Expression, scope: &Scope) -> Result<ValRef, String> {
    match expr {
        ast::Expression::String(s) => Ok(ValRef::String(Rc::new(s.clone()))),
        ast::Expression::Number(num) => Ok(ValRef::Number(*num)),
        ast::Expression::Name(name) => scope.lookup(name),
        ast::Expression::Call(exprs) => call(exprs, scope),
        ast::Expression::Quote(exprs) => Ok(ValRef::Quote(exprs.clone())),
    }
}
