use super::bstring::BString;
use super::eval::{PortVal, Scope, StackTrace, ValRef};
use std::cell::RefCell;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;

struct TextFile {
    f: fs::File,
}

impl PortVal for TextFile {
    fn read(&mut self) -> Result<ValRef, String> {
        let mut buf = Vec::new();
        match self.f.read_to_end(&mut buf) {
            Ok(_) => (),
            Err(err) => return Err(err.to_string()),
        };

        Ok(ValRef::String(Rc::new(BString::from_vec(buf))))
    }

    fn write(&mut self, val: &ValRef) -> Result<(), String> {
        let res = match val {
            ValRef::String(s) => self.f.write(s.as_bytes()),
            val => self.f.write(format!("{}", val).as_bytes()),
        };

        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn seek(&mut self, pos: io::SeekFrom) -> Result<(), String> {
        match self.f.seek(pos) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

pub fn lib_open(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 1 {
        return Err(StackTrace::from_str("'open' requires 1 argument"));
    }

    let path = match &args[0] {
        ValRef::String(s) => s,
        _ => {
            return Err(StackTrace::from_str(
                "'open' requires the first argument to be a string",
            ))
        }
    };

    let f = match fs::File::open(path.to_path()) {
        Ok(f) => f,
        Err(err) => {
            return Err(StackTrace::from_string(format!(
                "'open': {}: {}",
                path, err
            )))
        }
    };

    Ok(ValRef::Port(Rc::new(RefCell::new(TextFile { f }))))
}

pub fn lib_create(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 1 {
        return Err(StackTrace::from_str("'create' requires 1 argument"));
    }

    let path = match &args[0] {
        ValRef::String(s) => s,
        _ => {
            return Err(StackTrace::from_str(
                "'create' requires the first argument to be a string",
            ))
        }
    };

    let f = match fs::File::create(path.to_path()) {
        Ok(f) => f,
        Err(err) => {
            return Err(StackTrace::from_string(format!(
                "'create': {}: {}",
                path, err
            )))
        }
    };

    Ok(ValRef::Port(Rc::new(RefCell::new(TextFile { f }))))
}

struct ChildProc {
    c: Child,
}

impl PortVal for ChildProc {
    fn read(&mut self) -> Result<ValRef, String> {
        let stdout = match &mut self.c.stdout {
            Some(stdout) => stdout,
            None => return Err("Child proc has no stdout".to_string()),
        };

        let mut buf = Vec::new();
        match stdout.read_to_end(&mut buf) {
            Ok(_) => (),
            Err(err) => return Err(err.to_string()),
        };

        Ok(ValRef::String(Rc::new(BString::from_vec(buf))))
    }

    fn write(&mut self, val: &ValRef) -> Result<(), String> {
        let stdin = match &mut self.c.stdin {
            Some(stdin) => stdin,
            None => return Err("Child proc has no stdin".to_string()),
        };

        let res = match val {
            ValRef::String(s) => stdin.write(s.as_bytes()),
            val => stdin.write(format!("{}", val).as_bytes()),
        };

        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

pub fn lib_exec(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.is_empty() {
        return Err(StackTrace::from_str("'exec' requires at least 1 argument"));
    }

    let name = match &args[0] {
        ValRef::String(s) => s,
        _ => {
            return Err(StackTrace::from_str(
                "'exec' requires its arguments to be strings",
            ))
        }
    };

    let mut cmd = Command::new(name.to_os_str());
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    for item in args.into_iter().skip(1) {
        match item {
            ValRef::String(s) => cmd.arg(s.to_os_str()),
            _ => {
                return Err(StackTrace::from_str(
                    "'exec' requires its arguments to be strings",
                ))
            }
        };
    }

    match cmd.spawn() {
        Err(err) => Err(StackTrace::from_string(format!("exec: {}", err))),
        Ok(child) => Ok(ValRef::Port(Rc::new(RefCell::new(ChildProc { c: child })))),
    }
}

pub fn init(scope: &Rc<RefCell<Scope>>) {
    let mut s = scope.borrow_mut();
    s.put_func("open", Rc::new(lib_open));
    s.put_func("create", Rc::new(lib_create));
    s.put_func("exec", Rc::new(lib_exec));
}
