use super::bstring::BString;
use super::eval::{self, FuncArgs, FuncResult, PortVal, Scope, StackTrace, ValRef};
use super::parse;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::iter;
use std::mem;
use std::rc::Rc;
use std::vec;

/*
@(print (arg:any)*) -> none

Print the arguments to 'stdout', separated by a space.
*/
fn lib_print(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let args = args.drain(0..);

    let stdout = match scope.lookup(&BString::from_str("stdout")) {
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
    for arg in args {
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

    Ok((ValRef::None, scope))
}

/*
@(not val:bool) -> bool

Returns a bool value that's the inverse of its argument.

Examples:
(not true) -> false
(not false) -> true
*/
fn lib_not(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let arg = args.next_val()?;
    args.done()?;
    Ok((ValRef::Bool(!arg.to_bool()), scope))
}

/*
@(mod a:number b:number) -> number

Returns 'a' modulo 'b'.

Examples:
(mod 11 3) -> 2
[12 mod 3] -> 0
(mod 9 2) -> 1
(mod 8 2) -> 0
*/
fn lib_mod(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let a = args.next_val()?.get_number()?;
    let b = args.next_val()?.get_number()?;
    args.done()?;
    Ok((ValRef::Number(a % b), scope))
}

/*
@(+ (val:number)*) -> number

Returns all the numbers added together.

Examples:
(+ 10 20) -> 30
(+ 33) -> 33
[10 + 30] -> 40
(+ 1 2 3 4 5) -> 15
(+) -> 0
*/
fn lib_add(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Number(0.0), scope));
    }

    let mut num = args[0].to_num();
    for item in args.into_iter().skip(1) {
        num += item.to_num();
    }

    Ok((ValRef::Number(num), scope))
}

/*
@(- (val:number)*) -> number

Returns all subsequent numbers subtracted from the first number.
If there's only one argument, return the negative of that number.

Examples:
(- 10) -> -10
(- 10 3) -> 7
[10 - 4] -> 6
(- 10 2 3) -> 5
(-) -> 0
*/
fn lib_sub(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Number(0.0), scope));
    } else if args.len() == 1 {
        return Ok((ValRef::Number(-args[0].to_num()), scope));
    }

    let mut num = args[0].to_num();
    for item in args.into_iter().skip(1) {
        num -= item.to_num();
    }

    Ok((ValRef::Number(num), scope))
}

/*
@(* (val:number)*) -> number

Returns all numbers multiplied by each other.

Examples:
(* 10) -> 10
[10 * 5] -> 50
(* 10 3) -> 30
(* 10 2 3) -> 60
(*) -> 0
*/
fn lib_mul(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Number(0.0), scope));
    }

    let mut num = args[0].to_num();
    for item in args.into_iter().skip(1) {
        num *= item.to_num();
    }

    Ok((ValRef::Number(num), scope))
}

/*
@(/ (val:number)*) -> number

Returns all subsequent numbers divided from the first one.
If there's only one argument, return the reciprocal of that number.

Examples:
(/ 10) -> 0.1
(/ 10 2) -> 5
(/ 30 3 2) -> 5
[200 / 10] -> 20
(/) -> 0
*/
fn lib_div(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Number(0.0), scope));
    } else if args.len() == 1 {
        return Ok((ValRef::Number(1.0 / args[0].to_num()), scope));
    }

    let mut num = args[0].to_num();
    for item in args.into_iter().skip(1) {
        num /= item.to_num();
    }

    Ok((ValRef::Number(num), scope))
}

