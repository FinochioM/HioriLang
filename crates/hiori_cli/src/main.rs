use std::process;
use hiori_diagnostics::report;
use hiori_lexer::Lexer;
use hiori_parser::Parser;

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
    let expr = parser.parse();
    diagnostics.extend(parser.finish());

    if !diagnostics.is_empty() {
        report(&source, &diagnostics);
        process::exit(1);
    }

    println!("{:#?}", expr.expect("no diagnostics but no expression — this is a compiler bug"));
}