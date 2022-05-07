use super::eval::{Scope, StackTrace, ValRef};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

fn write_val<W>(w: &mut W, val: &ValRef, parent: String) -> Result<String, io::Error>
where
    W: io::Write,
{
    let name;
    match val {
        ValRef::None => {
            name = parent;
            writeln!(w, "{} [label=\"None\" shape=box]", name)?;
        }
        ValRef::Number(num) => {
            name = parent;
            writeln!(w, "{} [label=\"{}\" shape=box]", name, num)?;
        }
        ValRef::Bool(b) => {
            name = parent;
            writeln!(w, "{} [label=\"{}\" shape=box]", name, b)?;
        }
        ValRef::String(s) => {
            name = format!("v{:p}", s.as_ref());
            writeln!(
                w,
                "{} [label=\"string rc={}\"]",
                name,
                Rc::strong_count(s)
            )?;
            writeln!(w, "{}c [label={:?} shape=box]", name, s.as_ref())?;
            writeln!(w, "{} -> {}c [label=\"::content\"]", name, name)?;
        }
        ValRef::Block(b) => {
            name = format!("v{:p}", b.as_ref());
            writeln!(w, "{} [label=\"block rc={}\"]", name, Rc::strong_count(b))?;
        }
        ValRef::List(l) => {
            name = format!("v{:p}", l.as_ref());
            writeln!(w, "{} [label=\"list rc={}\"]", name, Rc::strong_count(l))?;

            let vec = l.borrow();
            for idx in 0..vec.len() {
                let n = write_val(w, &vec[idx], format!("{}v{}", name, idx))?;
                writeln!(w, "{} -> {} [label=\"[{}]\"]", name, n, idx)?;
            }
        }
        ValRef::Dict(d) => {
            name = format!("v{:p}", d.as_ref());
            writeln!(w, "{} [label=\"dict rc={}\"]", name, Rc::strong_count(d))?;

            let map = d.borrow();
            for (idx, (key, val)) in map.iter().enumerate() {
                let n = write_val(w, val, format!("{}v{}", name, idx))?;
                writeln!(w, "{} -> {} [label={:?}]", name, n, key)?;
            }
        }
        ValRef::Func(f) => {
            name = format!("v{:p}", f.as_ref());
            writeln!(w, "{} [label=\"func rc={}\"]", name, Rc::strong_count(f))?;
        }
        ValRef::Lambda(l) => {
            name = format!("v{:p}", l.as_ref());
            writeln!(
                w,
                "{} [label=\"lambda rc={}\"]",
                name,
                Rc::strong_count(l)
            )?;
        }
        ValRef::Binding(b, func) => {
            name = parent;
            writeln!(w, "{} [label=\"binding\"]", name)?;

            for (idx, (key, val)) in b.as_ref().iter().enumerate() {
                let n = write_val(w, val, format!("{}v{}", name, idx))?;
                writeln!(w, "{} -> {} [label={:?}]", name, n, key)?;
            }

            let n = write_val(w, func.as_ref(), format!("{}f", name))?;
            writeln!(w, "{} -> {} [label=\"::func\"]", name, n)?;
        }
        ValRef::Lazy(l) => {
            name = format!("v{:p}", l.as_ref());
            writeln!(w, "{} [label=\"lazy rc={}\"]", name, Rc::strong_count(l))?;
        }
        ValRef::ProtectedLazy(p) => {
            name = format!("v{:p}", p.as_ref());
            let lname = write_val(w, p.as_ref(), format!("{}l", name))?;
            writeln!(w, "{} [label=\"protected lazy\"]", name)?;
            writeln!(w, "{} -> {} [label=\"::lazy\"]", name, lname)?;
        }
        ValRef::Native(n) => {
            name = format!("v{:p}", n.as_ref());
            writeln!(
                w,
                "{} [label=\"native rc={}\"]",
                name,
                Rc::strong_count(n)
            )?;
        }
        ValRef::Port(p) => {
            name = format!("v{:p}", p.as_ref());
            writeln!(w, "{} [label=\"port rc={}\"]", name, Rc::strong_count(p))?;
        }
    }

    Ok(name)
}

fn write_scope<W>(w: &mut W, scope: &Rc<RefCell<Scope>>) -> Result<(), io::Error>
where
    W: io::Write,
{
    writeln!(w, "s{:p} [label=\"scope\"]", scope.as_ref())?;

    let s = scope.borrow();
    for (idx, (key, val)) in s.map.iter().enumerate() {
        let name = write_val(w, val, format!("s{:p}v{}", scope.as_ref(), idx))?;
        writeln!(
            w,
            "s{:p} -> {} [label={:?} type=s]",
            scope.as_ref(),
            name,
            key
        )?;
    }

    match &scope.borrow().parent {
        None => (),
        Some(parent) => {
            if parent.borrow().parent.is_some() {
                write_scope(w, parent)?;
                writeln!(
                    w,
                    "s{:p} -> s{:p} [label=\"::parent\"]",
                    scope.as_ref(),
                    parent.as_ref()
                )?;
            }
        }
    };

    Ok(())
}

pub fn write_dot<W>(w: &mut W, scope: &Rc<RefCell<Scope>>) -> Result<(), io::Error>
where
    W: io::Write,
{
    writeln!(w, "digraph d {{")?;
    write_scope(w, scope)?;
    writeln!(w, "}}")
}

fn lib_print_scope_dot(_: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    match write_dot(&mut io::stdout(), scope) {
        Ok(()) => Ok(ValRef::None),
        Err(err) => Err(StackTrace::from_string(err.to_string())),
    }
}

pub fn init(scope: &Rc<RefCell<Scope>>) {
    let mut s = scope.borrow_mut();
    s.put_func("print-scope-dot", Rc::new(lib_print_scope_dot));
}