/*
@(== (val:any)*) -> bool

Returns true if all values are equal, false otherwise.

Examples:
(== 10 10) -> true
(== 20 10) -> false
(== "Hello" "Hello" "Hello") -> true
(== "Hello" "Hello" 11) -> false
(== "11" 11) -> false
(==) -> true

; Equality is recursive:
(== (list 1 2 3) (list 1 2 3)) -> true
(==
    (list (list (list 1) (list 2)))
    (list (list (list 1) (list 2)))) -> true
(== (list 1 2 3) (list 1 2 4)) -> false
*/
fn lib_equals(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.len() <= 1 {
        return Ok((ValRef::Bool(true), scope));
    }

    for idx in 0..args.len() - 1 {
        if !ValRef::equals(&args[idx], &args[idx + 1]) {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(!= (val:any)*) -> bool

Returns false if all values are equal, true otherwise.

Examples:
(!= 10 10) -> false
(!= 20 10) -> true
(!= "Hello" "Hello" "Hello") -> false
(!= "Hello" "Hello" 11) -> true
(!= "11" 11) -> true
(!=) -> false
*/
fn lib_nequals(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    match lib_equals(args, scope) {
        Ok((ValRef::Bool(true), s)) => Ok((ValRef::Bool(false), s)),
        Ok((ValRef::Bool(false), s)) => Ok((ValRef::Bool(true), s)),
        val => val,
    }
}

/*
@(<= (val:number)*) -> bool

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
fn lib_lteq(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Bool(true), scope));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() > args[idx + 1].to_num() {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(< (val:number)*) -> bool

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
fn lib_lt(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Bool(true), scope));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() >= args[idx + 1].to_num() {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(>= (val:number)*) -> bool

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
fn lib_gteq(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Bool(true), scope));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() < args[idx + 1].to_num() {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(> (val:number)*) -> bool

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
fn lib_gt(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::Bool(true), scope));
    }

    for idx in 0..args.len() - 1 {
        if args[idx].to_num() <= args[idx + 1].to_num() {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(|| (val:any)*) -> bool

Returns true if any argument is truthy, and false otherwise.
All values other than 'false' and 'none' are considered truthy.

Examples:
(|| "hello" false) -> true
(|| false false) -> false
(|| true) -> true
(|| true false true) -> true
(||) -> false
*/
fn lib_or(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    for item in args {
        if item.to_bool() {
            return Ok((ValRef::Bool(true), scope));
        }
    }

    Ok((ValRef::Bool(false), scope))
}

/*
@(&& (val:any)*) -> bool

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
fn lib_and(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    for item in args {
        if !item.to_bool() {
            return Ok((ValRef::Bool(false), scope));
        }
    }

    Ok((ValRef::Bool(true), scope))
}

/*
@(?? (val:any)*) -> bool

Returns the first value that's not 'none'.

Examples:
(?? none 10 20) -> 10
(?? none) -> none
(?? "Hello" none "Goodbye") -> "Hello"
(?? none none none 3) -> 3
(??) -> none
*/
fn lib_first(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    for arg in args.drain(0..) {
        match arg {
            ValRef::None => (),
            _ => return Ok((arg, scope)),
        }
    }

    Ok((ValRef::None, scope))
}

/*
@(def (name:string value:any)*) -> none

Defines the given values in the current scope.

Examples:
(def 'x 10) -> none
x -> 10

(def 'x 40 'y 50) -> none
(+ x y) -> 90
*/
fn lib_def(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;
        scope = scope.insert(key.as_ref().clone(), val);
    }

    Ok((ValRef::None, scope))
}

/*
@(func name:string (arg:string)* body:block) -> none

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
fn lib_func(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let name = args.next_val()?.get_string()?;

    let mut argnames: Vec<BString> = Vec::new();
    let mut block = None;
    for arg in args.by_ref() {
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
        scope: scope.clone(),
    }));
    scope = scope.insert(name.as_ref().clone(), val);

    Ok((ValRef::None, scope))
}

/*
@(set (name:string value:any)*) -> none

Replace the value with the given name with the given value.

Examples:
(def 'x 100)
x -> 100
(set 'x 50) -> none
x -> 50
*/
fn lib_set(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;

        if !scope.has_shallow(key.as_ref()) {
            return Err(StackTrace::from_string(format!(
                "Variable '{}' doesn't exist",
                key
            )));
        }

        scope = scope.insert(key.as_ref().clone(), val);
    }

    Ok((ValRef::None, scope))
}

/*
@(mutate name:string cb:func (arg:any)*) -> any

Replace the value with the given name with the return value of the callback function.

This:

    (mutate 'x + 1)

Is semantically the same as this:

    (set 'x (+ x 1))

Except that it might allow for refcount==1 optimizations, and that
the modified value is returned.

Examples:
(def 'x 10)
x -> 10
(mutate 'x + 5) -> 15
x -> 15
*/
fn lib_mutate(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    if args.len() < 2 {
        return Err(StackTrace::from_str("Not enough arguments"));
    }

    let name = args[0].clone().get_string()?;
    let val = match scope.lookup_shallow(name.as_ref()) {
        Some(val) => val,
        None => {
            return Err(StackTrace::from_string(format!(
                "Variable '{}' doesn't exist in this scope",
                name
            )))
        }
    };

    scope.maybe_inplace_erase(name.as_ref());

    // Replace the name and the mutator function with the value to be
    // passed as the first argument, so that we can re-use the args array
    let func = args.remove(1);
    args[0] = val;

    let new_val;
    (new_val, scope) = eval::call(&func, args, scope)?;
    scope = scope.insert(name.as_ref().clone(), new_val.clone());
    Ok((new_val, scope))
}

/*
@(if cond:bool if-body:func (else-body:func)?) -> any

Run the if-body if the condition is truthy, and the else-body
if the condition is falsy. Returns the return value of whichever
function is executed (or none if the condition is false and there's no else-body).

Examples:
(if [10 == 10] {"10 is 10"} {"10 is not 10"}) -> "10 is 10"
(if [20 == 10] {"20 is 10"} {"20 is not 10"}) -> "20 is not 10"
(if true {
    (def 'x 10)
    [x + 30]
}) -> 40
(if false {10}) -> none
*/
fn lib_if(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
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
        Ok((ValRef::None, scope))
    }
}

