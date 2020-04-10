/*
 * ******************************************************************************************
 * Copyright (c) 2019 Pascal Kuthe. This file is part of the VARF project.
 * It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/jamescoding/VARF/blob/master/LICENSE.
 *  No part of VARF, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 * *****************************************************************************************
 */

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;

use ansi_term::Color::*;
use bumpalo::Bump;
use indexmap::map::IndexMap;
use log::*;

pub use source_map::SourceMap;
use source_map::SourceMapBuilder;
pub(crate) use source_map::{ArgumentIndex, CallDepth};

use crate::parser::error;
use crate::parser::error::{List, Type, Unsupported, Warning,Error};
use crate::parser::lexer::Token;
use crate::parser::primaries::parse_string;
use crate::span::{Index, IndexOffset, LineNumber, Range};
use crate::{Lexer, Span};

use super::Result;

mod source_map;

#[cfg(test)]
pub mod test;

enum TokenSource<'lt> {
    File(Lexer<'lt>),
    Insert(Peekable<IntoIter<MacroBodyElement<'lt>>>, bool),
}

impl<'lt> Debug for TokenSource<'lt> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            TokenSource::File(_) => f.write_str("FILE")?,
            TokenSource::Insert(ref iter, is_macro) => f.write_fmt(format_args!(
                "{}:{:?}",
                if *is_macro { "Macro:" } else { "" },
                iter
            ))?,
        }
        Ok(())
    }
}

type MacroBodyElement<'lt> = (Span, MacroBodyToken<'lt>);

#[derive(Clone, Debug)]
enum MacroBodyToken<'lt> {
    ArgumentReference(ArgumentIndex, CallDepth),
    LexerToken(Token),
    MacroReference(UnresolvedMacroReference<'lt>),
}

#[derive(Debug, Clone)]
struct Macro<'lt> {
    body: Vec<MacroBodyElement<'lt>>,
    arg_count: ArgumentIndex,
    source: &'lt str,
    line: LineNumber,
}

#[derive(Debug, Clone)]
struct UnresolvedMacroReference<'lt> {
    name: &'lt str,
    source: &'lt str,
    arg_bindings: Vec<MacroArg<'lt>>,
}

#[derive(Debug)]
/// Represents something that has been inserted into the preprocessor (macro reference / file inculde / main file)
/// * `start` - the start of this Insertion in the source map
/// * `offset` represents the difference between sourcemap- and local location of a token from this insert
///  (for this first token this is just start but for later tokens this may increase as other things are inserted inside the Intersertion (eg macros inside macros)
struct Insertion<'lt> {
    start: Index,
    offset: IndexOffset,
    token_source: TokenSource<'lt>,
}

impl<'lt> Insertion<'lt> {
    pub fn new(start: Index, token_source: TokenSource<'lt>) -> Self {
        Self {
            start,
            offset: start as IndexOffset,
            token_source,
        }
    }
}

#[derive(Debug, Clone)]
struct MacroArg<'lt> {
    tokens: Vec<MacroBodyElement<'lt>>,
    source: &'lt str,
}

impl<'lt> MacroArg<'lt> {
    pub fn new(tokens: Vec<MacroBodyElement<'lt>>, source: &'lt str) -> Self {
        Self { tokens, source }
    }
}
pub struct PreprocessorBuilder<'lt, 'source_map> {
    preprocessor: Preprocessor<'lt, 'source_map>,
}
impl<'lt, 'source_map> PreprocessorBuilder<'lt, 'source_map> {
    pub fn new(
        allocator: &'lt Bump,
        source_map_allocator: &'source_map Bump,
        main_file: &Path,
    ) -> std::io::Result<Self> {
        let (source_map_builder, main_lexer) =
            SourceMapBuilder::new(source_map_allocator, allocator, main_file)?;
        let mut res = Preprocessor {
            macros: HashMap::new(),
            called_macros: IndexMap::new(),
            source_map_builder,
            state_stack: Vec::new(),
            condition_stack: Vec::new(),
            current_token: Token::EOF,
            current_source_start: 0,
            current_start: 0,
            current_len: 0,
        };
        res.current_token = main_lexer.token();
        res.current_len = main_lexer.token_len();
        res.state_stack
            .push(Insertion::new(0, TokenSource::File(main_lexer)));
        Ok(Self { preprocessor: res })
    }
    pub fn init(mut self) -> (Result, Preprocessor<'lt, 'source_map>) {
        (self.preprocessor.process_token_until_success(),self.preprocessor)
    }
}
pub struct Preprocessor<'lt, 'source_map> {
    //internal state
    macros: HashMap<&'lt str, Macro<'lt>>,
    called_macros: IndexMap<&'lt str, Vec<MacroArg<'lt>>>,
    source_map_builder: SourceMapBuilder<'lt, 'source_map>,
    state_stack: Vec<Insertion<'lt>>,
    condition_stack: Vec<Span>,
    // Start of the current token in the source that contains it. (Either a file or a macro definition)
    current_source_start: Index,
    current_start: Index,
    current_len: Index,
    current_token: Token,
}

