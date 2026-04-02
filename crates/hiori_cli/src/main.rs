use std::process;
use hiori_diagnostics::report;
use hiori_lexer::Lexer;
use hiori_parser::Parser;
use hiori_sema::{resolve, type_check};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let source = match args.get(1) {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("error: could not read '{}': {}", path, err);
                process::exit(1);
            }
        },
        None => {
            eprintln!("usage: hiori <file>");
            process::exit(1);
        }
    };

    let (tokens, mut diagnostics) = Lexer::new(&source).tokenize();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    diagnostics.extend(parser.finish());

    if diagnostics.is_empty() {
        diagnostics.extend(resolve(&program));
    }

    if diagnostics.is_empty() {
        diagnostics.extend(type_check(&program));
    }

    if !diagnostics.is_empty() {
        report(&source, &diagnostics);
        process::exit(1);
    }

    println!("{:#?}", program);
}