/*
@(match (case:block)) -> any

Each argument should be a "block", where the first expression is a condition,
and the subsequent expressions form a "body".

Examples:
(def 'x 10)
(match
    {[x == 20] "x is 20"}
    {[x == 10] "x is 10"}
) -> "x is 10"

(match
    {false 50}
    {true
        (def 'num 99)
        [num + 1]}
) -> 100
*/
fn lib_match(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    while args.has_next() {
        let block = args.next_val()?.get_block()?;

        if block.len() < 1 {
            return Err(StackTrace::from_str("Blocks must have at least 1 element"));
        }

        let val;
        (val, _) = eval::eval(&block[0], scope.clone())?;
        if val.to_bool() {
            let (res, _) = eval::eval_multiple(&block[1..], scope.clone())?;
            return Ok((res, scope));
        }
    }

    Ok((ValRef::None, scope))
}

/*
@(do (args:any)*) -> any

Returns the last argument. Used to have multiple expressions where one expression
was expected, like the comma operator in C-like languages.

Examples:
(do 1 2 3) -> 3
(do (+ 1 3 5) (* 2 4) (- 9 1)) -> 8
(do) -> none

; Expressions may have side-effects, which is generally when you'd need 'do'
(do (def 'x 10) [x + 5]) -> 15
*/
fn lib_do(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if let Some(val) = args.pop() {
        Ok((val, scope))
    } else {
        Ok((ValRef::None, scope))
    }
}

/*
@(read port:port size:number?) -> any

Read from a port.
*/
fn lib_read(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
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
        Ok(val) => Ok((val, scope)),
        Err(err) => Err(StackTrace::from_string(err)),
    }
}

/*
@(write port:port value:any) -> none

Write to a port.
*/
fn lib_write(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let port = args.next_val()?.get_port()?;
    let val = args.next_val()?;
    args.done()?;

    let res = port.borrow_mut().write(&val);
    match res {
        Ok(_) => Ok((ValRef::None, scope)),
        Err(err) => Err(StackTrace::from_string(err)),
    }
}

/*
@(seek port:port offset:number from:string?) -> none

Seek a port. 'from' can be:
* set: Seek from the beginning (default)
* end: Seek from the end
* current: Seek from the current position
*/
fn lib_seek(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
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
        Ok(_) => Ok((ValRef::None, scope)),
        Err(err) => Err(StackTrace::from_string(err)),
    }
}

/*
@(error (message:any)*) -> error

Create an error. An error contains a value:
* If 'error' is called with no arguments, the value is 'none'.
* If 'error' is called with one argument, the value is that argument.
* If 'error' is called with multiple arguments, they are concatenated together
  and the value is the resulting string.
*/
fn lib_error(args: Vec<ValRef>, _: Scope) -> FuncResult {
    if args.is_empty() {
        Err(StackTrace::from_val(ValRef::None))
    } else if args.len() == 1 {
        Err(StackTrace::from_val(args[0].clone()))
    } else {
        let mut vec = Vec::new();

        for (idx, item) in args.iter().enumerate() {
            if idx != 0 {
                vec.extend_from_slice(b" ")
            }

            match &item {
                ValRef::String(bs) => vec.extend_from_slice(bs.as_bytes()),
                arg => vec.extend_from_slice(format!("{}", arg).as_bytes()),
            }
        }

        Err(StackTrace::from_val(ValRef::String(Rc::new(
            BString::from_vec(vec),
        ))))
    }
}

