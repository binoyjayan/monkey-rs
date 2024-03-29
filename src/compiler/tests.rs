#![allow(unused_imports)]
use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::code::definitions;
use crate::code::opcode::*;
use crate::common::environment::*;
use crate::common::error::*;
use crate::common::object::*;
use crate::evaluator::Evaluator;
use crate::parser::*;
use crate::scanner::*;

#[cfg(test)]
struct CompilerTestCase {
    input: &'static str,
    expected_constants: Vec<Object>,
    expected_instructions: Vec<Instructions>,
}

#[cfg(test)]
fn parse_program(input: &str) -> Program {
    let scanner = Scanner::new(input);
    let mut parser = Parser::new(scanner);
    let program = parser.parse_program();
    if parser.print_errors() {
        panic!("{} parse errors", parser.parse_errors().len());
    }
    program
}

#[cfg(test)]
pub fn test_constants(expected: &Vec<Object>, actual: &Vec<Rc<Object>>) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "Wrong nmber of constants. got={}, want={}",
        actual.len(),
        expected.len()
    );
    for (exp, got) in expected.iter().zip(actual) {
        match exp {
            Object::Bool(e) => test_boolean_object(got.clone(), e.clone()),
            Object::Number(e) => test_numeric_object(got.clone(), e.clone()),
            Object::Str(s) => test_string_object(got, &s.clone()),
            Object::CompiledFunc(func) => test_function_object(&got.clone(), &func),
            _ => {}
        }
    }
}

#[cfg(test)]
fn test_boolean_object(actual: Rc<Object>, exp: bool) {
    if let Object::Bool(act) = *actual.clone() {
        assert_eq!(
            act, exp,
            "object has wrong value. got={}, want={}",
            act, exp
        );
    } else {
        panic!("object is not boolean. got={}", actual);
    }
}

#[cfg(test)]
fn test_numeric_object(actual: Rc<Object>, exp: f64) {
    if let Object::Number(act) = *actual.clone() {
        assert_eq!(
            act, exp,
            "object has wrong value. got={}, want={}",
            act, exp
        );
    } else {
        panic!("object is not numeric. got={}", actual);
    }
}

#[cfg(test)]
fn test_string_object(actual: &Object, expected: &str) {
    if let Object::Str(result) = actual {
        assert_eq!(
            result, expected,
            "object has wrong value. got={}, want={}",
            result, expected
        );
    } else {
        panic!("object is not Str. got={:?}", actual);
    }
}

#[cfg(test)]
fn test_function_object(actual_obj: &Object, expected: &CompiledFunction) {
    if let Object::CompiledFunc(actual) = actual_obj {
        test_instructions(
            &vec![(&*expected.instructions).clone()],
            &actual.instructions,
        );
    } else {
        panic!("object is not a compiled function. got={:?}", actual_obj);
    }
}

/*
 * concat_instructions is needed because the expected_instructions field in
 * CompilerTestCase is not just a slice of bytes, but a slice of slices of
 * bytes. And that’s because 'make' is used to generate the expected_instructions,
 * which produces a [u8]. So in order to compare the expected_instructions with
 * the actual instructions, we need to turn the slice of slices into a flattened
 * slice by concatenating the instructions.
 */
#[cfg(test)]
fn concat_instructions(s: &[Instructions]) -> Instructions {
    let mut out = Instructions::default();
    for ins in s {
        out.code.extend_from_slice(&ins.code);
        out.lines.extend_from_slice(&ins.lines);
    }
    out
}

#[cfg(test)]
fn test_instructions(expected: &[Instructions], actual: &Instructions) {
    let concatted = concat_instructions(expected);

    assert_eq!(
        concatted.len(),
        actual.len(),
        "Wrong number of instructions. want={}, got={}",
        concatted,
        actual,
    );
    for i in 0..concatted.len() {
        if actual.get(i) != concatted.get(i) {
            panic!(
                "wrong instruction at index {}.\nwant={}\ngot ={}",
                i, concatted, actual
            );
        }
    }
}

