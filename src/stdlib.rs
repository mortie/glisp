use super::bstring::BString;
use super::eval::{self, FuncArgs, PortVal, Scope, StackTrace, ValRef};

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::mem;
use std::rc::Rc;
use std::vec;

/*
@(print (arg:any)*)
    -> none

Print the arguments to 'stdout', separated by a space.
*/
fn lib_print(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let stdout = match scope.borrow().lookup(&BString::from_str("stdout")) {
        Some(stdout) => match stdout {
            ValRef::Port(port) => port,
            _ => {
                return Err(StackTrace::from_str(
                    "'print' expects 'stdout' to be a port",
                ))
            }
        },
        None => {
            return Err(StackTrace::from_str(
                "'print' expects a variable 'stdout' to be defined",
            ))
        }
    };

    let mut out = stdout.borrow_mut();

    let space = ValRef::String(Rc::new(BString::from_str(" ")));
    let mut first = true;
    while let Some(arg) = args.next() {
        if !first {
            match out.write(&space) {
                Ok(_) => (),
                Err(err) => return Err(StackTrace::from_string(err)),
            };
        }

        match out.write(&arg) {
            Ok(_) => (),
            Err(err) => return Err(StackTrace::from_string(err)),
        };

        first = false;
    }

    match out.write(&ValRef::String(Rc::new(BString::from_str("\n")))) {
        Ok(_) => (),
        Err(err) => return Err(StackTrace::from_string(err)),
    }

    Ok(ValRef::None)
}

/*
@(not val:bool)
    -> bool

Return a bool value that's the inverse of its argument.

Examples:
(not true) -> false
(not false) -> true
*/
fn lib_not(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let arg = args.next_val()?;
    args.done()?;
    Ok(ValRef::Bool(!arg.to_bool()))
}