/*
@(try body:func catch:func) -> any

Call 'body'. If it returns an error, call 'catch' with that error's value as an argument.

Examples:
(try {
    (error "Oh no")
} (lambda 'err {
    ; somehow handle the error
    "An error occurred"
})) -> "An error occurred"
*/
fn lib_try(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let try_body = args.next_val()?;
    let catch_body = args.next_val()?;
    args.done()?;

    match eval::call(&try_body, Vec::new(), scope.subscope()) {
        Ok(res) => Ok(res),
        Err(err) => eval::call(&catch_body, vec![err.message], scope),
    }
}

/*
@(bool value:any) -> bool

Convert the argument to a bool.

Examples:
(bool true) -> true
(bool false) -> false
(bool none) -> false
(bool "hello") -> true
*/
fn lib_bool(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);
    let arg = args.next_val()?;
    args.done()?;

    match arg {
        ValRef::Bool(..) => Ok((arg, scope)),
        _ => Ok((ValRef::Bool(arg.to_bool()), scope)),
    }
}

/*
@(number value:any) -> number

Convert the argument to a number.

Examples:
(number 10) -> 10
(number false) -> 0
(number true) -> 1
(number "20") -> 20
*/
fn lib_number(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);
    let arg = args.next_val()?;
    args.done()?;

    match arg {
        ValRef::Number(..) => Ok((arg, scope)),
        ValRef::String(s) => {
            let filename = BString::from_str("string");
            let mut r = parse::Reader::new(s.as_bytes(), filename);
            match parse::read_number(&mut r) {
                Ok(num) => Ok((ValRef::Number(num), scope)),
                Err(err) => Err(StackTrace::from_string(err.msg)),
            }
        }
        _ => Ok((ValRef::Number(arg.to_num()), scope)),
    }
}

/*
@(string (value:any)*) -> string

Create a string from a value. If there are multiple values,
they will be converted to strings and concatenated together.

Examples:
(string) -> ""
(string "Hello") -> "Hello"
(string 10) -> "10"
(string "There are " 10 " trees") -> "There are 10 trees"
(string [3 + 5] " things") -> "8 things"
*/
fn lib_string(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.is_empty() {
        return Ok((ValRef::String(Rc::new(BString::from_str(""))), scope));
    }

    if args.len() == 1 && matches!(args[0], ValRef::String(..)) {
        return Ok((args.pop().unwrap(), scope));
    }

    let args = args.drain(0..);
    let mut buf: Vec<u8> = Vec::new();
    for arg in args {
        if let ValRef::String(s) = arg {
            buf.extend_from_slice(s.as_ref().as_bytes());
        } else {
            buf.extend_from_slice(arg.to_bstring().as_bytes());
        }
    }

    Ok((ValRef::String(Rc::new(BString::from_vec(buf))), scope))
}

/*
@(lazy f:func) -> lazy

Create a lazy variable.
A lazy variable contains a reference to a function,
and whenever the variable is used, that function
is implicitly called and the variable evaluates to
the function's return value.

Examples:
(def 'make-ten {10})
(def 'ten (lazy make-ten))
ten -> 10
*/
fn lib_lazy(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let val = args.next_val()?;
    args.done()?;

    Ok((ValRef::ProtectedLazy(Rc::new(val)), scope))
}

