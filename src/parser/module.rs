/*
 * ******************************************************************************************
 * Copyright (c) 2019 Pascal Kuthe. This file is part of the VARF project.
 * It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/jamescoding/VARF/blob/master/LICENSE.
 *  No part of VARF, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 * *****************************************************************************************
 */

use std::collections::HashSet;

use crate::ast::VariableType::{INTEGER, REAL, REALTIME, TIME};
use crate::ast::{AttributeNode, Attributes, Module, ModuleId, ModuleItem, Push};
use crate::error::Error;
use crate::parser::error;
use crate::parser::error::{Expected, Result};
use crate::parser::lexer::Token;
use crate::parser::Parser;
use crate::symbol::Ident;
use crate::symbol_table::{SymbolDeclaration, SymbolTable};

impl<'lt, 'ast, 'astref, 'source_map> Parser<'lt, 'ast, 'astref, 'source_map> {
    pub(crate) const SYMBOL_TABLE_DEFAULT_SIZE: usize = 512;
    pub(super) fn parse_module(&mut self, attributes: Attributes<'ast>) -> Result<ModuleId<'ast>> {
        let start = self.preprocessor.current_start();
        let name = self.parse_identifier(false)?;
        //parameters
        if self.look_ahead()?.0 == Token::Hash {
            self.expect(Token::ParenOpen)?;
            self.parse_parameter_list()?;
            self.expect(Token::ParenClose)?;
        }
        self.scope_stack
            .push(SymbolTable::with_capacity(Self::SYMBOL_TABLE_DEFAULT_SIZE));
        let port_list_start = self.preprocessor.current_start();
        //ports
        let (mut port_list, mut expected_ports) = if self.look_ahead()?.0 == Token::ParenOpen {
            self.lookahead.take();
            let (next_token, next_span) = self.look_ahead()?;
            let (port_list, expected_ports) = match next_token {
                Token::ParenClose => (None, None),
                Token::Input | Token::Output | Token::Inout | Token::ParenOpen => {
                    (Some(self.parse_port_declaration_list()?), None)
                }
                Token::SimpleIdentifier | Token::EscapedIdentifier => {
                    (None, Some(self.parse_port_list()?))
                }
                _ => {
                    return Err(Error {
                        error_type: error::Type::UnexpectedTokens {
                            expected: vec![Expected::PortDeclaration, Expected::Port],
                        },
                        source: next_span,
                    })
                }
            };
            self.expect(Token::ParenClose)?;
            (port_list, expected_ports)
        } else {
            (None, None)
        };
        let port_list_span = self
            .span_to_current_end(port_list_start)
            .negative_offset(start);

        self.expect(Token::Semicolon)?;
        let mut module_items = Vec::with_capacity(8);
        loop {
            let attributes = self.parse_attributes()?;
            let (token, span) = self.look_ahead()?;
            match token {
                Token::Inout | Token::Input | Token::Output => {
                    if let Some(ref mut expected) = expected_ports {
                        let declarations =
                            self.parse_port_declaration(attributes, expected, port_list_span)?;
                        if let Some(ref mut port_list) = port_list {
                            port_list.end = declarations.end;
                        } else {
                            port_list = Some(declarations);
                        }
                    } else {
                        let port_base = self.parse_port_declaration_base(attributes)?;
                        let source = self.ast[port_base]
                            .source //we do this here so that the error doesnt just underline the input token but the entire declaration instead
                            .negative_offset(start);
                        self.non_critical_errors.push(Error {
                            source: self.span_to_current_end(start),
                            error_type: error::Type::PortRedeclaration(source, port_list_span),
                        });
                    }
                }
                Token::EOF => {
                    return Err(Error {
                        error_type: error::Type::UnexpectedEof {
                            expected: vec![Token::EndModule],
                        },
                        source: span,
                    })
                }
                Token::EndModule => {
                    self.lookahead.take();
                    break;
                }
                _ => {
                    if let Some(module_item) = self.parse_module_item(attributes)? {
                        module_items.push(module_item);
                    }
                }
            }
        }
        let module = self.ast.push(AttributeNode {
            attributes,
            source: self.span_to_current_end(start),
            contents: Module {
                name,
                port_list,
                symbol_table: self.scope_stack.pop().unwrap(),
                children: vec![],
            },
        });
        self.insert_symbol(name, SymbolDeclaration::Module(module));
        Ok(module)
    }
    fn parse_port_list(&mut self) -> Result<HashSet<Ident>> {
        let mut res = HashSet::with_capacity(1);
        res.insert(self.parse_identifier(false)?);
        while self.look_ahead()?.0 == Token::Comma {
            self.lookahead.take();
            res.insert(self.parse_identifier(false)?);
        }
        Ok(res)
    }

    fn parse_parameter_list(&mut self) -> Result {
        unimplemented!()
    }

    //TODO avoid code duplication
    fn parse_module_item(
        &mut self,
        attributes: Attributes<'ast>,
    ) -> Result<Option<ModuleItem<'ast>>> {
        let res = match self.look_ahead()?.0 {
            Token::Analog => {
                self.lookahead.take();
                let res = self.parse_statement(attributes)?;
                Some(ModuleItem::AnalogStmt(self.ast.push(res)))
            }
            Token::Branch => {
                self.lookahead.take();
                self.parse_branch_declaration(attributes)?;
                None
            }
            Token::Integer => {
                self.lookahead.take();
                self.parse_variable_declaration(INTEGER, attributes)?;
                None
            }
            Token::Real => {
                self.lookahead.take();
                self.parse_variable_declaration(REAL, attributes)?;
                None
            }
            Token::Realtime => {
                self.lookahead.take();
                self.parse_variable_declaration(REALTIME, attributes)?;
                None
            }
            Token::Time => {
                self.lookahead.take();
                self.parse_variable_declaration(TIME, attributes)?;
                None
            }

            _ => {
                self.parse_net_declaration(attributes)?;
                None
            }
        };
        Ok(res)
    }
}
