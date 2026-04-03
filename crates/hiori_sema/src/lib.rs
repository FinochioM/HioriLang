mod resolver;
mod type_checker;
mod interpreter;

pub use resolver::resolve;
pub use type_checker::type_check;
pub use interpreter::interpret;