impl<'lt, 'source_map> Preprocessor<'lt, 'source_map> {
    pub fn done(self) -> &'source_map SourceMap<'source_map> {
        if self.current_token != Token::EOF {
            error!(
                "{}{}",
                Red.bold().paint("error"),
                Fixed(253)
                    .bold()
                    .paint(": preprocessor.done() was called before EOF file reached ")
            );
            panic!();
        }
        self.source_map_builder.done()
    }

    fn include(&mut self, file: &Path, include_directive_span: Span) -> Result {
        let lexer = self
            .source_map_builder
            .enter_file(file, self.current_start, include_directive_span)
            .map_err(|io_err| Error {
                source: include_directive_span.signed_offset(self.current_offset()),
                error_type: io_err.into(),
            })?;

        self.state_stack.last_mut().unwrap().offset -=
            include_directive_span.get_len() as IndexOffset;

        self.state_stack
            .push(Insertion::new(self.current_start, TokenSource::File(lexer)));

        Ok(())
    }

    /// Advances the current state of the preprocessor
    /// This returns an error if a macro couldnt' be resolved however it always consumes to current token so the same error cant be returned twice
    fn advance_state(&mut self) -> Result {
        let (token, range) = self.next()?;
        self.current_token = token;
        self.current_source_start = range.start;
        self.current_start = ((range.start as IndexOffset + self.current_offset()) as Index);
        self.current_len = range.end - range.start;
        Ok(())
    }

    /// This function advances to the next token from either a lexical source or an insertion source (macros and their arguments)
    /// This returns an error if a macro couldn't be resolved however it always consumes to current token so the same error cant be returned twice
    fn next(&mut self) -> Result<(Token, Range)> {
        loop {
            let main_file = self.state_stack.len() == 1;
            let current_state = self.state_stack.last_mut().unwrap();
            let new_state = {
                match current_state.token_source {
                    TokenSource::File(ref mut lexer) => {
                        lexer.advance();
                        if lexer.token() != Token::EOF || main_file {
                            return Ok((lexer.token(), lexer.range()));
                        } else {
                            None
                        }
                    }
                    TokenSource::Insert(ref mut iter, _) => {
                        if let Some(macro_body_element) = iter.next() {
                            let span = macro_body_element.0;
                            match macro_body_element.1 {
                                MacroBodyToken::LexerToken(token) => {
                                    return Ok((token, span.into()));
                                }
                                MacroBodyToken::ArgumentReference(id, macro_depth) => {
                                    let arg = self
                                        .called_macros
                                        .get_index(
                                            self.called_macros.len() - (macro_depth as usize),
                                        )
                                        .unwrap()
                                        .1
                                        .get(id as usize)
                                        .unwrap();
                                    let token_source = TokenSource::Insert(
                                        arg.tokens.clone().into_iter().peekable(),
                                        false,
                                    );
                                    let start = (span.get_start() as IndexOffset
                                        + current_state.offset)
                                        as Index;
                                    current_state.offset -= span.get_len() as IndexOffset;
                                    self.source_map_builder
                                        .enter_non_root_substitution(span, arg.source);
                                    Some((start, token_source))
                                }
                                MacroBodyToken::MacroReference(unresolved_reference) => {
                                    let start = (span.get_start() as IndexOffset
                                        + current_state.offset)
                                        as Index;
                                    let token_source =
                                        self.resolve_macro_reference(span, unresolved_reference)?;
                                    Some((start, token_source))
                                }
                            }
                        } else {
                            None
                        }
                    }
                }
            };

            //the token source stack can only be mutated here and not inside the match
            if let Some(state) = new_state {
                self.state_stack.push(Insertion::new(state.0, state.1))
            } else {
                let old_insertion = self.state_stack.pop().expect("Main file was popped");

                let original_length = self.source_map_builder.finish_substitution();
                // This represents the length that was added to the insertion due to other macros/includes being expanded inside it
                let added_length = old_insertion.offset - old_insertion.start as IndexOffset;
                self.state_stack.last_mut().unwrap().offset +=
                    added_length + original_length as IndexOffset;

                match old_insertion.token_source {
                    TokenSource::Insert(_, is_macro) if is_macro => {
                        self.called_macros.pop();
                    }
                    _ => (),
                };
            }
        }
    }