/*
@(lambda (param:string)* body:block) -> lambda

Create a lambda, which is like a block, but which creates
its own scope when called and which has named arguments.

Examples:
(def 'add (lambda 'x 'y {
    [x + y]
}))
(add 10 20) -> 30
(add 5 7) -> 12
[9 add 10] -> 19
*/
fn lib_lambda(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let mut argnames: Vec<BString> = Vec::new();
    let mut block = None;
    for arg in args.by_ref() {
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

    Ok((
        ValRef::Lambda(Rc::new(eval::LambdaVal {
            args: argnames,
            body: block.clone(),
            scope: scope.clone(),
        })),
        scope,
    ))
}

/*
@(list (value:any)*) -> list

Create a list.

A list can be called with a numeric index as its argument.
The list then returns the value at that index, or 'none'.

Examples:
((list) 0) -> none

(def 'l (list 10 20))
(l 0) -> 10
(l 1) -> 20
(l 2) -> none

; This is an alternate function call syntax
l.0 -> 10
l.1 -> 20
l.[0 + 1] -> 20
l.(+ 0 1) -> 20
*/
fn lib_list(args: Vec<ValRef>, scope: Scope) -> FuncResult {
    Ok((ValRef::List(Rc::new(RefCell::new(args))), scope))
}

/*
@(list-push l:list (value:any*)) -> list

Returns a new list with new values appended.

Examples:
(def 'l (list 10))
l -> (list 10)
(list-push l 20) -> (list 10 20)
(mutate 'l list-push 30 40)
l -> (list 10 30 40)
*/
fn lib_list_push(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    let mut lstmut = lst.borrow_mut();
    for val in args {
        lstmut.push(val);
    }

    drop(lstmut);
    Ok((ValRef::List(lst), scope))
}

/*
@(list-pop l:list) -> list

Returns a new list with the last value removed.

Examples:
(def 'l (list 10 20))
l -> (list 10 20)
(mutate 'l list-pop)
l -> (list 10)
*/
fn lib_list_pop(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    args.done()?;

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    lst.borrow_mut().pop();
    Ok((ValRef::List(lst), scope))
}

/*
@(list-insert l:list idx:number (value:any)*) -> list

Returns a new list with some items inserted into the list at the given index.
'l.[idx]' becomes the first 'value'.

Examples:
(def 'l (list 1 2 3))
l -> (list 1 2 3)
(mutate 'l list-insert 0 10)
l -> (list 10 1 2 3)
(mutate 'l list-insert 2 99 100)
l -> (list 10 1 99 100 2 3)
*/
fn lib_list_insert(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let idx = args.next_val()?.get_number()? as usize;

    if idx >= lst.borrow().len() {
        return Err(StackTrace::from_str("Index out of bounds"));
    }

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    lst.borrow_mut().splice(idx..idx, args);
    Ok((ValRef::List(lst), scope))
}

/*
@(list-remove l:list idx:number end:number?) -> list

Returns a new list with some values removed.
If an 'end' argument is provided, all values from 'idx' (inclusive)
to 'end' (exclusive) are removed.
If no 'end' argument is provided, only 'idx' is removed.

Examples:
(def 'l (list 1 2 3))
l -> (list 1 2 3)
(mutate 'l list-remove 1)
l -> (list 1 3)

(def 'l (list 1 2 3 4))
l -> (list 1 2 3 4)
(mutate 'l list-remove 1 3)
l -> (list 1 4)
*/
fn lib_list_remove(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let idx = args.next_val()?.get_number()? as usize;
    let end = match args.next() {
        None => idx + 1,
        Some(x) => x.get_number()? as usize,
    };

    if idx >= lst.borrow().len() {
        return Err(StackTrace::from_str("Index out of bounds"));
    }

    let lst = if Rc::strong_count(&lst) == 1 {
        lst
    } else {
        Rc::new((*lst).clone())
    };

    lst.borrow_mut().splice(idx..end, iter::empty());
    Ok((ValRef::List(lst), scope))
}

/*
@(list-map l:list transform:func) -> list

Returns a new list where every value is transformed by the transform function.

Examples:
(def 'l (list 1 2 3))
l -> (list 1 2 3)
(mutate 'l list-map (lambda 'x {[x * 10]}))
l -> (list 10 20 30)
*/
fn lib_list_map(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let func = args.next_val()?;
    args.done()?;

    if Rc::strong_count(&lst) == 1 {
        let mut lstmut = lst.borrow_mut();
        for idx in 0..lstmut.len() {
            let val = mem::replace(&mut lstmut[idx], ValRef::None);
            let vec = vec![val, ValRef::Number(idx as f64)];
            let res;
            (res, scope) = eval::call(&func, vec, scope)?;
            lstmut[idx] = res;
        }

        drop(lstmut);
        Ok((ValRef::List(lst), scope))
    } else {
        let lst = lst.borrow();
        let mut lstmut: Vec<ValRef> = Vec::new();
        lstmut.reserve(lst.len());
        for idx in 0..lst.len() {
            let vec = vec![lst[idx].clone(), ValRef::Number(idx as f64)];
            let res;
            (res, scope) = eval::call(&func, vec, scope)?;
            lstmut.push(res);
        }

        Ok((ValRef::List(Rc::new(RefCell::new(lstmut))), scope))
    }
}

/*
@(list-last l:list) -> any

Returns the last vaule of a list, or 'none'.

Examples:
(list-last (list 10 20)) -> 20
(list-last (list)) -> none
*/
fn lib_list_last(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    args.done()?;

    let lst = lst.borrow();
    match lst.last() {
        Some(v) => Ok((v.clone(), scope)),
        None => Ok((ValRef::None, scope)),
    }
}

/*
@(list-for l:list f:func) -> any

Call the function with every element of the list.
The return value is whatever the last function call returned.

Examples:
(def 'l (list 1 2 3 99))
(list-for l (lambda 'el {
    el
})) -> 99
*/
fn lib_list_for(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let func = args.next_val()?;
    args.done()?;

    let mut retval = ValRef::None;
    for idx in 0..lst.borrow().len() {
        drop(retval);
        (retval, scope) = eval::call(&func, vec![lst.borrow()[idx].clone()], scope)?;
    }

    Ok((retval, scope))
}

/*
@(list-reduce l:list acc:any f:func) -> any

Reduce over the list, calling the function with every value and the accumulator.

Examples:
(def 'l (list 1 2 3 10))
(list-reduce l 0 (lambda 'el 'sum {
    [sum + el]
})) -> 16
(list-reduce (list 10 20) 1 *) -> 200
*/
fn lib_list_reduce(mut args: Vec<ValRef>, mut scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let lst = args.next_val()?.get_list()?;
    let mut acc = args.next_val()?;
    let func = args.next_val()?;
    args.done()?;

    for idx in 0..lst.borrow().len() {
        (acc, scope) = eval::call(&func, vec![lst.borrow()[idx].clone(), acc], scope)?;
    }

    Ok((acc, scope))
}

/*
@(list-len l:list) -> number

Get the length of a list.

Examples:
(list-len (list)) -> 0
(list-len (list 1 2 3)) -> 3
*/
fn lib_list_len(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);
    let lst = args.next_val()?.get_list()?;
    args.done()?;
    let lst = lst.borrow();
    Ok((ValRef::Number(lst.len() as f64), scope))
}