#[cfg(test)]
fn run_compiler_tests(tests: &[CompilerTestCase]) {
    for (n, t) in tests.iter().enumerate() {
        let program = parse_program(&t.input);
        let mut compiler = Compiler::new();
        let result = compiler.compile(program);
        if let Err(err) = result {
            panic!("[{}] {}", n, err);
        }
        // println!("[{}] Compiler Test", n);
        let bytecode = compiler.bytecode();
        test_instructions(&t.expected_instructions, &bytecode.instructions);
        test_constants(&t.expected_constants, &bytecode.constants);
    }
}

#[test]
fn test_integer_arithmetic() {
    let tests = vec![
        CompilerTestCase {
            input: "1 + 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Add, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 - 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Sub, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 * 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Mul, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "2 / 1",
            expected_constants: vec![Object::Number(2.), Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Div, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1; 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "-1",
            expected_constants: vec![Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Minus, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_boolean_expressions() {
    let tests = vec![
        CompilerTestCase {
            input: "true",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::True, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "false",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::False, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 > 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Greater, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 < 2",
            // Constants are in reverse order: '1 < 2' is '2 > 1'
            expected_constants: vec![Object::Number(2.), Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Greater, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 == 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Equal, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "1 != 2",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::NotEqual, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "true == false",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::True, &[], 1),
                definitions::make(Opcode::False, &[], 1),
                definitions::make(Opcode::Equal, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "true != false",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::True, &[], 1),
                definitions::make(Opcode::False, &[], 1),
                definitions::make(Opcode::NotEqual, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "!false",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::False, &[], 1),
                definitions::make(Opcode::Bang, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_conditional() {
    let tests = vec![
        CompilerTestCase {
            input: "if (true) { 10 }; 3333;",
            expected_constants: vec![Object::Number(10.), Object::Number(3333.)],
            expected_instructions: vec![
                // 0000 : The condition
                definitions::make(Opcode::True, &[], 1),
                // 0001 : Jump to the 'Nil' instruction following 'then_stmt'
                definitions::make(Opcode::JumpIfFalse, &[10], 1),
                // 0004 : The 'then_stmt'
                definitions::make(Opcode::Constant, &[0], 1),
                // 0007 : To Jump over the 'else_stmt' which is 'Nil' here.
                definitions::make(Opcode::Jump, &[11], 1),
                // 0010
                definitions::make(Opcode::Nil, &[], 1),
                // 0011 : [ Not part of the if expr - Pop its result ]
                definitions::make(Opcode::Pop, &[], 1),
                // 0012 : The instruction following the if expr
                definitions::make(Opcode::Constant, &[1], 1),
                // 0015
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "if (true) { 10 } else { 20 } ; 3333;",
            expected_constants: vec![
                Object::Number(10.),
                Object::Number(20.),
                Object::Number(3333.),
            ],
            expected_instructions: vec![
                // 0000 : The condition
                definitions::make(Opcode::True, &[], 1),
                // 0001: Jump to 'else_stmt' if condition is false
                definitions::make(Opcode::JumpIfFalse, &[10], 1),
                // 0004 : The 'then_stmt'
                definitions::make(Opcode::Constant, &[0], 1),
                // 0007 : Jump to the instruction following the 'if' expression
                definitions::make(Opcode::Jump, &[13], 1),
                // 0010 : The 'else_stmt'
                definitions::make(Opcode::Constant, &[1], 1),
                // 0013 : [ Not part of the if expr - Pop its result ]
                definitions::make(Opcode::Pop, &[], 1),
                // 0014 : The instruction following the if expr
                definitions::make(Opcode::Constant, &[2], 1),
                // 0017
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_global_let_statements() {
    let tests = vec![
        CompilerTestCase {
            input: "let one = 1;let two = 2;",
            expected_constants: vec![Object::Number(1.), Object::Number(2.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::SetGlobal, &[1], 1),
            ],
        },
        CompilerTestCase {
            input: "let one = 1;one;",
            expected_constants: vec![Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "let one = 1;let two = one;two;",
            expected_constants: vec![Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::SetGlobal, &[1], 1),
                definitions::make(Opcode::GetGlobal, &[1], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_string_expressions() {
    let tests = vec![
        CompilerTestCase {
            input: r#""monkey""#,
            expected_constants: vec![Object::Str(String::from("monkey"))],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: r#""mon" + "key""#,
            expected_constants: vec![
                Object::Str(String::from("mon")),
                Object::Str(String::from("key")),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Add, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_array_literals() {
    // An array literal expression involves building 'n' elements on the stack
    // followed by an OpArray instruction
    let tests = vec![
        CompilerTestCase {
            input: "[]",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::Array, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "[1, 2, 3]",
            expected_constants: vec![Object::Number(1.), Object::Number(2.), Object::Number(3.)],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Array, &[3], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "[1 + 2, 3 - 4, 5 * 6]",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::Number(3.),
                Object::Number(4.),
                Object::Number(5.),
                Object::Number(6.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Add, &[], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Sub, &[], 1),
                definitions::make(Opcode::Constant, &[4], 1),
                definitions::make(Opcode::Constant, &[5], 1),
                definitions::make(Opcode::Mul, &[], 1),
                definitions::make(Opcode::Array, &[3], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_hash_literals() {
    let tests = vec![
        CompilerTestCase {
            input: "{}",
            expected_constants: vec![],
            expected_instructions: vec![
                definitions::make(Opcode::Map, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "{1: 2, 3: 4, 5: 6}",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::Number(3.),
                Object::Number(4.),
                Object::Number(5.),
                Object::Number(6.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Constant, &[4], 1),
                definitions::make(Opcode::Constant, &[5], 1),
                definitions::make(Opcode::Map, &[6], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "{1: 2 + 3, 4: 5 * 6}",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::Number(3.),
                Object::Number(4.),
                Object::Number(5.),
                Object::Number(6.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Add, &[], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Constant, &[4], 1),
                definitions::make(Opcode::Constant, &[5], 1),
                definitions::make(Opcode::Mul, &[], 1),
                definitions::make(Opcode::Map, &[4], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];
    run_compiler_tests(&tests);
}

#[test]
fn test_index_expressions() {
    let tests = vec![
        CompilerTestCase {
            input: "[1, 2, 3][1 + 1]",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::Number(3.),
                Object::Number(1.),
                Object::Number(1.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Array, &[3], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Constant, &[4], 1),
                definitions::make(Opcode::Add, &[], 1),
                definitions::make(Opcode::Index, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "{1: 2}[2 - 1]",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::Number(2.),
                Object::Number(1.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Constant, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Map, &[2], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Sub, &[], 1),
                definitions::make(Opcode::Index, &[], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_functions() {
    let tests = vec![
        CompilerTestCase {
            input: "fn() { return 5 + 10 }",
            expected_constants: vec![
                Object::Number(5.),
                Object::Number(10.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::Constant, &[1], 1),
                        definitions::make(Opcode::Add, &[], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // Number of free variables is '0'
                definitions::make(Opcode::Closure, &[2, 0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "fn() { 5 + 10 }",
            expected_constants: vec![
                Object::Number(5.),
                Object::Number(10.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::Constant, &[1], 1),
                        definitions::make(Opcode::Add, &[], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // Number of free variables is '0'
                definitions::make(Opcode::Closure, &[2, 0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "fn() { 1; 2 }",
            expected_constants: vec![
                Object::Number(1.),
                Object::Number(2.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::Constant, &[0], 1),
                        // Pop the first value
                        definitions::make(Opcode::Pop, &[], 1),
                        definitions::make(Opcode::Constant, &[1], 1),
                        // The pop is replaced by the implicit return
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[2, 0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];
    run_compiler_tests(&tests);
}

#[test]
fn test_functions_without_return_value() {
    let tests = vec![CompilerTestCase {
        input: "fn() { }",
        expected_constants: vec![Object::CompiledFunc(Rc::new(CompiledFunction::new(
            definitions::make(Opcode::Return, &[], 1),
            0,
            0,
        )))],
        expected_instructions: vec![
            definitions::make(Opcode::Closure, &[0, 0], 1),
            definitions::make(Opcode::Pop, &[], 1),
        ],
    }];
    run_compiler_tests(&tests);
}

#[test]
fn test_compiler_scopes() {
    let mut compiler = Compiler::new();
    assert_eq!(compiler.scope_index, 0);

    let global_symbol_table = compiler.symtab.clone();
    compiler.emit(Opcode::Mul, &[0], 1);

    compiler.enter_scope();
    assert_eq!(compiler.scope_index, 1);

    compiler.emit(Opcode::Sub, &[0], 1);
    assert_eq!(compiler.scopes[compiler.scope_index].instructions.len(), 1);
    let last = &compiler.scopes[compiler.scope_index].last_ins;
    assert_eq!(last.opcode, Opcode::Sub);

    if let Some(outer) = compiler.symtab.outer.clone() {
        assert_eq!(outer.as_ref().clone(), global_symbol_table);
    } else {
        panic!("compiler did not enclose symbol table");
    }

    compiler.leave_scope();
    assert_eq!(compiler.scope_index, 0, "scope index wrong");

    assert_eq!(
        compiler.symtab, global_symbol_table,
        "compiler did not restore symbol table"
    );
    assert!(
        compiler.symtab.outer.is_none(),
        "compiler modified global symbol table incorrectly"
    );

    compiler.emit(Opcode::Add, &[0], 1);

    assert_eq!(compiler.scopes[compiler.scope_index].instructions.len(), 2);
    let last = &compiler.scopes[compiler.scope_index].last_ins;
    assert_eq!(last.opcode, Opcode::Add);

    let previous = &compiler.scopes[compiler.scope_index].prev_ins;
    assert_eq!(previous.opcode, Opcode::Mul);
}

#[test]
fn test_function_calls() {
    let tests = vec![
        CompilerTestCase {
            input: "fn() { 24 }()",
            expected_constants: vec![
                Object::Number(24.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // The literal '24'
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // The compiled function (closure)
                definitions::make(Opcode::Closure, &[1, 0], 1),
                definitions::make(Opcode::Call, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            // Function is bound to a name here
            input: "let noArg = fn() { 24 }; noArg();",
            expected_constants: vec![
                Object::Number(24.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // The literal '24'
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // The compiled function
                definitions::make(Opcode::Closure, &[1, 0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Call, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "
                let oneArg = fn(a) { a };
                oneArg(24);
            ",
            expected_constants: vec![
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    1,
                ))),
                Object::Number(24.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[0, 0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Call, &[1], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "
                let manyArg = fn(a, b, c) { a; b; c; };
                manyArg(24, 25, 26);
            ",
            expected_constants: vec![
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // References to the arguments to the function
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::Pop, &[], 1),
                        definitions::make(Opcode::GetLocal, &[1], 1),
                        definitions::make(Opcode::Pop, &[], 1),
                        definitions::make(Opcode::GetLocal, &[2], 1),
                        // returning the last reference
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    3,
                ))),
                Object::Number(24.),
                Object::Number(25.),
                Object::Number(26.),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[0, 0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Constant, &[1], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Constant, &[3], 1),
                definitions::make(Opcode::Call, &[3], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];
    run_compiler_tests(&tests);
}

#[test]
fn test_let_statement_scopes() {
    let tests = vec![
        CompilerTestCase {
            input: "let num = 55; fn() { num }",
            expected_constants: vec![
                Object::Number(55.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // push the value of global variable 'num'
                        definitions::make(Opcode::GetGlobal, &[0], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    0,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // constant - number 55
                definitions::make(Opcode::Constant, &[0], 1),
                // set the global variable 'num'
                definitions::make(Opcode::SetGlobal, &[0], 1),
                // constant - compiled function (closure)
                definitions::make(Opcode::Closure, &[1, 0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "fn() { let num = 55; num }",
            expected_constants: vec![
                Object::Number(55.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // constant - number 55
                        definitions::make(Opcode::Constant, &[0], 1),
                        // set the global variable 'num'
                        definitions::make(Opcode::SetLocal, &[0], 1),
                        // push the value of global variable 'num'
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    1,
                    0,
                ))),
            ],
            expected_instructions: vec![
                // constant - compiled function (closure)
                definitions::make(Opcode::Closure, &[1, 0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "
                fn() {
                    let a = 55;
                    let b = 77;
                    a + b
                }
            ",
            expected_constants: vec![
                Object::Number(55.),
                Object::Number(77.),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::Constant, &[0], 1), // 55
                        definitions::make(Opcode::SetLocal, &[0], 1), // 'a'
                        definitions::make(Opcode::Constant, &[1], 1), // 77
                        definitions::make(Opcode::SetLocal, &[1], 1), // 'b'
                        definitions::make(Opcode::GetLocal, &[0], 1), // 'a'
                        definitions::make(Opcode::GetLocal, &[1], 1), // 'b'
                        definitions::make(Opcode::Add, &[], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    2,
                    0,
                ))),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[2, 0], 1), // compiled fn (closure)
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];
    run_compiler_tests(&tests);
}

#[test]
fn test_builtins() {
    let tests = vec![
        CompilerTestCase {
            input: r#"
                len([]);
                push([], 1);
            "#,
            expected_constants: vec![Object::Number(1.)],
            expected_instructions: vec![
                definitions::make(Opcode::GetBuiltin, &[0], 1),
                definitions::make(Opcode::Array, &[0], 1),
                // call built-in fn 'len' with one argument
                definitions::make(Opcode::Call, &[1], 1),
                definitions::make(Opcode::Pop, &[], 1),
                definitions::make(Opcode::GetBuiltin, &[5], 1),
                definitions::make(Opcode::Array, &[0], 1),
                definitions::make(Opcode::Constant, &[0], 1),
                // call built-in fn 'push' with two arguments
                definitions::make(Opcode::Call, &[2], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: "fn() { len([]) }",
            expected_constants: vec![Object::CompiledFunc(Rc::new(CompiledFunction::new(
                concat_instructions(&[
                    definitions::make(Opcode::GetBuiltin, &[0], 1),
                    definitions::make(Opcode::Array, &[0], 1),
                    // call built-in fn 'len' with one argument
                    definitions::make(Opcode::Call, &[1], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            )))],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[0, 0], 1), // compiled fn (closure)
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}

#[test]
fn test_closures() {
    let tests = vec![CompilerTestCase {
        input: "
                fn(a) {
                    fn(b) {
                        a + b
                    }
                }
            ",

        expected_constants: vec![
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                // the real closure
                concat_instructions(&[
                    // variable 'a' defined in the enclosing scope is a
                    // 'free' variable for this scope
                    definitions::make(Opcode::GetFree, &[0], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                concat_instructions(&[
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    // #free-vars is 1 as there is one free variable on the stack
                    // that needs to be saved into the free field of the closure
                    definitions::make(Opcode::Closure, &[0, 1], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
        ],

        expected_instructions: vec![
            definitions::make(Opcode::Closure, &[1, 0], 1),
            definitions::make(Opcode::Pop, &[], 1),
        ],
    }];
    run_compiler_tests(&tests);
}

// There are three nested functions. The innermost function, the one with the
// c parameter, references two free variables: a and b. b is defined in the
// immediate enclosing scope, but a is defined in the outermost function, two
// scopes removed. The middle function is expected to contain an OpClosure
// instruction that turns the innermost function into a closure. Since the
// second operand is 2, there are supposed to be two free variables sitting
// on the stack when the VM executes it. From the perspective of the middle
// function, a is also a free variable. It is neither defined in scope nor
// as a parameter.
#[test]
fn test_nested_closures() {
    let tests = vec![CompilerTestCase {
        input: "
                fn(a) {
                    fn(b) {
                        fn(c) {
                            a + b + c
                        }
                    }
                }
            ",
        expected_constants: vec![
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                // This is the defintion of the inner-most function that has
                // two free variables 'a' and 'b' but defines no closure(s)
                // within itself (via the OpClosure instruction).
                concat_instructions(&[
                    definitions::make(Opcode::GetFree, &[0], 1),
                    definitions::make(Opcode::GetFree, &[1], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                // middle function has one free variable 'a' and defines the
                // inner-most function as a closure that has two free variables
                // The number of free variables is passed as the second arg.
                concat_instructions(&[
                    definitions::make(Opcode::GetFree, &[0], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    // two free variables on stack
                    definitions::make(Opcode::Closure, &[0, 2], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                // outer-most function has no free variables but compiles
                // the middle closure that has a single free variable
                concat_instructions(&[
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Closure, &[1, 1], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
        ],
        expected_instructions: vec![
            definitions::make(Opcode::Closure, &[2, 0], 1),
            definitions::make(Opcode::Pop, &[], 1),
        ],
    }];
    run_compiler_tests(&tests);
}

#[test]
fn test_closures_with_scopes() {
    let tests = vec![CompilerTestCase {
        input: "
                let global = 55;
                fn() {
                    let a = 66;
                    fn() {
                        let b = 77;
                        fn() {
                            let c = 88;
                            global + a + b + c;
                        }
                    }
                }
            ",
        expected_constants: vec![
            Object::Number(55.),
            Object::Number(66.),
            Object::Number(77.),
            Object::Number(88.),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                concat_instructions(&[
                    definitions::make(Opcode::Constant, &[3], 1),
                    definitions::make(Opcode::SetLocal, &[0], 1),
                    definitions::make(Opcode::GetGlobal, &[0], 1),
                    definitions::make(Opcode::GetFree, &[0], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::GetFree, &[1], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Add, &[], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                concat_instructions(&[
                    definitions::make(Opcode::Constant, &[2], 1),
                    definitions::make(Opcode::SetLocal, &[0], 1),
                    definitions::make(Opcode::GetFree, &[0], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Closure, &[4, 2], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
            Object::CompiledFunc(Rc::new(CompiledFunction::new(
                concat_instructions(&[
                    definitions::make(Opcode::Constant, &[1], 1),
                    definitions::make(Opcode::SetLocal, &[0], 1),
                    definitions::make(Opcode::GetLocal, &[0], 1),
                    definitions::make(Opcode::Closure, &[5, 1], 1),
                    definitions::make(Opcode::ReturnValue, &[], 1),
                ]),
                1,
                0,
            ))),
        ],
        expected_instructions: vec![
            definitions::make(Opcode::Constant, &[0], 1),
            definitions::make(Opcode::SetGlobal, &[0], 1),
            definitions::make(Opcode::Closure, &[6, 0], 1),
            definitions::make(Opcode::Pop, &[], 1),
        ],
    }];
    run_compiler_tests(&tests);
}

#[test]
fn test_recursive_functions() {
    let tests = vec![
        CompilerTestCase {
            input: r#"
            let countDown = fn(x) { countDown(x - 1); };
            countDown(1);
        "#,
            expected_constants: vec![
                Object::Number(1.0),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        // first load callee, the the args and then the OpCall
                        // Here, the callee is also the current closure
                        definitions::make(Opcode::CurrClosure, &[], 1),
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::Sub, &[], 1),
                        definitions::make(Opcode::Call, &[1], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    1,
                    0,
                ))),
                Object::Number(1.0),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[1, 0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Constant, &[2], 1),
                definitions::make(Opcode::Call, &[1], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
        CompilerTestCase {
            input: r#"
            let wrapper = fn() {
                let countDown = fn(x) { countDown(x - 1); };
                countDown(1);
            };
            wrapper();
        "#,
            expected_constants: vec![
                Object::Number(1.0),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::CurrClosure, &[], 1),
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::Constant, &[0], 1),
                        definitions::make(Opcode::Sub, &[], 1),
                        definitions::make(Opcode::Call, &[1], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    1,
                    0,
                ))),
                Object::Number(1.0),
                Object::CompiledFunc(Rc::new(CompiledFunction::new(
                    concat_instructions(&[
                        definitions::make(Opcode::Closure, &[1, 0], 1),
                        definitions::make(Opcode::SetLocal, &[0], 1),
                        definitions::make(Opcode::GetLocal, &[0], 1),
                        definitions::make(Opcode::Constant, &[2], 1),
                        definitions::make(Opcode::Call, &[1], 1),
                        definitions::make(Opcode::ReturnValue, &[], 1),
                    ]),
                    1,
                    0,
                ))),
            ],
            expected_instructions: vec![
                definitions::make(Opcode::Closure, &[3, 0], 1),
                definitions::make(Opcode::SetGlobal, &[0], 1),
                definitions::make(Opcode::GetGlobal, &[0], 1),
                definitions::make(Opcode::Call, &[0], 1),
                definitions::make(Opcode::Pop, &[], 1),
            ],
        },
    ];

    run_compiler_tests(&tests);
}