    pub fn advance(&mut self) -> Result {
        self.advance_state()?;
        self.process_token_until_success()
    }
    fn process_token_until_success(&mut self)->Result{
        if let Err(error) = self.process_token() {
            self.advance_state();
            while self.process_token().is_err() {
                self.advance_state();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn process_token(&mut self) -> Result {
        loop {
            // advance state until error occurs or token not handled by the preprocessor is encountered
            match self.current_token {
                Token::UnexpectedEOF => {
                    let error = error::Type::UnexpectedEof {
                        expected: vec![Token::CommentEnd],
                    };
                    return self.token_error(error);
                }
                Token::Newline | Token::CommentNewline => self.source_map_builder.new_line(),
                Token::MacroDefNewLine => {
                    self.source_map_builder.new_line();
                    return self.token_error(Type::UnexpectedToken { expected: vec![] })
                }
                Token::Include => {
                    let start = self.current_source_start;
                    self.advance_state()?;
                    self.consume(Token::LiteralString)?;
                    let mut path_str = parse_string(self.slice());
                    match path_str.as_str() {
                        "constants.va" | "constants.vams" | "constants.v" => {
                            let mut path = PathBuf::from(
                                std::env::var_os("VAMS_STD")
                                    .expect("VAMS_STD enviorment variable not set"),
                            );
                            path.push("constants.va");
                            path_str = String::from(path.to_str().unwrap())
                        }
                        "disciplines.va" | "disciplines.vams" | "disciplines.v" => {
                            let mut path = PathBuf::from(
                                std::env::var_os("VAMS_STD")
                                    .expect("VAMS_STD enviorment variable not set"),
                            );
                            path.push("disciplines.va");
                            path_str = String::from(path.to_str().unwrap())
                        }
                        _ => (),
                    };
                    let path = Path::new(&path_str);
                    self.include(
                        path,
                        Span::new(start, self.current_source_start + self.current_len),
                    )?;
                }
                Token::MacroReference => {
                    let unresolved_reference = self.parse_reference(&Vec::new(), 0)?;

                    let start = (unresolved_reference.0.get_start() as IndexOffset
                        + self.current_offset()) as Index;

                    let token_source = self
                        .resolve_macro_reference(unresolved_reference.0, unresolved_reference.1)?;

                    self.state_stack.push(Insertion::new(start, token_source));
                }
                Token::MacroIf => self.process_condition(false)?,
                Token::MacroIfn => self.process_condition(true)?,
                Token::MacroEndIf => {
                    if self.condition_stack.is_empty() {
                        self.advance_state()?;
                        return self.token_error(error::Type::ConditionEndWithoutStart);
                    } else {
                        self.condition_stack.pop();
                    }
                }
                Token::MacroElse | Token::MacroElsif => self.skip_to_condition_end()?, //When a condition is first encountered we skip all irrelevant parts. So if we encounter an else or elseif that means a prior condition inside the condition block has already matched and we can skip this
                Token::EOF if !self.condition_stack.is_empty() => {
                    let error = error::Type::UnclosedConditions(self.condition_stack.clone());
                    self.condition_stack.clear(); //We call this  so this error doesnt reoccur
                    return Err(Error {
                        source: self.span().negative_offset(1),
                        error_type: error,
                    });
                }
                Token::EOF if self.state_stack.len() != 1 => {
                    unreachable!("Unclosed token_sources! {:?}", self.state_stack);
                }
                Token::MacroDef => {
                    self.parse_definition()?;
                }
                _ => {
                    return Ok(());
                }
            }
            self.advance_state()?;
        }
    }

    fn process_condition(&mut self, mut invert: bool) -> Result {
        self.advance_state()?;
        let mut start = self.current_start;
        loop {
            self.consume(Token::SimpleIdentifier)?;
            let name = self.slice();
            if invert ^ self.macros.contains_key(name) {
                debug!(
                    "Condition if{}def {} is fulfilled",
                    if invert { "n" } else { "" },
                    self.slice()
                );
                self.condition_stack
                    .push(Span::new(start, self.current_start + self.current_len));
                return Ok(());
            } else {
                debug!(
                    "Condition if{}def {} is not fulfilled",
                    if invert { "n" } else { "" },
                    self.slice()
                );
                loop {
                    match self.current_token {
                        Token::MacroElsif => {
                            invert = false; //elsif is never inverted according to standard (the compiler should (hopefully) figure out that after this point invert ^ x = x)
                            start = self.current_start;
                            break;
                        }
                        Token::MacroEndIf => return Ok(()),
                        Token::MacroElse => {
                            self.condition_stack.push(self.span());
                            return Ok(());
                        }
                        Token::UnexpectedEOF => {
                            let error = error::Type::UnexpectedEof {
                                expected: vec![Token::CommentEnd],
                            };
                            return self.token_error(error);
                        }
                        Token::Newline | Token::CommentNewline => {
                            self.source_map_builder.new_line();
                            self.advance_state()?
                        }
                        Token::MacroIf | Token::MacroIfn => self.skip_to_condition_end()?,
                        _ => self.advance_state()?,
                    }
                }
            };
        }
    }

    fn skip_to_condition_end(&mut self) -> Result {
        self.advance_state()?;
        let mut ignore_conditions: Vec<Span> = Vec::new();
        loop {
            match self.current_token {
                Token::MacroEndIf if ignore_conditions.is_empty() => {
                    self.condition_stack.pop();
                    return Ok(());
                }
                Token::MacroIfn | Token::MacroIf => {
                    ignore_conditions.push(self.span());
                }
                Token::MacroEndIf => {
                    ignore_conditions.pop();
                }
                Token::Newline | Token::CommentNewline => {
                    self.source_map_builder.new_line();
                }
                Token::EOF => {
                    let error = error::Type::UnclosedConditions(ignore_conditions);
                    return Err(Error {
                        source: self.span().negative_offset(1),
                        error_type: error,
                    });
                }
                _ => (),
            }
            self.advance_state()?
        }
    }

    fn parse_definition(&mut self) -> Result {
        self.advance_state()?;
        let (name, args) = match self.current_token {
            Token::SimpleIdentifier => (self.slice(), Vec::new()),

            Token::SimpleIdentWithBracket => {
                let name = &self.slice()[..self.slice().len() - 1];
                self.advance_state()?;
                let mut args = Vec::with_capacity(2);
                self.consume(Token::SimpleIdentifier)?;
                args.push(self.slice());
                loop {
                    self.advance_state()?;
                    match self.current_token {
                        Token::Newline | Token::CommentNewline => {
                            self.source_map_builder.new_line();
                            self.advance_state()?
                        }
                        Token::ParenClose => {
                            break;
                        }
                        Token::Comma => {
                            self.advance_state()?;
                            self.consume(Token::SimpleIdentifier)?;
                            args.push(self.slice());
                        }
                        Token::EOF => {
                            return self.token_error(error::Type::UnexpectedEof {
                                expected: vec![Token::ParenClose],
                            });
                        }
                        _ => {
                            return self.token_error(error::Type::UnexpectedToken {
                                expected: vec![Token::ParenClose, Token::Comma],
                            });
                        }
                    }
                }
                (name, args)
            }
            _ => {
                let error = error::Type::UnexpectedToken {
                    expected: vec![Token::SimpleIdentifier],
                };
                let error = self.token_error(error);
                self.advance_state();
                return error;
            }
        };
        let mut peek = self.peek()?;
        let body_start = peek.0.start;
        let line = self.source_map_builder.current_root_line();
        let mut body = Vec::new();
        while peek.1 != Token::Newline && peek.1 != Token::EOF {
            self.advance_state()?;
            body.push(self.current_macro_body_token(body_start, &args, 1)?);
            peek = self.peek()?;
        }
        let decl_source =
            &self.source()[body_start as usize..peek.0.end as usize];
        let maco_decl = Macro {
            body,
            arg_count: args.len() as ArgumentIndex,
            source: decl_source,
            line,
        };
        if let Some(old) = self.macros.insert(name, maco_decl) {
            /*Warning {
                error_type: WarningType::MacroOverwritten(old.span),
                source: decl_range,
            }.print() TODO warning architecture for preprocessor*/
        }
        Ok(())
    }

    fn peek(&mut self) -> Result<(Range, Token)> {
        let in_main_file = self.state_stack.len() == 1;
        match self.state_stack.last_mut().unwrap().token_source {
            TokenSource::File(ref lexer) => {
                let (range,token) = lexer.peek();
                if token == Token::EOF&&!in_main_file{
                    Err(Error {
                        source: Span::new(range.start,range.end),
                        error_type: error::Type::CompilerDirectiveSplit,
                    })
                }else {
                    Ok((range,token))
                }
            }
            TokenSource::Insert(ref mut iter, _) => {
                let res = iter.peek().unwrap();
                let token = if let MacroBodyToken::LexerToken(token) = res.1 {
                    token
                } else {
                    return Err(Error {
                        source: res.0,
                        error_type: error::Type::CompilerDirectiveSplit,
                    });
                };
                Ok((res.0.into(), token))
            }
        }
    }

    fn current_macro_body_token(
        &mut self,
        start: Index,
        args: &[&'lt str],
        depth: CallDepth,
    ) -> Result<MacroBodyElement<'lt>> {
        let res = match self.current_token {
            Token::SimpleIdentifier => {
                let identifier = self.slice();
                if let Some(index) = args.iter().position(|arg_name| *arg_name == identifier) {
                    MacroBodyToken::ArgumentReference(index as ArgumentIndex, depth)
                } else {
                    MacroBodyToken::LexerToken(Token::SimpleIdentifier)
                }
            }
            Token::MacroReference => {
                let res = self.parse_reference(args, depth + 1)?;
                let span = Span::new_with_length(res.0.get_start() - start, res.0.get_len());
                return Ok((span, MacroBodyToken::MacroReference(res.1)));
            } //we only do this here to avoid recomputing call depth constantly also this parse is lightly more expensive so its nice for performance too. Not worth it for other directives (except definition; see below) since parsing is trivial for those
            //TODO parse macro definition here
            Token::MacroDef => {
                return self.token_error(Type::Unsupported(Unsupported::MacroDefinedInMacro))
            }
            Token::UnexpectedEOF => {
                let error = error::Type::UnexpectedEof {
                    expected: vec![Token::CommentEnd],
                };
                return self.token_error(error);
            }
            Token::MacroDefNewLine => {
                self.source_map_builder.new_line();
                MacroBodyToken::LexerToken(Token::Newline)
            } //map to newline so we can keep track of lines inside macros
            Token::Newline => return self.token_error(error::Type::MacroEndTooEarly),
            token => MacroBodyToken::LexerToken(token),
        };
        Ok((
            Span::new_with_length(self.current_source_start - start, self.current_len),
            res,
        ))
    }

    fn parse_reference(
        &mut self,
        parent_macro_args: &[&'lt str],
        macro_call_depth: CallDepth,
    ) -> Result<(Span, UnresolvedMacroReference<'lt>)> {
        let name = &self.slice()[1..];
        let start = self.current_start;
        let source_start = self.current_source_start;
        let mut arg_bindings: Vec<MacroArg> = Vec::new();
        let mut source_end = self.current_source_start + self.current_len;
        let peek = self.peek()?.1;
        if peek == Token::ParenOpen {
            self.advance_state()?; //skip bracket
            self.advance_state()?;
            let mut current_arg_body = Vec::new();
            let mut last_colon = start;
            let mut current_arg_start = self.current_source_start;
            let mut depth = 0;
            loop {
                match self.current_token {
                    Token::ParenClose if depth == 0 => break,
                    Token::ParenOpen => {
                        depth += 1;
                        current_arg_body.push(self.current_macro_body_token(
                            current_arg_start,
                            parent_macro_args,
                            macro_call_depth,
                        )?)
                    }
                    Token::ParenClose => {
                        depth -= 1;
                        current_arg_body.push(self.current_macro_body_token(
                            current_arg_start,
                            parent_macro_args,
                            macro_call_depth,
                        )?)
                    }
                    Token::Comma => {
                        if current_arg_body.is_empty() {
                            return Err(Error {
                                error_type: error::Type::EmptyListEntry(List::MacroArgument),
                                source: Span::new(
                                    last_colon,
                                    self.current_start + self.current_len,
                                ),
                            });
                        }
                        let source = &self.source()
                            [current_arg_start as usize..self.current_source_start as usize];
                        arg_bindings.push(MacroArg::new(current_arg_body, source));
                        current_arg_body = Vec::new();
                        last_colon = self.current_start;
                        current_arg_start = self.current_source_start + 1;
                        //colon is not included
                    }
                    Token::EOF => {
                        return self.token_error(error::Type::UnexpectedEof {
                            expected: vec![Token::ParenClose],
                        });
                    }
                    _ => current_arg_body.push(self.current_macro_body_token(
                        current_arg_start,
                        parent_macro_args,
                        macro_call_depth,
                    )?),
                }
                self.advance_state()?;
            }
            if current_arg_body.is_empty() {
                return Err(Error {
                    error_type: error::Type::EmptyListEntry(List::MacroArgument),
                    source: Span::new(last_colon, self.current_start + self.current_len),
                });
            }
            let source =
                &self.source()[current_arg_start as usize..self.current_source_start as usize];
            arg_bindings.push(MacroArg::new(current_arg_body, source));
            source_end = self.current_source_start + 1;
        }

        let source = &self.source()[source_start as usize..source_end as usize];
        Ok((
            Span::new(source_start, source_end),
            UnresolvedMacroReference {
                name,
                source,
                arg_bindings,
            },
        ))
    }

    fn resolve_macro_reference(
        &mut self,
        source_span: Span,
        reference: UnresolvedMacroReference<'lt>,
    ) -> Result<TokenSource<'lt>> {
        let span = source_span.signed_offset(self.current_offset());
        if self.called_macros.contains_key(&reference.name) {
            return Err(Error {
                error_type: error::Type::MacroRecursion,
                source: span,
            });
        }
        if let Some(definition) = self.macros.get(reference.name) {
            if reference.arg_bindings.len() as ArgumentIndex != definition.arg_count {
                Err(Error {
                    error_type: error::Type::MacroArgumentCount {
                        found: reference.arg_bindings.len() as ArgumentIndex,
                        expected: definition.arg_count,
                    },
                    source: span,
                })
            } else {
                self.source_map_builder.enter_macro(
                    span.get_start(),
                    source_span,
                    definition.source,
                    definition.line,
                    &reference.name,
                );

                self.called_macros
                    .insert(reference.name, reference.arg_bindings.clone());

                self.state_stack.last_mut().unwrap().offset -= source_span.get_len() as IndexOffset;
                let token_source =
                    TokenSource::Insert(definition.body.clone().into_iter().peekable(), true);
                Ok(token_source)
            }
        } else {
            Err(Error {
                error_type: error::Type::MacroNotFound,
                source: span,
            })
        }
    }

    pub fn current_token(&self) -> Token {
        self.current_token
    }

    fn token_error<T>(&self, etype: error::Type) -> Result<T> {
        Err(Error {
            source: self.span(),
            error_type: etype,
        })
    }

    pub fn span(&self) -> Span {
        Span::new_with_length(self.current_start, self.current_len)
    }

    pub fn current_start(&self) -> Index {
        self.current_start
    }

    pub fn current_end(&self) -> Index {
        self.current_start + self.current_len
    }

    pub fn current_len(&self) -> Index {
        self.current_len
    }

    fn current_offset(&self) -> IndexOffset {
        self.state_stack.last().unwrap().offset
    }

    /// Returns the source text of the current scope (macro / file include / main file)
    fn source(&self) -> &'lt str {
        self.source_map_builder.source()
    }

    pub fn slice(&self) -> &'lt str {
        let source = self.source();
        &source[self.current_source_start as usize
            ..(self.current_source_start + self.current_len) as usize]
    }

    fn consume(&mut self, token: Token) -> Result<()> {
        if self.current_token != token {
            let error = error::Type::UnexpectedToken {
                expected: vec![token],
            };
            let error = self.token_error(error);
            error
        } else {
            Ok(())
        }
    }
}
