//! Riven REPL — Interactive shell for the Riven programming language.
//!
//! Uses Cranelift JIT for in-process compilation and execution.

#[allow(dead_code)]
mod commands;
mod completion;
#[allow(dead_code)]
mod display;
#[allow(dead_code)]
mod env;
mod eval;
mod highlight;
#[allow(dead_code)]
mod jit;
mod session;
mod validate;

use std::borrow::Cow;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Config, Context, EditMode, Editor, Helper};

use completion::RivenCompleter;
use highlight::RivenHighlighter;
use session::ReplSession;
use validate::RivenValidator;

/// Split a piped-stdin buffer into logical REPL chunks.
///
/// Accumulates lines until delimiter balance returns to zero (no
/// open `def`/`class`/`enum`/`(`/`[`/`{` etc.), then emits the
/// accumulated chunk. Used when stdin is not a TTY so multi-line
/// items like `class Foo ... end` are evaluated as one input.
fn split_repl_chunks(input: &str) -> Vec<String> {
    use riven_core::lexer::Lexer;
    use riven_core::lexer::token::TokenKind;

    let mut chunks = Vec::new();
    let mut current = String::new();
    for raw_line in input.split_inclusive('\n') {
        current.push_str(raw_line);
        let trimmed = current.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Commands are single-line: emit immediately.
        if trimmed.starts_with(':') {
            chunks.push(std::mem::take(&mut current));
            continue;
        }
        let mut block: i32 = 0;
        let mut paren: i32 = 0;
        let mut bracket: i32 = 0;
        let mut brace: i32 = 0;
        let mut unclosed_string = false;
        if let Ok(tokens) = Lexer::new(&current).tokenize() {
            for t in &tokens {
                match &t.kind {
                    TokenKind::Def | TokenKind::Class | TokenKind::Struct
                    | TokenKind::Enum | TokenKind::Trait | TokenKind::Impl
                    | TokenKind::Module | TokenKind::If | TokenKind::While
                    | TokenKind::For | TokenKind::Loop | TokenKind::Match
                    | TokenKind::Do => block += 1,
                    TokenKind::End => block -= 1,
                    TokenKind::LParen => paren += 1,
                    TokenKind::RParen => paren -= 1,
                    TokenKind::LBracket => bracket += 1,
                    TokenKind::RBracket => bracket -= 1,
                    TokenKind::LBrace => brace += 1,
                    TokenKind::RBrace => brace -= 1,
                    TokenKind::Eof => break,
                    _ => {}
                }
            }
        } else {
            // Unbalanced quotes → keep accumulating.
            if current.chars().filter(|&c| c == '"').count() % 2 != 0 {
                unclosed_string = true;
            }
        }
        let balanced = block <= 0 && paren <= 0 && bracket <= 0 && brace <= 0 && !unclosed_string;
        if balanced {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Combined rustyline helper implementing all traits.
struct RivenHelper {
    completer: RivenCompleter,
    highlighter: RivenHighlighter,
    validator: RivenValidator,
}

impl Completer for RivenHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Highlighter for RivenHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        self.highlighter.highlight_prompt(prompt, default)
    }
}

impl Hinter for RivenHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Validator for RivenHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        self.validator.validate(ctx)
    }
}

impl Helper for RivenHelper {}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PRIMARY_PROMPT: &str = "riven> ";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" => {
                println!("riven-repl {}", VERSION);
                return;
            }
            "--help" | "-h" => {
                println!("riven-repl {} — interactive Riven REPL", VERSION);
                println!();
                println!("Usage: riven-repl");
                println!();
                println!("Once inside, type :help for REPL commands.");
                return;
            }
            _ => {}
        }
    }

    println!("Riven {} REPL — Type :help for commands", VERSION);

    let mut session = match ReplSession::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            std::process::exit(1);
        }
    };

    // Non-TTY stdin (piped input): read the whole stream and split into
    // logical chunks by lexer-balance so multi-line `class`/`enum`/`trait`
    // bodies stay together. rustyline's line-at-a-time readline only
    // marks a chunk incomplete if `eval_input` errors with Incomplete,
    // but the parser isn't reliable about that for all opener keywords
    // — a whole-stream pass is robust.
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_ok() {
            for chunk in split_repl_chunks(&buf) {
                let trimmed = chunk.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match eval::eval_input(&mut session, &chunk) {
                    eval::EvalResult::Ok(Some(output)) => println!("{}", output),
                    eval::EvalResult::Ok(None) => {}
                    eval::EvalResult::Command(output) => println!("{}", output),
                    eval::EvalResult::Quit => {
                        println!("Goodbye!");
                        return;
                    }
                    eval::EvalResult::Incomplete => {
                        eprintln!("Error: Incomplete input: {}", trimmed);
                    }
                    eval::EvalResult::Error(msg) => eprintln!("{}", msg),
                }
            }
        }
        println!("Goodbye!");
        return;
    }

    let config = Config::builder()
        .edit_mode(EditMode::Emacs)
        .auto_add_history(true)
        .build();

    let helper = RivenHelper {
        completer: RivenCompleter,
        highlighter: RivenHighlighter,
        validator: RivenValidator,
    };

    let mut editor: Editor<RivenHelper, _> = match Editor::with_config(config) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to create editor: {}", e);
            std::process::exit(1);
        }
    };
    editor.set_helper(Some(helper));

    let _ = editor.load_history(&session.history_path);

    // Accumulates continuation lines while the evaluator reports
    // `Incomplete` (e.g., an unclosed `def ... end` block when input is
    // piped, where rustyline's interactive multi-line handling does not
    // kick in).
    let mut pending: String = String::new();

    loop {
        let prompt = if pending.is_empty() { PRIMARY_PROMPT } else { "..... " };
        let readline = editor.readline(prompt);

        match readline {
            Ok(line) => {
                if pending.is_empty() && line.trim().is_empty() {
                    continue;
                }

                // Combine pending input with the new line.
                let to_eval = if pending.is_empty() {
                    line.clone()
                } else {
                    let mut combined = std::mem::take(&mut pending);
                    combined.push('\n');
                    combined.push_str(&line);
                    combined
                };

                match eval::eval_input(&mut session, &to_eval) {
                    eval::EvalResult::Ok(Some(output)) => println!("{}", output),
                    eval::EvalResult::Ok(None) => {}
                    eval::EvalResult::Command(output) => println!("{}", output),
                    eval::EvalResult::Quit => {
                        println!("Goodbye!");
                        break;
                    }
                    eval::EvalResult::Incomplete => {
                        // Keep accumulating until the parser accepts it.
                        pending = to_eval;
                    }
                    eval::EvalResult::Error(msg) => eprintln!("{}", msg),
                }
            }
            Err(ReadlineError::Interrupted) => {
                pending.clear();
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = editor.save_history(&session.history_path);
}