/*
@(dict (key:string value:any)*) -> dict

Create a dict.

A dict can be called with a string key as its argument.
The list then returns the value at that key, or 'none'.

Examples:
((dict) 'x) -> none

(def 'd (dict
    'x 10
    'y 20))
(d 'x) -> 10
(d 'y) -> 20
(d 'z) -> none

; This is an alternate function call syntax
d.x -> 10
d.y -> 20
*/
fn lib_dict(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    let mut args = args.drain(0..);

    let mut dict: HashMap<BString, ValRef> = HashMap::new();
    while args.has_next() {
        let key = args.next_val()?.get_string()?;
        let val = args.next_val()?;
        dict.insert(key.as_ref().clone(), val.clone());
    }

    Ok((ValRef::Dict(Rc::new(RefCell::new(dict))), scope))
}

/*
@(dict-set (key:string value:any)*) -> dict

Returns a new dict with the new keys and values.

Examples:
(def 'd (dict 'x 10 'y 20))
d -> (dict 'x 10 'y 20)
(mutate 'd dict-set 'x 30)
d -> (dict 'x 30 'y 20)
*/
fn lib_dict_set(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
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
    Ok((ValRef::Dict(dict), scope))
}

/*
@(dict-mutate d:dict key:string cb:func (arg:any)*) -> dict

Returns a new dict with the key modified by the callback function.

This:

    (dict-mutate d 'x + 1)

Is semantically the same as this:

    (dict-set d 'x (+ d.x 1))

Except that it might allow for refcount==1 optimizations.

Examples:
(func 'add-one 'x {
    [x + 1]
})
(def 'd (dict 'x 10 'y 20))
d.x -> 10
((dict-mutate d 'x add-one) 'x) -> 11
((dict-mutate d 'x + 1) 'x) -> 11

; We can use it together with 'mutate'
(mutate 'd dict-mutate 'x - 3)
d.x -> 7
*/
fn lib_dict_mutate(mut args: Vec<ValRef>, scope: Scope) -> FuncResult {
    if args.len() < 3 {
        return Err(StackTrace::from_str("Not enough arguments"));
    }

    let mut it = args.drain(0..2);
    let dict = it.next_val()?.get_dict()?;
    let name = it.next_val()?.get_string()?;
    drop(it);

    let dict = if Rc::strong_count(&dict) == 1 {
        dict
    } else {
        Rc::new((*dict).clone())
    };

    let val = match dict.borrow_mut().remove(name.as_ref()) {
        Some(val) => val,
        None => {
            return Err(StackTrace::from_string(format!(
                "Variable '{}' doesn't exist",
                name
            )))
        }
    };

    let func = mem::replace(&mut args[0], val);

    let (res, scope) = eval::call(&func, args, scope)?;
    dict.borrow_mut().insert(name.as_ref().clone(), res);

    Ok((ValRef::Dict(dict), scope))
}