/*
@(+ (val:number)*)
    -> number

Return all the numbers added together.

Examples:
(+ 10 20) -> 30
(+ 33) -> 33
(+ 1 2 3 4 5) -> 15
(+) -> 0
*/
fn lib_add(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num += &args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

/*
@(- (val:number)*)
    -> number

Return all subsequent numbers subtracted from the first number.
If there's only one argument, return the negative of that number.

Examples:
(- 10) -> -10
(- 10 3) -> 7
(- 10 2 3) -> 5
(-) -> 0
*/
fn lib_sub(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    } else if args.len() == 1 {
        return Ok(ValRef::Number(-args[0].to_num()));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num -= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

/*
@(* (val:number)*)
    -> number

Return all numbers multiplied by each other.

Examples:
(* 10) -> 10
(* 10 3) -> 30
(* 10 2 3) -> 60
(*) -> 0
*/
fn lib_mul(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num *= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

/*
@(/ (val:number)*)
    -> number

Return all subsequent numbers divided from the first one.
If there's only one argument, return the reciprocal of that number.

Examples:
(/ 10) -> 0.1
(/ 10 2) -> 5
(/ 30 3 2) -> 5
(/) -> 0
*/
fn lib_div(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 1 {
        return Ok(ValRef::Number(0.0));
    } else if args.len() == 1 {
        return Ok(ValRef::Number(1.0 / args[0].to_num()));
    }

    let mut num = args[0].to_num();
    for idx in 1..args.len() {
        num /= args[idx].to_num();
    }

    Ok(ValRef::Number(num))
}

/*
@(== (val:any)*)
    -> bool

Return true if all values are equal, false otherwise.

Examples:
(== 10 10) -> true
(== 20 10) -> false
(== "Hello" "Hello" "Hello") -> true
(== "Hello" "Hello" 11) -> false
(== "11" 11) -> false
(==) -> true
*/
fn lib_equals(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
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

/*
@(!= (val:any)*)
    -> bool

Return false if all values are equal, true otherwise.

Examples:
(!= 10 10) -> false
(!= 20 10) -> true
(!= "Hello" "Hello" "Hello") -> false
(!= "Hello" "Hello" 11) -> true
(!= "11" 11) -> true
(!=) -> false
*/
fn lib_nequals(args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    match lib_equals(args, scope) {
        Ok(ValRef::Bool(true)) => Ok(ValRef::Bool(false)),
        Ok(ValRef::Bool(false)) => Ok(ValRef::Bool(true)),
        val => val,
    }
}

/*
@(<= (val:number)*)
    -> bool

Returns true if every value is less than or equal to the value to its right.

Examples:
(<= 10 20 30) -> true
(<= 10 10 10) -> true
(<= 4 5) -> true
(<= 50 40 30) -> false
(<= 10 20 30 50 40) -> false
(<= 10) -> true
(<=) -> true
*/
fn lib_lteq(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() == 0 {
        return Ok(ValRef::Bool(true));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() > args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

/*
@(< (val:number)*)
    -> bool

Returns true if every value is less than the value to its right.

Examples:
(< 10 20 30) -> true
(< 10 10 10) -> false
(< 4 5) -> true
(< 50 40 30) -> false
(< 10 20 30 50 40) -> false
(< 10) -> true
(<) -> true
*/
fn lib_lt(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() == 0 {
        return Ok(ValRef::Bool(true));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() >= args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

/*
@(>= (val:number)*)
    -> bool

Returns true if every value is greater than or equal to the value to its right.

Examples:
(>= 10 20 30) -> false
(>= 10 10 10) -> true
(>= 4 5) -> false
(>= 50 40 30) -> true
(>= 10 20 30 50 40) -> false
(>= 10) -> true
(>=) -> true
*/
fn lib_gteq(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() == 0 {
        return Ok(ValRef::Bool(true));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() < args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

/*
@(> (val:number)*)
    -> bool

Returns true if every value is greater than the value to its right.

Examples:
(> 10 20 30) -> false
(> 10 10 10) -> false
(> 4 5) -> false
(> 50 40 30) -> true
(> 10 20 30 50 40) -> false
(> 10) -> true
(>) -> true
*/
fn lib_gt(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() == 0 {
        return Ok(ValRef::Bool(true));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() <= args[idx + 1].to_num() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

/*
@(|| (val:any)*)
    -> bool

Returns true if any argument is truthy, and false otherwise.
All values other than 'false' and 'none' are considered truthy.

Examples:
(|| "hello" false) -> true
(|| false false) -> false
(|| true) -> true
(|| true false true) -> true
(||) -> false
*/
fn lib_or(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() {
        if args[idx].to_bool() {
            return Ok(ValRef::Bool(true));
        }
    }

    Ok(ValRef::Bool(false))
}

/*
@(&& (val:any)*)
    -> bool

Returns false if any argument is falsy, and true otherwise.
The values 'false' and 'none' are considered falsy.

Examples:
(&& "hello" false) -> false
(&& false false) -> false
(&& true) -> true
(&& true true) -> true
(&& true false true) -> false
(&&) -> true
*/
fn lib_and(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for idx in 0..args.len() {
        if !args[idx].to_bool() {
            return Ok(ValRef::Bool(false));
        }
    }

    Ok(ValRef::Bool(true))
}

/*
@(?? (val:any)*)
    -> bool

Returns the first value that's not 'none'.

Examples:
(?? none 10 20) -> 10
(?? none) -> none
(?? "Hello" none "Goodbye") -> "Hello"
(?? none none none 3) -> 3
(??) -> none
*/
fn lib_first(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    for arg in args.drain(0..) {
        match arg {
            ValRef::None => (),
            _ => return Ok(arg),
        }
    }

    Ok(ValRef::None)
}

/*
@(def (name:string value:any)*)
    -> none

Defines the given values in the current scope.

Examples:
(def 'x 10)
(+ x 20) -> 30

(def 'x 40 'y 50)
(+ x y) -> 90
*/
fn lib_def(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let mut scopemut = scope.borrow_mut();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;
        scopemut.insert(key.as_ref().clone(), val);
    }

    Ok(ValRef::None)
}

/*
@(func name:string (arg:string)* body:block)
    -> none

Defines a lambda with the given name and parameters in the current scope.

Examples:
(func 'square 'x {
    [x * x]
})
(square 10) -> 100
(square 5) -> 25

(func 'add 'a 'b {
    [a + b]
})
(add 10 20) -> 30
(add 9 10) -> 19
*/
fn lib_func(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let name = args.next_val()?.get_string()?;

    let mut argnames: Vec<BString> = Vec::new();
    let mut block = None;
    while let Some(arg) = args.next() {
        match arg {
            ValRef::String(s) => argnames.push(s.as_ref().clone()),
            ValRef::Block(b) => {
                block = Some(b);
                break;
            }
            _ => {
                return Err(StackTrace::from_str("Expected string or block"));
            }
        }
    }

    args.done()?;
    let block = match block {
        Some(block) => block,
        None => return Err(StackTrace::from_str("Expected block")),
    };

    let val = ValRef::Lambda(Rc::new(eval::LambdaVal {
        args: argnames,
        body: block,
    }));
    scope.borrow_mut().insert(name.as_ref().clone(), val.clone());

    Ok(ValRef::None)
}

fn lib_set(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let mut scopemut = scope.borrow_mut();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;

        if !scopemut.replace(key.as_ref().clone(), val) {
            return Err(StackTrace::from_string(format!(
                "Variable '{}' doesn't exist",
                key
            )));
        }
    }

    Ok(ValRef::None)
}

fn lib_mutate(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 2 {
        return Err(StackTrace::from_str("Not enough arguments"));
    }

    let name = args[0].clone().get_string()?;

    let (val, s) = match Scope::rlookup(scope, name.as_ref()) {
        Some(val) => val,
        None => {
            return Err(StackTrace::from_string(format!(
                "Variable '{}' doesn't exist",
                name
            )))
        }
    };

    s.borrow_mut().remove(name.as_ref());

    // Replace the name and the mutator function with the value to be
    // passed as the first argument, so that we can re-use the args array
    let func = args.remove(1);
    args[0] = val;

    let res = eval::call(&func, args, scope)?;
    s.borrow_mut().insert(name.as_ref().clone(), res.clone());
    Ok(res)
}

fn lib_if(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let cond = args.next_val()?.to_bool();
    let if_body = args.next_val()?;
    let else_body = args.next();
    args.done()?;

    if cond {
        eval::call(&if_body, vec![], scope)
    } else if let Some(else_body) = else_body {
        eval::call(&else_body, vec![], scope)
    } else {
        Ok(ValRef::None)
    }
}

fn lib_match(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    while args.has_next() {
        let block = args.next_val()?.get_block()?;

        if block.len() < 1 {
            return Err(StackTrace::from_str("Blocks must have at least 1 element"));
        }

        if eval::eval(&block[0], scope)?.to_bool() {
            return eval::eval_multiple(&block[1..], scope);
        }
    }

    Ok(ValRef::None)
}

fn lib_while(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let cond = args.next_val()?;
    let body = args.next();
    args.done()?;

    let mut retval: ValRef = ValRef::None;
    loop {
        if !eval::call(&cond, vec![], scope)?.to_bool() {
            return Ok(retval);
        }

        match &body {
            Some(body) => {
                drop(retval);
                retval = eval::call(body, vec![], scope)?;
            }
            _ => (),
        };
    }
}

fn lib_do(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if let Some(val) = args.pop() {
        Ok(val)
    } else {
        Ok(ValRef::None)
    }
}

fn lib_bind(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let func = args.next_val()?;

    let mut map: HashMap<BString, ValRef> = HashMap::new();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;
        map.insert(key.as_ref().clone(), val);
    }

    Ok(ValRef::Binding(Rc::new(map), Rc::new(func)))
}

fn lib_with(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let mut s = Scope::new_with_parent(scope.clone());
    while args.len() > 1 {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;

        s.insert(key.as_ref().clone(), val);
    }

    let func = args.next_val()?;
    args.done()?;

    eval::call(&func, vec![], &Rc::new(RefCell::new(s)))
}

fn lib_read(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let port = args.next_val()?.get_port()?;

    let res = if args.has_next() {
        let size = args.next_val()?.get_number()?;
        args.done()?;
        port.borrow_mut().read_chunk(size as usize)
    } else {
        port.borrow_mut().read()
    };

    match res {
        Ok(val) => Ok(val),
        Err(err) => Err(StackTrace::from_string(err)),
    }
}

fn lib_write(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let port = args.next_val()?.get_port()?;
    let val = args.next_val()?;
    args.done()?;

    let res = port.borrow_mut().write(&val);
    match res {
        Ok(_) => Ok(ValRef::None),
        Err(err) => Err(StackTrace::from_string(err)),
    }
}

fn lib_seek(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let port = args.next_val()?.get_port()?;
    let num = args.next_val()?.get_number()?;
    let pos = if args.has_next() {
        match args.next_val()?.get_string()?.as_bytes() {
            b"set" => io::SeekFrom::Start(num as u64),
            b"end" => io::SeekFrom::End(num as i64),
            b"current" => io::SeekFrom::Current(num as i64),
            _ => {
                return Err(StackTrace::from_str(
                    "'seek' requires the seek offset to be 'set', 'end' or 'current'",
                ))
            }
        }
    } else {
        io::SeekFrom::Start(num as u64)
    };

    args.done()?;

    let res = port.borrow_mut().seek(pos);
    match res {
        Ok(_) => Ok(ValRef::None),
        Err(err) => Err(StackTrace::from_string(err.to_string())),
    }
}

fn lib_error(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() == 0 {
        Err(StackTrace::from_val(ValRef::None))
    } else if args.len() == 1 {
        Err(StackTrace::from_val(args[0].clone()))
    } else {
        let mut vec = Vec::new();

        for idx in 0..args.len() {
            if idx != 0 {
                vec.extend_from_slice(b" ")
            }

            match &args[idx] {
                ValRef::String(bs) => vec.extend_from_slice(bs.as_bytes()),
                arg => vec.extend_from_slice(format!("{}", arg).as_bytes()),
            }
        }

        Err(StackTrace::from_val(ValRef::String(Rc::new(
            BString::from_vec(vec),
        ))))
    }
}

fn lib_try(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let try_body = args.next_val()?;
    let catch_body = args.next_val()?;
    args.done()?;

    match eval::call(&try_body, Vec::new(), scope) {
        Ok(val) => Ok(val),
        Err(err) => eval::call(&catch_body, vec![err.message], scope),
    }
}

fn lib_lazy(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let val = args.next_val()?;
    args.done()?;

    Ok(ValRef::ProtectedLazy(Rc::new(val)))
}

fn lib_lambda(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let mut argnames: Vec<BString> = Vec::new();
    let mut block = None;
    while let Some(arg) = args.next() {
        match arg {
            ValRef::String(s) => argnames.push(s.as_ref().clone()),
            ValRef::Block(b) => {
                block = Some(b);
                break;
            }
            _ => {
                return Err(StackTrace::from_str("Expected string or block"));
            }
        }
    }

    args.done()?;
    let block = match block {
        Some(block) => block,
        None => return Err(StackTrace::from_str("Expected block")),
    };

    Ok(ValRef::Lambda(Rc::new(eval::LambdaVal {
        args: argnames,
        body: block,
    })))
}

fn lib_list(args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    Ok(ValRef::List(Rc::new(RefCell::new(args))))
}

fn lib_list_push(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    let mut lstmut = lst.borrow_mut();
    while let Some(val) = args.next() {
        lstmut.push(val);
    }

    drop(lstmut);
    Ok(ValRef::List(lst))
}

fn lib_list_pop(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    args.done()?;

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    lst.borrow_mut().pop();
    Ok(ValRef::List(lst))
}

fn lib_list_map(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let func = args.next_val()?;
    args.done()?;

    if Rc::strong_count(&lst) == 1 {
        let mut lstmut = lst.borrow_mut();
        for idx in 0..lstmut.len() {
            let val = mem::replace(&mut lstmut[idx], ValRef::None);
            let vec = vec![val, ValRef::Number(idx as f64)];
            lstmut[idx] = eval::call(&func, vec, scope)?;
        }

        drop(lstmut);
        Ok(ValRef::List(lst))
    } else {
        let lst = lst.borrow();
        let mut lstmut: Vec<ValRef> = Vec::new();
        lstmut.reserve(lst.len());
        for idx in 0..lst.len() {
            let vec = vec![lst[idx].clone(), ValRef::Number(idx as f64)];
            lstmut.push(eval::call(&func, vec, scope)?);
        }

        Ok(ValRef::List(Rc::new(RefCell::new(lstmut))))
    }
}

fn lib_list_for(mut args: Vec<ValRef>, scope: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let func = args.next_val()?;
    args.done()?;

    let mut retval = ValRef::None;
    for idx in 0..lst.borrow().len() {
        drop(retval);
        retval = eval::call(&func, vec![lst.borrow()[idx].clone()], scope)?;
    }

    Ok(retval)
}

fn lib_dict(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);

    let mut dict: HashMap<BString, ValRef> = HashMap::new();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;
        dict.insert(key.as_ref().clone(), val.clone());
    }

    Ok(ValRef::Dict(Rc::new(RefCell::new(dict))))
}

fn lib_dict_set(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    let mut args = args.drain(0..);
    let dict = args.next_val()?.get_dict()?;

    let dict = if Rc::strong_count(&dict) == 1 {
        dict
    } else {
        Rc::new((*dict).clone())
    };

    let mut dictmut = dict.borrow_mut();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;

        dictmut.insert(key.as_ref().clone(), val.clone());
    }

    drop(dictmut);
    Ok(ValRef::Dict(dict))
}

/*
@(dict-mutate d:dict key:string cb:func (arg:any)*)
    -> dict

Create a new dict with the key modified by the callback function.
It's semantically the same as this:
    (dict-set d key (cb d.key arg1 arg2 ...))
Except that it might allow for refcount==1 optimizations.
*/
fn lib_dict_mutate(mut args: Vec<ValRef>, _: &Rc<RefCell<Scope>>) -> Result<ValRef, StackTrace> {
    if args.len() < 3 {
        return Err(StackTrace::from_str("Not enough arguments"));
    }

    let mut args = args.drain(0..);
    let dict = args.next_val()?.get_dict()?;

    let dict = if Rc::strong_count(&dict) == 1 {
        dict
    } else {
        Rc::new((*dict).clone())
    };

    let mut dictmut = dict.borrow_mut();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;

        dictmut.insert(key.as_ref().clone(), val.clone());
    }

    drop(dictmut);
    Ok(ValRef::Dict(dict))
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

    s.put_func("not", Rc::new(lib_not));
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
    s.put_func("func", Rc::new(lib_func));
    s.put_func("set", Rc::new(lib_set));
    s.put_func("mutate", Rc::new(lib_mutate));

    s.put_func("if", Rc::new(lib_if));
    s.put_func("match", Rc::new(lib_match));
    s.put_func("while", Rc::new(lib_while));
    s.put_func("do", Rc::new(lib_do));

    s.put_func("bind", Rc::new(lib_bind));
    s.put_func("with", Rc::new(lib_with));
    s.put_func("read", Rc::new(lib_read));
    s.put_func("write", Rc::new(lib_write));
    s.put_func("seek", Rc::new(lib_seek));

    s.put_func("error", Rc::new(lib_error));
    s.put_func("try", Rc::new(lib_try));

    s.put_func("lambda", Rc::new(lib_lambda));

    s.put_func("lazy", Rc::new(lib_lazy));

    s.put_func("list", Rc::new(lib_list));
    s.put_func("list-push", Rc::new(lib_list_push));
    s.put_func("list-pop", Rc::new(lib_list_pop));
    s.put_func("list-map", Rc::new(lib_list_map));
    s.put_func("list-for", Rc::new(lib_list_for));

    s.put_func("dict", Rc::new(lib_dict));
    s.put_func("dict-set", Rc::new(lib_dict_set));
    s.put_func("dict-mutate", Rc::new(lib_dict_mutate));
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
    init_with_stdio(
        scope,
        StdIo {
            stdin: Rc::new(RefCell::new(ReadPort::new(Rc::new(RefCell::new(
                io::stdin(),
            ))))),
            stdout: Rc::new(RefCell::new(WritePort::new(Rc::new(RefCell::new(
                io::stdout(),
            ))))),
            stderr: Rc::new(RefCell::new(WritePort::new(Rc::new(RefCell::new(
                io::stderr(),
            ))))),
        },
    )
}
