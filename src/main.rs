use std::io;
use std::io::{BufRead, Write};

mod evaluator;
mod parser;
mod scanner;
mod token;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_DESC: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() {
    println!("{} v{}", PKG_DESC, PKG_VERSION);
    println!("Ctrl+D to quit");
    run_prompt();
}

pub fn run_prompt() {
    // Define evaluator outside REPL loop so the environment is retained
    let mut evaluator = evaluator::Evaluator::new();
    let stdin = io::stdin();
    print!(">> ");
    io::stdout().flush().unwrap();
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            if !line.trim().is_empty() {
                run(&line, &mut evaluator)
            }
        }
        print!(">> ");
        io::stdout().flush().unwrap();
    }
    println!("\nExiting...");
}

fn run(source: &str, evaluator: &mut evaluator::Evaluator) {
    let scanner = scanner::Scanner::new(source);
    let mut parser = parser::Parser::new(scanner);
    let program = parser.parse_program();
    if print_parse_errors(&parser) {
        return;
    }
    let evaluated = evaluator.eval_program(program);
    match evaluated {
        Ok(obj) => {
            if !obj.is_nil() {
                println!("{}", obj);
            }
        }
        Err(err) => {
            eprintln!("{}", err);
        }
    }
}

fn print_parse_errors(parser: &parser::Parser) -> bool {
    if parser.print_errors() {
        eprintln!("{} parse errors", parser.parse_errors().len());
        true
    } else {
        false
    }
}
