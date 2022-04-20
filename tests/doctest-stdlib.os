; This test is auto-generated from doctest.py
; based on: src/stdlib.rs

(test-case 'not {
	(asserteq (not true) false)
	(asserteq (not false) true)
})

(test-case '+ {
	(asserteq (+ 10 20) 30)
	(asserteq (+ 33) 33)
	(asserteq (+ 1 2 3 4 5) 15)
	(asserteq (+) 0)
})

(test-case '- {
	(asserteq (- 10) -10)
	(asserteq (- 10 3) 7)
	(asserteq (- 10 2 3) 5)
	(asserteq (-) 0)
})

(test-case '* {
	(asserteq (* 10) 10)
	(asserteq (* 10 3) 30)
	(asserteq (* 10 2 3) 60)
	(asserteq (*) 0)
})

(test-case '/ {
	(asserteq (/ 10) 0.1)
	(asserteq (/ 10 2) 5)
	(asserteq (/ 30 3 2) 5)
	(asserteq (/) 0)
})

(test-case '== {
	(asserteq (== 10 10) true)
	(asserteq (== 20 10) false)
	(asserteq (== "Hello" "Hello" "Hello") true)
	(asserteq (== "Hello" "Hello" 11) false)
	(asserteq (== "11" 11) false)
	(asserteq (==) true)
})

(test-case '!= {
	(asserteq (!= 10 10) false)
	(asserteq (!= 20 10) true)
	(asserteq (!= "Hello" "Hello" "Hello") false)
	(asserteq (!= "Hello" "Hello" 11) true)
	(asserteq (!= "11" 11) true)
	(asserteq (!=) false)
})

(test-case '<= {
	(asserteq (<= 10 20 30) true)
	(asserteq (<= 10 10 10) true)
	(asserteq (<= 4 5) true)
	(asserteq (<= 50 40 30) false)
	(asserteq (<= 10 20 30 50 40) false)
	(asserteq (<= 10) true)
	(asserteq (<=) true)
})

(test-case '< {
	(asserteq (< 10 20 30) true)
	(asserteq (< 10 10 10) false)
	(asserteq (< 4 5) true)
	(asserteq (< 50 40 30) false)
	(asserteq (< 10 20 30 50 40) false)
	(asserteq (< 10) true)
	(asserteq (<) true)
})

(test-case '>= {
	(asserteq (>= 10 20 30) false)
	(asserteq (>= 10 10 10) true)
	(asserteq (>= 4 5) false)
	(asserteq (>= 50 40 30) true)
	(asserteq (>= 10 20 30 50 40) false)
	(asserteq (>= 10) true)
	(asserteq (>=) true)
})

(test-case '> {
	(asserteq (> 10 20 30) false)
	(asserteq (> 10 10 10) false)
	(asserteq (> 4 5) false)
	(asserteq (> 50 40 30) true)
	(asserteq (> 10 20 30 50 40) false)
	(asserteq (> 10) true)
	(asserteq (>) true)
})

(test-case '|| {
	(asserteq (|| "hello" false) true)
	(asserteq (|| false false) false)
	(asserteq (|| true) true)
	(asserteq (|| true false true) true)
	(asserteq (||) false)
})

(test-case '&& {
	(asserteq (&& "hello" false) false)
	(asserteq (&& false false) false)
	(asserteq (&& true) true)
	(asserteq (&& true true) true)
	(asserteq (&& true false true) false)
	(asserteq (&&) true)
})

(test-case '?? {
	(asserteq (?? none 10 20) 10)
	(asserteq (?? none) none)
	(asserteq (?? "Hello" none "Goodbye") "Hello")
	(asserteq (?? none none none 3) 3)
	(asserteq (??) none)
})

(test-case 'def {
	(asserteq (def 'x 10) none)
	(asserteq (== x 10) true)

	(asserteq (def 'x 40 'y 50) none)
	(asserteq (+ x y) 90)
})

(test-case 'func {
	(func 'square 'x {
		[x * x]
	})
	(asserteq (square 10) 100)
	(asserteq (square 5) 25)

	(func 'add 'a 'b {
		[a + b]
	})
	(asserteq (add 10 20) 30)
	(asserteq (add 9 10) 19)
})

(test-case 'set {
	(def 'x 100)
	(asserteq (== x 100) true)
	(asserteq (set 'x 50) none)
	(asserteq (== x 50) true)

	({
		(set 'x 3)
	})
	(asserteq (== x 3) true)
})

(test-case 'mutate {
	(def 'x 10)
	(asserteq (== x 10) true)
	(asserteq (mutate 'x + 5) 15)
	(asserteq (== x 15) true)
})

(test-case 'if {
	(asserteq (if [10 == 10] {"10 is 10"} {"10 is not 10"}) "10 is 10")
	(asserteq (if [20 == 10] {"20 is 10"} {"20 is not 10"}) "20 is not 10")
	(asserteq (if true {
		(def 'x 10)
		[x + 30]
	}) 40)
	(asserteq (if false {10}) none)
})

(test-case 'match {
	(def 'x 10)
	(asserteq (match
		{[x == 20] "x is 20"}
		{[x == 10] "x is 10"}
	) "x is 10")

	(asserteq (match
		{false 50}
		{true
			(def 'num 99)
			[num + 1]}
	) 100)
})

(test-case 'while {
	(def 'index 0)
	(def 'sum 1)
	(asserteq (while {[index < 4]} {
		(set 'sum [sum * 2])
		(set 'index [index + 1])
		sum
	}) 16)

	(asserteq (== sum 16) true)
	(asserteq (== index 4) true)

	(asserteq (while {false}) none)
})

(test-case 'do {
	(asserteq (do 1 2 3) 3)
	(asserteq (do (+ 1 3 5) (* 2 4) (- 9 1)) 8)
	(asserteq (do) none)

	(asserteq (do (def 'x 10) [x + 5]) 15)
})

(test-case 'bind {
})

(test-case 'with {
	(asserteq (with 'num [[100 * 3] + [10 * 2]] {
		[num + 5]
	}) 325)
})
