use super::bstring::BString;
use super::eval::{self, Scope, ValRef, PortVal, StackTrace};

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

fn lib_print(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let stdout = match scope.borrow().lookup(&BString::from_str("stdout")) {
        Some(stdout) => stdout,
        None => return Err(StackTrace::new("'print' expects a variable 'stdout' to be defined".into())),
    };

    let stdout = match stdout {
        ValRef::Port(port) => port,
        _ => return Err(StackTrace::new("'print' expects 'stdout' to be a port".into())),
    };
    let mut out = stdout.borrow_mut();

    let space = ValRef::String(Rc::new(BString::from_str(" ")));
    for idx in 0..args.len() {
        if idx != 0 {
            match out.write(&space) {
                Ok(_) => (),
                Err(err) => return Err(StackTrace::new(err)),
            };
        }

        match out.write(&args[idx]) {
            Ok(_) => (),
            Err(err) => return Err(StackTrace::new(err)),
        };
    }
    match out.write(&ValRef::String(Rc::new(BString::from_str("\n")))) {
        Ok(_) => (),
        Err(err) => return Err(StackTrace::new(err)),
    }

    Ok(ValRef::None)
}

fn lib_add(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num += &args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

fn lib_sub(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num -= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

fn lib_mul(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num *= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

fn lib_div(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num /= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

fn lib_equals(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() <= 1 {
        return Ok(ValRef::Bool(true));
    }

    for idx in 0..args.len() - 1 {
        if !ValRef::equals(&args[idx], &args[idx + 1]) {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

fn lib_nequals(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    match lib_equals(args, scope) {
        Ok(ValRef::Bool(true)) => Ok(ValRef::Bool(false)),
        Ok(ValRef::Bool(false)) => Ok(ValRef::Bool(true)),
        val => val,
    }
}

fn lib_lteq(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if args[idx].to_num() > args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

fn lib_lt(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if args[idx].to_num() >= args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

fn lib_gteq(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if args[idx].to_num() < args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

fn lib_gt(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if args[idx].to_num() <= args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

fn lib_or(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if args[idx].to_bool() {
            return Ok(args[idx].clone());
        }
    }

    Ok(args[args.len() - 1].clone())
}

fn lib_and(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() - 1 {
        if !args[idx].to_bool() {
            return Ok(args[idx].clone());
        }
    }

    Ok(args[args.len() - 1].clone())
}

fn lib_first(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for arg in args {
        match arg {
            ValRef::None => (),
            _ => return Ok(arg.clone()),
        }
    }

    Ok(ValRef::None)
}

fn lib_def(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 2 {
        return Err(StackTrace::new("'def' requires 2 arguments".into()));
    }

    let name = match &args[0] {
        ValRef::String(s) => s.as_ref(),
        _ => return Err(StackTrace::new("'def' requires the first argument to be a string".into())),
    };

    scope.borrow_mut().insert(name.clone(), args[1].clone());
    Ok(ValRef::None)
}

fn lib_set(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 2 {
        return Err(StackTrace::new("'set' requires 2 arguments".into()));
    }

    let name = match &args[0] {
        ValRef::String(s) => s.as_ref(),
        _ => return Err(StackTrace::new("'set' requires the first argument to be a string".into())),
    };

    if scope.borrow_mut().replace(name.clone(), args[1].clone()) {
        Ok(ValRef::None)
    } else {
        Err(StackTrace::new(format!("Variable '{}' doesn't exist", name)))
    }
}

fn lib_if(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 2 && args.len() != 3 {
        return Err(StackTrace::new("'if' requires 2 or 3 arguments".into()));
    }

    let expr;
    if args[0].to_bool() {
        expr = &args[1];
    } else if args.len() == 3 {
        expr = &args[2];
    } else {
        return Ok(ValRef::None);
    }

    expr.call_or_get(scope)
}

fn lib_match(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for arg in args {
        let exprs = match arg {
            ValRef::Quote(exprs) => exprs,
            _ => return Err(StackTrace::new("'match' requires all arguments to be quotes".into())),
        };

        if exprs.len() < 1 {
            return Err(StackTrace::new("'match' requires all arguments to have at least 1 element".into()));
        }

        if eval::eval(&exprs[0], scope)?.to_bool() {
            return eval::eval_multiple(&exprs[1..], scope);
        }
    }

    Ok(ValRef::None)
}

fn lib_while(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 1 && args.len() != 2 {
        return Err(StackTrace::new("'while' requires 1 or 2 arguments".into()));
    }

    let cond = match &args[0] {
        ValRef::Quote(func) => func,
        _ => return Err(StackTrace::new("'while' expects the firt argument to be a function".into())),
    };

    let body = if args.len() >= 1 {
        match &args[1] {
            ValRef::Quote(func) => Some(func),
            _ => return Err(StackTrace::new("'while' expects the second argument to be a function".into())),
        }
    } else {
        None
    };

    let mut retval: ValRef = ValRef::None;
    loop {
        if !eval::eval_call(cond.as_ref(), scope)?.to_bool() {
            return Ok(retval);
        }

        match body {
            Some(body) => retval = eval::eval_call(body, scope)?,
            _ => (),
        };
    }
}

fn lib_do(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() > 0 {
        Ok(args[args.len() - 1].clone())
    } else {
        Ok(ValRef::None)
    }
}

fn lib_bind(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Err(StackTrace::new("'bind' requires at least 1 argument".into()));
    }

    let vals = match &args[0] {
        ValRef::List(l) => l,
        _ => return Err(StackTrace::new("'bind' expects first argument to be a list".into())),
    };

    let mut argidx = 0;
    for idx in 1..args.len() - 1 {
        let arg = &args[idx];
        match arg {
            ValRef::String(name) => {
                if argidx >= vals.len() {
                    return Err(StackTrace::new("'bind': Wrong argument count".into()));
                }

                scope
                    .borrow_mut()
                    .insert(name.as_ref().clone(), vals[argidx].clone());
                argidx += 1;
            }
            _ => return Err(StackTrace::new("'bind' expects strings only".into())),
        }
    }

    match &args[args.len() - 1] {
        ValRef::Quote(q) => eval::eval_call(q.as_ref(), scope),
        _ => return Err(StackTrace::new("'bind' expects its last argument to be a quote".into())),
    }
}

fn lib_with(args: &[ValRef], scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut idx = 0;
    while idx < args.len() - 1 {
        let name = match &args[idx] {
            ValRef::String(s) => s,
            _ => return Err(StackTrace::new("'with' requires names to be string".into())),
        };

        idx += 1;
        let val = &args[idx];
        idx += 1;

        scope
            .borrow_mut()
            .insert(name.as_ref().clone(), val.clone());
    }

    match &args[args.len() - 1] {
        ValRef::Quote(q) => eval::eval_call(q.as_ref(), scope),
        _ => return Err(StackTrace::new("'bind' expects its last argument to be a quote".into())),
    }
}

fn lib_read(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 1 && args.len() != 2 {
        return Err(StackTrace::new("'read' requires 1 or 2 arguments".into()));
    }

    let port = match &args[0] {
        ValRef::Port(port) => port,
        _ => return Err(StackTrace::new("'read' requires the first argument to be a port".into())),
    };

    if args.len() == 1 {
        match port.borrow_mut().read() {
            Ok(val) => Ok(val),
            Err(err) => Err(StackTrace::new(err)),
        }
    } else {
        let size = match args[1] {
            ValRef::Number(num) => num,
            _ => return Err(StackTrace::new("'read' requires the second argument to be a number".into())),
        };

        match port.borrow_mut().read_chunk(size as usize) {
            Ok(val) => Ok(val),
            Err(err) => Err(StackTrace::new(err)),
        }
    }
}

fn lib_write(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 2 {
        return Err(StackTrace::new("'write' requires 2 arguments".into()));
    }

    let port = match &args[0] {
        ValRef::Port(port) => port,
        _ => return Err(StackTrace::new("'write' requires the first argument to be a port".into())),
    };

    match port.borrow_mut().write(&args[1]) {
        Ok(_) => Ok(ValRef::None),
        Err(err) => Err(StackTrace::new(err)),
    }
}

fn lib_seek(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 2 && args.len() != 3 {
        return Err(StackTrace::new("'seek' requires 2 or 3 arguments".into()));
    }

    let port = match &args[0] {
        ValRef::Port(port) => port,
        _ => return Err(StackTrace::new("'seek' requires the first argument to be a port".into())),
    };

    let num = match &args[1] {
        ValRef::Number(num) => *num,
        _ => return Err(StackTrace::new("'seek' requires the second argument to be a number".into())),
    };

    let pos = if args.len() == 2 {
        io::SeekFrom::Start(num as u64)
    } else {
        let name = match &args[2] {
            ValRef::String(s) => s,
            _ => return Err(StackTrace::new("'seek' requires the third argument to be a string".into())),
        };

        match name.as_bytes() {
            b"set" => io::SeekFrom::Start(num as u64),
            b"end" => io::SeekFrom::End(num as i64),
            b"current" => io::SeekFrom::Current(num as i64),
            _ => {
                return Err(StackTrace::new("'seek' requires the seek offset to be 'set', 'end' or 'current'".into()),
                )
            }
        }
    };

    match port.borrow_mut().seek(pos) {
        Ok(_) => Ok(ValRef::None),
        Err(err) => Err(StackTrace::new(err)),
    }
}

fn lib_lazy(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() != 1 {
        return Err(StackTrace::new("'lazy' requires 1 argument".into()));
    }

    Ok(ValRef::ProtectedLazy(Rc::new(args[0].clone())))
}

fn lib_lambda(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut argnames: Vec<BString> = Vec::new();
    for idx in 0..args.len() {
        match &args[idx] {
            eval::ValRef::String(bs) => argnames.push(bs.as_ref().clone()),
            eval::ValRef::Quote(q) => {
                if idx != args.len() - 1 {
                    return Err(StackTrace::new("'lambda' requires the quote to be the last argument".into()));
                }

                return Ok(eval::ValRef::Lambda(Rc::new(eval::LambdaVal{
                    args: argnames,
                    body: q.clone(),
                })));
            },
            _ => return Err(StackTrace::new("'lambda' requires arguments to be quotes or strings".into())),
        }
    }

    Err(StackTrace::new("'lambda' requires a quote argument".into()))
}

fn lib_list(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    Ok(ValRef::List(Rc::new(args.to_vec())))
}

fn lib_dict(args: &[ValRef], _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() % 2 != 0 {
        return Err(StackTrace::new("'dict' requires an even number of arguments".into()));
    }

    let mut map: HashMap<BString, ValRef> = HashMap::new();
    let mut idx = 0;
    while idx < args.len() {
        let key = &args[idx];
        idx += 1;
        let val = &args[idx];
        idx += 1;

        let keystr = match key {
            ValRef::String(s) => s,
            _ => return Err(StackTrace::new("'dict' requires keys to be strings".into())),
        };

        map.insert(keystr.as_ref().clone(), val.clone());
    }

    Ok(ValRef::Map(Rc::new(map)))
}

pub struct StdIo {
    pub stdin: Rc<RefCell<dyn PortVal>>,
    pub stdout: Rc<RefCell<dyn PortVal>>,
    pub stderr: Rc<RefCell<dyn PortVal>>,
}

pub fn init_with_stdio(scope: &Rc<RefCell<Scope>>, stdio: StdIo) {
    let mut s = scope.borrow_mut();
    s.put("stdin", ValRef::Port(stdio.stdin));
    s.put("stdout", ValRef::Port(stdio.stdout));
    s.put("stderr", ValRef::Port(stdio.stderr));

    s.put("none", ValRef::None);
    s.put("false", ValRef::Bool(false));
    s.put("true", ValRef::Bool(true));

    s.put_func("print", Rc::new(lib_print));
    s.put_func("+", Rc::new(lib_add));
    s.put_func("-", Rc::new(lib_sub));
    s.put_func("*", Rc::new(lib_mul));
    s.put_func("/", Rc::new(lib_div));
    s.put_func("==", Rc::new(lib_equals));
    s.put_func("!=", Rc::new(lib_nequals));
    s.put_func("<=", Rc::new(lib_lteq));
    s.put_func("<", Rc::new(lib_lt));
    s.put_func(">=", Rc::new(lib_gteq));
    s.put_func(">", Rc::new(lib_gt));
    s.put_func("||", Rc::new(lib_or));
    s.put_func("&&", Rc::new(lib_and));
    s.put_func("??", Rc::new(lib_first));
    s.put_func("def", Rc::new(lib_def));
    s.put_func("set", Rc::new(lib_set));
    s.put_func("if", Rc::new(lib_if));
    s.put_func("match", Rc::new(lib_match));
    s.put_func("while", Rc::new(lib_while));
    s.put_func("do", Rc::new(lib_do));
    s.put_func("bind", Rc::new(lib_bind));
    s.put_func("with", Rc::new(lib_with));
    s.put_func("read", Rc::new(lib_read));
    s.put_func("write", Rc::new(lib_write));
    s.put_func("seek", Rc::new(lib_seek));

    s.put_func("lambda", Rc::new(lib_lambda));

    s.put_func("lazy", Rc::new(lib_lazy));

    s.put_func("list", Rc::new(lib_list));

    s.put_func("dict", Rc::new(lib_dict));
}

pub struct WritePort {
    w: Rc<RefCell<dyn io::Write>>,
}

impl WritePort {
    pub fn new(w: Rc<RefCell<dyn io::Write>>) -> Self {
        Self { w }
    }
}

impl PortVal for WritePort {
    fn write(&mut self, v: &ValRef) -> Result<(), String> {
        let res = match v {
            ValRef::String(s) => write!(self.w.borrow_mut(), "{}", s),
            _ => write!(self.w.borrow_mut(), "{}", v),
        };
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

pub struct ReadPort {
    r: Rc<RefCell<dyn io::Read>>,
}

impl ReadPort {
    pub fn new(r: Rc<RefCell<dyn io::Read>>) -> Self {
        Self { r }
    }
}

impl PortVal for ReadPort {
    fn read(&mut self) -> Result<ValRef, String> {
        let mut buf = [0u8; 4096];
        let size = match self.r.borrow_mut().read(&mut buf[..]) {
            Ok(size) => size,
            Err(err) => return Err(err.to_string()),
        };

        Ok(ValRef::String(Rc::new(BString::from_bytes(&buf[..size]))))
    }
}

pub fn init(scope: &Rc<RefCell<Scope>>) {
    init_with_stdio(scope, StdIo {
        stdin: Rc::new(RefCell::new(ReadPort::new(Rc::new(RefCell::new(io::stdin()))))),
        stdout: Rc::new(RefCell::new(WritePort::new(Rc::new(RefCell::new(io::stdout()))))),
        stderr: Rc::new(RefCell::new(WritePort::new(Rc::new(RefCell::new(io::stderr()))))),
    })
}