pub struct StdIo {
    pub stdin: Rc<RefCell<dyn PortVal>>,
    pub stdout: Rc<RefCell<dyn PortVal>>,
    pub stderr: Rc<RefCell<dyn PortVal>>,
}

pub fn init_with_stdio(mut s: Scope, stdio: StdIo) -> Scope {
    s = s.put("stdin", ValRef::Port(stdio.stdin));
    s = s.put("stdout", ValRef::Port(stdio.stdout));
    s = s.put("stderr", ValRef::Port(stdio.stderr));

    s = s.put("none", ValRef::None);
    s = s.put("false", ValRef::Bool(false));
    s = s.put("true", ValRef::Bool(true));

    s = s.put_func("print", Rc::new(lib_print));

    s = s.put_func("not", Rc::new(lib_not));
    s = s.put_func("mod", Rc::new(lib_mod));
    s = s.put_func("+", Rc::new(lib_add));
    s = s.put_func("-", Rc::new(lib_sub));
    s = s.put_func("*", Rc::new(lib_mul));
    s = s.put_func("/", Rc::new(lib_div));
    s = s.put_func("==", Rc::new(lib_equals));
    s = s.put_func("!=", Rc::new(lib_nequals));
    s = s.put_func("<=", Rc::new(lib_lteq));
    s = s.put_func("<", Rc::new(lib_lt));
    s = s.put_func(">=", Rc::new(lib_gteq));
    s = s.put_func(">", Rc::new(lib_gt));
    s = s.put_func("||", Rc::new(lib_or));
    s = s.put_func("&&", Rc::new(lib_and));
    s = s.put_func("??", Rc::new(lib_first));

    s = s.put_func("def", Rc::new(lib_def));
    s = s.put_func("func", Rc::new(lib_func));
    s = s.put_func("set", Rc::new(lib_set));
    s = s.put_func("mutate", Rc::new(lib_mutate));

    s = s.put_func("if", Rc::new(lib_if));
    s = s.put_func("match", Rc::new(lib_match));
    s = s.put_func("do", Rc::new(lib_do));

    s = s.put_func("read", Rc::new(lib_read));
    s = s.put_func("write", Rc::new(lib_write));
    s = s.put_func("seek", Rc::new(lib_seek));

    s = s.put_func("error", Rc::new(lib_error));
    s = s.put_func("try", Rc::new(lib_try));

    s = s.put_func("number", Rc::new(lib_number));
    s = s.put_func("bool", Rc::new(lib_bool));
    s = s.put_func("string", Rc::new(lib_string));

    s = s.put_func("lambda", Rc::new(lib_lambda));

    s = s.put_func("lazy", Rc::new(lib_lazy));

    s = s.put_func("list", Rc::new(lib_list));
    s = s.put_func("list-push", Rc::new(lib_list_push));
    s = s.put_func("list-pop", Rc::new(lib_list_pop));
    s = s.put_func("list-insert", Rc::new(lib_list_insert));
    s = s.put_func("list-remove", Rc::new(lib_list_remove));
    s = s.put_func("list-map", Rc::new(lib_list_map));
    s = s.put_func("list-last", Rc::new(lib_list_last));
    s = s.put_func("list-for", Rc::new(lib_list_for));
    s = s.put_func("list-reduce", Rc::new(lib_list_reduce));
    s = s.put_func("list-len", Rc::new(lib_list_len));

    s = s.put_func("dict", Rc::new(lib_dict));
    s = s.put_func("dict-set", Rc::new(lib_dict_set));
    s = s.put_func("dict-mutate", Rc::new(lib_dict_mutate));

    s
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

pub fn init(scope: Scope) -> Scope {
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
