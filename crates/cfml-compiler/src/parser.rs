//! CFML Parser - Converts tokens to AST

use crate::ast::*;
use crate::lexer::{Lexer, TokenWithLoc};
use crate::token::Token;
use cfml_common::position::SourceLocation;
use std::convert::TryFrom;

pub struct Parser {
    tokens: Vec<TokenWithLoc>,
    current: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Parser {
    pub fn new(source: String) -> Self {
        let tokens = Lexer::new(source).tokenize();
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        Ok(Program {
            statements,
            location: self.current_location(),
        })
    }

    fn is_at_end(&self) -> bool {
        matches!(&self.tokens[self.current].token, Token::Eof)
    }

    fn peek(&self, offset: usize) -> &Token {
        let idx = self.current + offset;
        if idx >= self.tokens.len() {
            return &Token::Eof;
        }
        &self.tokens[idx].token
    }

    fn current_location(&self) -> SourceLocation {
        if self.current < self.tokens.len() {
            self.tokens[self.current].location
        } else {
            SourceLocation::default()
        }
    }

    fn advance(&mut self) -> TokenWithLoc {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> TokenWithLoc {
        self.tokens[self.current - 1].clone()
    }

    fn check(&self, token: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(self.peek(0)) == std::mem::discriminant(token)
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            return true;
        }
        false
    }

    #[allow(dead_code)]
    fn match_any(&mut self, tokens: &[Token]) -> Option<Token> {
        for token in tokens {
            if self.check(token) {
                let t = self.advance().token.clone();
                return Some(t);
            }
        }
        None
    }

    /// True if the token at `offset` is an identifier equal (case-insensitive)
    /// to `word`. Used to recognise the verbose, multi-word comparison operators
    /// (GREATER THAN, DOES NOT CONTAIN, EQUAL, ...) whose words are NOT reserved
    /// keywords — they lex as identifiers so they remain usable as variable names.
    fn peek_word(&self, offset: usize, word: &str) -> bool {
        matches!(self.peek(offset), Token::Identifier(s) if s.eq_ignore_ascii_case(word))
    }

    /// Consume an identifier matching `word` (case-insensitive); returns whether
    /// it was consumed.
    fn match_word(&mut self, word: &str) -> bool {
        if self.peek_word(0, word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_error(&self, message: &str) -> ParseError {
        let loc = self.current_location();
        ParseError {
            message: format!("{} (found {:?})", message, self.peek(0)),
            line: loc.start.line,
            column: loc.start.column,
        }
    }

    // ---- Statement Parsing ----

    fn parse_statement(&mut self) -> Result<CfmlNode, ParseError> {
        let stmt_loc = self.current_location();

        // Check for access modifiers before function — but only if followed by
        // function, static, or a return type + function.  Otherwise "private" etc.
        // are valid as variable names in CFML.
        if matches!(
            self.peek(0),
            Token::Public | Token::Private | Token::Remote | Token::Package
        ) && self.is_access_modifier_for_function()
        {
            let access = self.parse_access_modifier();
            // Skip optional return type annotation (e.g. "private array function ..."
            // or "public MachII.framework.AppManager function ...")
            if matches!(self.peek(0), Token::Identifier(_)) {
                // Look ahead past dotted name: Ident.Ident.Ident... then Function
                let mut lookahead = 1;
                while matches!(self.peek(lookahead), Token::Dot) && matches!(self.peek(lookahead + 1), Token::Identifier(_)) {
                    lookahead += 2;
                }
                if matches!(self.peek(lookahead), Token::Function) {
                    for _ in 0..lookahead {
                        self.advance(); // skip return type tokens
                    }
                }
            }
            if self.match_token(&Token::Function) {
                let mut func = self.parse_function()?;
                func.access = access;
                return Ok(CfmlNode::Statement(Statement::FunctionDecl(FunctionDecl {
                    func,
                })));
            }
            if self.match_token(&Token::Static) {
                // Skip optional return type after static (including dotted names)
                if matches!(self.peek(0), Token::Identifier(_)) {
                    let mut lookahead = 1;
                    while matches!(self.peek(lookahead), Token::Dot) && matches!(self.peek(lookahead + 1), Token::Identifier(_)) {
                        lookahead += 2;
                    }
                    if matches!(self.peek(lookahead), Token::Function) {
                        for _ in 0..lookahead {
                            self.advance();
                        }
                    }
                }
                if self.match_token(&Token::Function) {
                    let mut func = self.parse_function()?;
                    func.access = access;
                    func.is_static = true;
                    return Ok(CfmlNode::Statement(Statement::FunctionDecl(FunctionDecl {
                        func,
                    })));
                }
            }
        }

        if self.match_token(&Token::Var) {
            return Ok(CfmlNode::Statement(Statement::Var(self.parse_var()?)));
        }

        if self.match_token(&Token::If) {
            return Ok(CfmlNode::Statement(Statement::If(self.parse_if()?)));
        }

        if self.match_token(&Token::For) {
            return self.parse_for_statement();
        }

        if self.match_token(&Token::While) {
            return Ok(CfmlNode::Statement(Statement::While(self.parse_while()?)));
        }

        if self.match_token(&Token::Do) {
            return Ok(CfmlNode::Statement(Statement::Do(self.parse_do()?)));
        }

        if self.match_token(&Token::Switch) {
            return Ok(CfmlNode::Statement(Statement::Switch(self.parse_switch()?)));
        }

        if self.match_token(&Token::Try) {
            return Ok(CfmlNode::Statement(Statement::Try(self.parse_try()?)));
        }

        if self.match_token(&Token::Throw) {
            // throw(...) with parens = function call form (VM-intercepted)
            if self.check(&Token::LParen) {
                self.consume(&Token::LParen)?;

                // Check if first arg is named (identifier followed by = or :)
                let is_named = matches!(self.peek(0), Token::Identifier(_))
                    && (matches!(self.peek(1), Token::Equal | Token::Colon));

                let arguments = if is_named {
                    // Parse named args like throw(message="oops", type="custom") or throw(message : "oops", type : "custom")
                    // Convert to positional: throw("oops", "custom", "", "")
                    let mut named: Vec<(String, Expression)> = Vec::new();
                    loop {
                        let key = self.extract_identifier()?.to_lowercase();
                        // Consume either = or :
                        if !self.match_token(&Token::Equal) {
                            self.consume(&Token::Colon)?;
                        }
                        let value = self.parse_expression()?;
                        named.push((key, value));
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                        // Check if next arg is also named
                        if !matches!(self.peek(0), Token::Identifier(_)) || !(matches!(self.peek(1), Token::Equal | Token::Colon)) {
                            break;
                        }
                    }
                    // Map to positional: message, type, detail, errorcode
                    let get_arg = |name: &str| -> Expression {
                        named.iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Expression::Literal(Literal {
                                value: LiteralValue::String(String::new()),
                                location: stmt_loc,
                            }))
                    };
                    vec![get_arg("message"), get_arg("type"), get_arg("detail"), get_arg("errorcode")]
                } else {
                    self.parse_arguments()?
                };

                self.consume(&Token::RParen)?;
                self.match_token(&Token::Semicolon);

                let throw_ident = Expression::Identifier(Identifier {
                    name: "throw".to_string(),
                    location: stmt_loc,
                });
                let call_expr = Expression::FunctionCall(Box::new(FunctionCall {
                    name: Box::new(throw_ident),
                    arguments,
                    location: stmt_loc,
                }));
                return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                    expr: call_expr,
                    location: stmt_loc,
                })));
            }
            return Ok(CfmlNode::Statement(Statement::Throw(self.parse_throw()?)));
        }

        if self.match_token(&Token::Rethrow) {
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Rethrow(stmt_loc)));
        }

        if self.match_token(&Token::Return) {
            return Ok(CfmlNode::Statement(Statement::Return(self.parse_return()?)));
        }

        if self.match_token(&Token::Break) {
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Break(Break {
                label: None,
                location: stmt_loc,
            })));
        }

        if self.match_token(&Token::Continue) {
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Continue(Continue {
                label: None,
                location: stmt_loc,
            })));
        }

        if self.match_token(&Token::Function) {
            return Ok(CfmlNode::Statement(Statement::FunctionDecl(FunctionDecl {
                func: self.parse_function()?,
            })));
        }

        // `component` is a SOFT keyword: it introduces a CFC only when it actually
        // begins a declaration (`component {`, `component Name ...`, `component
        // extends=...`, `component output="false" ...`). Used anywhere else — a
        // bare `component = x` assignment, `component.foo`, `component[1]`, or as
        // the `component` attribute of a script-statement cfinvoke — it is an
        // ordinary identifier and must fall through to expression parsing. This
        // matches Lucee/ACF/BoxLang, which all treat `component` as soft.
        if self.check(&Token::Component) && self.is_component_declaration() {
            self.advance(); // consume `component`
            return Ok(CfmlNode::Statement(Statement::ComponentDecl(
                ComponentDecl {
                    component: self.parse_component()?,
                },
            )));
        }

        if self.match_token(&Token::Interface) {
            return Ok(CfmlNode::Statement(Statement::InterfaceDecl(
                InterfaceDecl {
                    interface: self.parse_interface()?,
                },
            )));
        }

        // cfscript param statement: param name="varName" default="value";
        // or shorthand: param varName = defaultValue;
        if self.match_token(&Token::Param) {
            return self.parse_param_statement(stmt_loc);
        }

        // cfscript lock block: lock name="x" type="exclusive" timeout="5" { body }
        if self.match_token(&Token::Lock) {
            return self.parse_lock(stmt_loc);
        }

        if self.match_token(&Token::Include) {
            let path = self.parse_expression()?;
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Include(Include {
                path,
                location: stmt_loc,
            })));
        }

        if self.match_token(&Token::Import) {
            let path = self.extract_identifier()?;
            let alias = if self.match_token(&Token::Identifier("as".into())) {
                Some(self.extract_identifier()?)
            } else {
                None
            };
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Import(Import {
                path,
                alias,
                location: stmt_loc,
            })));
        }

        // Handle 'http url="..." method="..." result="..." { httpparam...; }' in CFScript
        if matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "http")
            && self.is_identifier_like_at(1)
            && matches!(self.peek(2), Token::Equal)
        {
            self.advance(); // consume 'http'

            // Parse attributes: key=value pairs
            let mut attrs: Vec<(String, Expression)> = Vec::new();
            while !self.check(&Token::LBrace) && !self.check(&Token::Semicolon) && !self.is_at_end() {
                if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                    let attr_name = self.extract_identifier()?;
                    self.advance(); // consume =
                    let attr_value = self.parse_expression()?;
                    attrs.push((attr_name.to_lowercase(), attr_value));
                } else {
                    break;
                }
            }

            // Parse httpparam statements if block body present.
            // KNOWN LIMITATION (issue #55): only literal `httpparam` statements are
            // collected here — control flow in the body (e.g. `for (x in coll) {
            // cfhttpparam(...); }`) is not supported and needs runtime param
            // collection, like cfquery got in 28af97d.
            let mut params: Vec<Expression> = Vec::new();
            if self.check(&Token::LBrace) {
                self.advance(); // consume {
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    // Expect httpparam statements
                    if matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "httpparam") {
                        self.advance(); // consume 'httpparam'
                        let mut param_pairs: Vec<(Expression, Expression)> = Vec::new();
                        while !self.check(&Token::Semicolon) && !self.check(&Token::RBrace) && !self.is_at_end() {
                            if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                                let pname = self.extract_identifier()?;
                                self.advance(); // consume =
                                let pvalue = self.parse_expression()?;
                                param_pairs.push((
                                    Expression::Literal(Literal {
                                        value: LiteralValue::String(pname.to_lowercase()),
                                        location: stmt_loc.clone(),
                                    }),
                                    pvalue,
                                ));
                            } else {
                                break;
                            }
                        }
                        self.match_token(&Token::Semicolon);
                        params.push(Expression::Struct(Struct {
                            pairs: param_pairs,
                            ordered: false,
                            location: stmt_loc.clone(),
                        }));
                    } else {
                        // Skip unknown tokens inside http block
                        self.advance();
                    }
                }
                self.consume(&Token::RBrace)?; // consume }
            } else {
                self.match_token(&Token::Semicolon);
            }

            // Build the struct argument for cfhttp({ url: ..., method: ..., params: [...] })
            let mut struct_pairs: Vec<(Expression, Expression)> = Vec::new();

            // Extract result var (default "cfhttp") and add remaining attrs
            let mut result_var = "cfhttp".to_string();
            for (name, value) in &attrs {
                if name == "result" {
                    // Extract the string value for the result variable name
                    if let Expression::Literal(ref lit) = value {
                        if let LiteralValue::String(ref s) = lit.value {
                            result_var = s.clone();
                        }
                    } else if let Expression::Identifier(ref id) = value {
                        result_var = id.name.clone();
                    }
                } else {
                    struct_pairs.push((
                        Expression::Literal(Literal {
                            value: LiteralValue::String(name.clone()),
                            location: stmt_loc.clone(),
                        }),
                        value.clone(),
                    ));
                }
            }

            // Add default method if not specified
            let has_method = attrs.iter().any(|(n, _)| n == "method");
            if !has_method {
                struct_pairs.push((
                    Expression::Literal(Literal {
                        value: LiteralValue::String("method".to_string()),
                        location: stmt_loc.clone(),
                    }),
                    Expression::Literal(Literal {
                        value: LiteralValue::String("GET".to_string()),
                        location: stmt_loc.clone(),
                    }),
                ));
            }

            // Add params array if any httpparam statements were found
            if !params.is_empty() {
                struct_pairs.push((
                    Expression::Literal(Literal {
                        value: LiteralValue::String("params".to_string()),
                        location: stmt_loc.clone(),
                    }),
                    Expression::Array(Array {
                        elements: params,
                        location: stmt_loc.clone(),
                    }),
                ));
            }

            // Build: result_var = cfhttp({ ... });
            let cfhttp_call = Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "cfhttp".to_string(),
                    location: stmt_loc.clone(),
                })),
                arguments: vec![Expression::Struct(Struct {
                    pairs: struct_pairs,
                    ordered: false,
                    location: stmt_loc.clone(),
                })],
                location: stmt_loc.clone(),
            }));

            return Ok(CfmlNode::Statement(Statement::Assignment(Assignment {
                target: AssignTarget::Variable(result_var),
                value: cfhttp_call,
                operator: AssignOp::Equal,
                location: stmt_loc,
            })));
        }

        // Handle 'cfinvoke'/'invoke' as a CFScript STATEMENT (the tag-in-script
        // form), e.g.
        //     cfinvoke component="Svc" method="m" returnvariable="r" arg=1;
        //     cfinvoke component=obj method="m" { invokeargument name="a" value=1; }
        // -> [<r> =] __cfinvoke(<component>, "<method>", <argStruct-or-collection>)
        // The function-CALL form `cfinvoke(component=...)` (peek(1) == LParen) is
        // NOT matched here and keeps parsing as an ordinary call. `component`,
        // `method`, etc. lex as keyword tokens, hence is_identifier_like_at.
        if matches!(self.peek(0), Token::Identifier(ref s)
                if s.eq_ignore_ascii_case("cfinvoke") || s.eq_ignore_ascii_case("invoke"))
            && self.is_identifier_like_at(1)
            && matches!(self.peek(2), Token::Equal)
        {
            self.advance(); // consume 'cfinvoke' / 'invoke'

            let mut component_expr: Option<Expression> = None;
            let mut method_expr: Option<Expression> = None;
            let mut return_var_expr: Option<Expression> = None;
            let mut arg_collection: Option<Expression> = None;
            let mut extra_pairs: Vec<(Expression, Expression)> = Vec::new();

            while !self.check(&Token::LBrace) && !self.check(&Token::Semicolon) && !self.is_at_end() {
                if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                    let key = self.extract_identifier()?.to_lowercase();
                    self.advance(); // consume =
                    let value = self.parse_expression()?;
                    match key.as_str() {
                        "component" => component_expr = Some(value),
                        "method" => method_expr = Some(value),
                        "returnvariable" => return_var_expr = Some(value),
                        "argumentcollection" => arg_collection = Some(value),
                        _ => extra_pairs.push((
                            Expression::Literal(Literal {
                                value: LiteralValue::String(key),
                                location: stmt_loc.clone(),
                            }),
                            value,
                        )),
                    }
                } else {
                    break;
                }
            }

            // Optional body of `invokeargument`/`cfinvokeargument name=.. value=..;`
            if self.check(&Token::LBrace) {
                self.advance(); // consume {
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    if matches!(self.peek(0), Token::Identifier(ref s)
                            if s.eq_ignore_ascii_case("invokeargument")
                                || s.eq_ignore_ascii_case("cfinvokeargument"))
                    {
                        self.advance(); // consume keyword
                        let mut arg_name: Option<Expression> = None;
                        let mut arg_value: Option<Expression> = None;
                        while !self.check(&Token::Semicolon) && !self.check(&Token::RBrace) && !self.is_at_end() {
                            if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                                let k = self.extract_identifier()?.to_lowercase();
                                self.advance(); // consume =
                                let v = self.parse_expression()?;
                                if k == "name" {
                                    arg_name = Some(v);
                                } else if k == "value" {
                                    arg_value = Some(v);
                                }
                            } else {
                                break;
                            }
                        }
                        self.match_token(&Token::Semicolon);
                        if let (Some(n), Some(v)) = (arg_name, arg_value) {
                            extra_pairs.push((n, v));
                        }
                    } else {
                        self.advance(); // skip unknown tokens inside the block
                    }
                }
                self.consume(&Token::RBrace)?;
            } else {
                self.match_token(&Token::Semicolon);
            }

            let str_lit = |s: &str| Expression::Literal(Literal {
                value: LiteralValue::String(s.to_string()),
                location: stmt_loc.clone(),
            });

            // Third arg: an explicit argumentcollection wins; otherwise a struct
            // of the remaining attributes + any invokeargument entries.
            let third_arg = arg_collection.unwrap_or_else(|| Expression::Struct(Struct {
                pairs: extra_pairs,
                ordered: false,
                location: stmt_loc.clone(),
            }));

            let call = Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "__cfinvoke".to_string(),
                    location: stmt_loc.clone(),
                })),
                arguments: vec![
                    component_expr.unwrap_or_else(|| str_lit("")),
                    method_expr.unwrap_or_else(|| str_lit("")),
                    third_arg,
                ],
                location: stmt_loc.clone(),
            }));

            // Bind the result if a returnVariable was given.
            match return_var_expr {
                None => {
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: call,
                        location: stmt_loc,
                    })));
                }
                Some(rv) => {
                    // A STATIC name -> a real lvalue, reusing the normal assignment
                    // path so `local.rv` / `variables.x` / dotted paths all work.
                    let static_name = match &rv {
                        Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => Some(s.clone()),
                        Expression::Identifier(id) => Some(id.name.clone()),
                        _ => None,
                    };
                    if let Some(name) = static_name {
                        if !name.is_empty() {
                            if let Some(lvalue) = self.returnvar_lvalue(&name) {
                                return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                                    expr: Expression::BinaryOp(Box::new(BinaryOp {
                                        left: Box::new(lvalue),
                                        operator: BinaryOpType::Assign,
                                        right: Box::new(call),
                                        location: stmt_loc.clone(),
                                    })),
                                    location: stmt_loc,
                                })));
                            }
                        }
                        // Empty/unusable static name: just invoke, drop the result.
                        return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                            expr: call,
                            location: stmt_loc,
                        })));
                    }
                    // A DYNAMIC returnVariable (e.g. "#arguments.rv#") -> assign by
                    // name at runtime. (setVariable resolves variables/request/
                    // session/application prefixes; other scopes go to variables.)
                    let set_call = Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "setVariable".to_string(),
                            location: stmt_loc.clone(),
                        })),
                        arguments: vec![rv, call],
                        location: stmt_loc.clone(),
                    }));
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: set_call,
                        location: stmt_loc,
                    })));
                }
            }
        }

        // Handle CFScript tag-as-statement syntax: keyword attr=value attr=value;
        // content type="..."; → __cfcontent({...})
        // header name="..." value="..."; → __cfheader({...})
        // location url="..."; → __cflocation({...})
        // setting requesttimeout="..."; → __cfsetting({...})
        // cookie name="..." value="..."; → __cfcookie({...})
        // log text="..." type="..."; → __cflog({...})
        {
            let tag_fn = if let Token::Identifier(ref s) = self.peek(0) {
                match s.to_lowercase().as_str() {
                    "content" => Some("__cfcontent"),
                    "header" => Some("__cfheader"),
                    "location" => Some("__cflocation"),
                    "setting" => Some("__cfsetting"),
                    "cookie" => Some("__cfcookie"),
                    "log" => Some("__cflog"),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(func_name) = tag_fn {
                if self.is_identifier_like_at(1) && matches!(self.peek(2), Token::Equal) {
                    self.advance(); // consume keyword
                    let mut struct_pairs: Vec<(Expression, Expression)> = Vec::new();
                    while !self.check(&Token::Semicolon) && !self.is_at_end() {
                        if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                            let attr_name = self.extract_identifier()?;
                            self.advance(); // consume =
                            let attr_value = self.parse_expression()?;
                            struct_pairs.push((
                                Expression::Literal(Literal {
                                    value: LiteralValue::String(attr_name.to_lowercase()),
                                    location: stmt_loc.clone(),
                                }),
                                attr_value,
                            ));
                        } else {
                            break;
                        }
                    }
                    self.match_token(&Token::Semicolon);
                    let call = Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: func_name.to_string(),
                            location: stmt_loc.clone(),
                        })),
                        arguments: vec![Expression::Struct(Struct {
                            pairs: struct_pairs,
                            ordered: false,
                            location: stmt_loc.clone(),
                        })],
                        location: stmt_loc.clone(),
                    }));
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: call,
                        location: stmt_loc,
                    })));
                }
            }
        }

        // Handle 'thread name="..." action="run" { body }' in CFScript
        if matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "thread")
            && self.is_identifier_like_at(1)
            && matches!(self.peek(2), Token::Equal)
        {
            self.advance(); // consume 'thread'
            let mut attrs: Vec<(String, Expression)> = Vec::new();
            while !self.check(&Token::LBrace) && !self.check(&Token::Semicolon) && !self.is_at_end() {
                if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                    let attr_name = self.extract_identifier()?;
                    self.advance(); // consume =
                    let attr_value = self.parse_expression()?;
                    attrs.push((attr_name.to_lowercase(), attr_value));
                } else {
                    break;
                }
            }

            // Extract name and action
            let thread_name = attrs.iter()
                .find(|(k, _)| k == "name")
                .map(|(_, v)| v.clone())
                .unwrap_or(Expression::Literal(Literal {
                    value: LiteralValue::String("thread1".to_string()),
                    location: stmt_loc.clone(),
                }));
            let action = attrs.iter()
                .find(|(k, _)| k == "action")
                .and_then(|(_, v)| if let Expression::Literal(ref lit) = v {
                    if let LiteralValue::String(ref s) = lit.value { Some(s.to_lowercase()) } else { None }
                } else { None })
                .unwrap_or_else(|| "run".to_string());

            match action.as_str() {
                "run" => {
                    // Parse body block
                    let body = self.parse_block()?;
                    // → __cfthread_run(name, function() { body })
                    let closure = Expression::Closure(Box::new(Closure {
                        params: vec![],
                        body,
                        location: stmt_loc.clone(),
                        metadata: Vec::new(),
                    }));
                    let call = Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfthread_run".to_string(),
                            location: stmt_loc.clone(),
                        })),
                        arguments: vec![thread_name, closure],
                        location: stmt_loc.clone(),
                    }));
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: call,
                        location: stmt_loc,
                    })));
                }
                "join" => {
                    self.match_token(&Token::Semicolon);
                    let timeout = attrs.iter()
                        .find(|(k, _)| k == "timeout")
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Expression::Literal(Literal {
                            value: LiteralValue::Int(0),
                            location: stmt_loc.clone(),
                        }));
                    let call = Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfthread_join".to_string(),
                            location: stmt_loc.clone(),
                        })),
                        arguments: vec![thread_name, timeout],
                        location: stmt_loc.clone(),
                    }));
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: call,
                        location: stmt_loc,
                    })));
                }
                "terminate" => {
                    self.match_token(&Token::Semicolon);
                    let call = Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfthread_terminate".to_string(),
                            location: stmt_loc.clone(),
                        })),
                        arguments: vec![thread_name],
                        location: stmt_loc.clone(),
                    }));
                    return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                        expr: call,
                        location: stmt_loc,
                    })));
                }
                _ => {
                    self.match_token(&Token::Semicolon);
                }
            }
        }

        // Handle 'savecontent variable="varname" { body }' in CFScript
        if matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "savecontent") {
            self.advance(); // consume 'savecontent'
            // Parse attributes: variable = "name"
            let mut var_name = "__savecontent_result".to_string();
            while !self.check(&Token::LBrace) && !self.is_at_end() {
                if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                    let attr_name = self.extract_identifier()?;
                    self.advance(); // consume =
                    let attr_value = self.parse_expression()?;
                    if attr_name.to_lowercase() == "variable" {
                        if let Expression::Literal(ref lit) = attr_value {
                            if let LiteralValue::String(ref s) = lit.value {
                                var_name = s.clone();
                            }
                        } else if let Expression::Identifier(ref id) = attr_value {
                            var_name = id.name.clone();
                        }
                    }
                } else {
                    break;
                }
            }
            // Parse body block
            let body = self.parse_block()?;
            // Convert to: __cfsavecontent_start(); body; varname = __cfsavecontent_end();
            let mut stmts = Vec::new();
            stmts.push(Statement::Expression(ExpressionStatement {
                expr: Expression::FunctionCall(Box::new(FunctionCall {
                    name: Box::new(Expression::Identifier(Identifier {
                        name: "__cfsavecontent_start".to_string(),
                        location: stmt_loc.clone(),
                    })),
                    arguments: vec![],
                    location: stmt_loc.clone(),
                })),
                location: stmt_loc.clone(),
            }));
            stmts.extend(body);
            stmts.push(Statement::Assignment(Assignment {
                target: AssignTarget::Variable(var_name),
                value: Expression::FunctionCall(Box::new(FunctionCall {
                    name: Box::new(Expression::Identifier(Identifier {
                        name: "__cfsavecontent_end".to_string(),
                        location: stmt_loc.clone(),
                    })),
                    arguments: vec![],
                    location: stmt_loc.clone(),
                })),
                operator: AssignOp::Equal,
                location: stmt_loc.clone(),
            }));
            return Ok(CfmlNode::Statement(Statement::Output(Output {
                body: stmts,
                location: stmt_loc,
            })));
        }

        // Handle script-call-with-body forms for body-block tags:
        //   cfsavecontent(variable="x") { body }
        //   cflock(name="x", type="exclusive", timeout=10) { body }
        //   cftransaction(...) { body }
        //   cfmail(to="...", from="...", subject="...") { body }
        // Plus the bare `transaction { body }` keyword form.
        // These lower to the same bytecode as the angle-bracket tag forms.
        if let Token::Identifier(ref nm) = self.peek(0).clone() {
            let nlow = nm.to_lowercase();
            let is_body_tag = matches!(nlow.as_str(),
                "cfsavecontent" | "cflock" | "cftransaction" | "cfmail");
            if is_body_tag && matches!(self.peek(1), Token::LParen) {
                let saved = self.current;
                self.advance(); // tag name
                self.advance(); // (
                let attrs = match self.parse_tag_call_attrs() {
                    Ok(a) => a,
                    Err(_) => { self.current = saved; Vec::new() }
                };
                if self.current != saved && self.check(&Token::RParen) {
                    self.advance(); // )
                    if self.check(&Token::LBrace) {
                        let body = self.parse_block()?;
                        return Ok(self.lower_script_body_tag(&nlow, attrs, body, stmt_loc));
                    } else {
                        self.current = saved;
                    }
                }
            }
            // `transaction { body }` (bare) or `transaction action="begin" { body }`
            // (with space-separated tag attributes). Lucee/Adobe CF/BoxLang accept
            // both. The attribute form mirrors the angle-bracket `<cftransaction
            // action="...">` tag and lowers to the same start/commit/rollback shape.
            if nlow == "transaction"
                && (matches!(self.peek(1), Token::LBrace)
                    || (self.is_identifier_like_at(1) && matches!(self.peek(2), Token::Equal)))
            {
                let saved = self.current;
                self.advance(); // consume 'transaction'
                let mut attrs: Vec<(String, Expression)> = Vec::new();
                while self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                    let key = self.extract_identifier()?;
                    self.advance(); // consume =
                    match self.parse_expression() {
                        Ok(v) => attrs.push((key, v)),
                        Err(_) => break,
                    }
                }
                if self.check(&Token::LBrace) {
                    let body = self.parse_block()?;
                    return Ok(self.lower_script_body_tag("cftransaction", attrs, body, stmt_loc));
                }
                // No `{` body (e.g. a bare `transaction action="commit";` statement
                // form we don't special-case) — restore and fall through.
                self.current = saved;
            }
        }

        // Handle 'abort' keyword as __cfabort() call
        if matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "abort") {
            self.advance(); // consume 'abort'
            self.match_token(&Token::Semicolon);
            // Build a function call expression to __cfabort()
            let abort_call = Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "__cfabort".to_string(),
                    location: stmt_loc.clone(),
                })),
                arguments: vec![],
                location: stmt_loc.clone(),
            }));
            return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                expr: abort_call,
                location: stmt_loc,
            })));
        }

        // Expression statement (may be assignment)
        let expr = self.parse_expression()?;

        // Check for compound assignment on expressions
        if let Some(assign_op) = self.check_assignment_op() {
            self.advance(); // consume the operator
            let value = self.parse_expression()?;
            self.match_token(&Token::Semicolon);

            let target = self.expression_to_assign_target(&expr)?;
            return Ok(CfmlNode::Statement(Statement::Assignment(Assignment {
                target,
                value,
                operator: assign_op,
                location: stmt_loc,
            })));
        }

        // Check for postfix ++ / --
        if self.match_token(&Token::PlusPlus) || self.match_token(&Token::MinusMinus) {
            let op = match self.previous().token {
                Token::PlusPlus => PostfixOpType::Increment,
                _ => PostfixOpType::Decrement,
            };
            self.match_token(&Token::Semicolon);
            return Ok(CfmlNode::Statement(Statement::Expression(
                ExpressionStatement {
                    expr: Expression::PostfixOp(Box::new(PostfixOp {
                        operand: Box::new(expr),
                        operator: op,
                        location: stmt_loc,
                    })),
                    location: stmt_loc,
                },
            )));
        }

        self.match_token(&Token::Semicolon);

        Ok(CfmlNode::Statement(Statement::Expression(
            ExpressionStatement {
                expr,
                location: stmt_loc,
            },
        )))
    }

    fn check_assignment_op(&self) -> Option<AssignOp> {
        match self.peek(0) {
            Token::PlusEqual => Some(AssignOp::PlusEqual),
            Token::MinusEqual => Some(AssignOp::MinusEqual),
            Token::StarEqual => Some(AssignOp::StarEqual),
            Token::SlashEqual => Some(AssignOp::SlashEqual),
            Token::AmpEqual => Some(AssignOp::ConcatEqual),
            Token::PercentEqual => Some(AssignOp::PercentEqual),
            _ => None,
        }
    }

    /// Map a compound-assignment operator token to the binary op it applies, for
    /// desugaring `lhs OP= rhs` into `lhs = (lhs OP rhs)` (used in the
    /// for-increment clause, which is parsed as an expression).
    fn compound_assign_binop(&self) -> Option<BinaryOpType> {
        match self.peek(0) {
            Token::PlusEqual => Some(BinaryOpType::Add),
            Token::MinusEqual => Some(BinaryOpType::Sub),
            Token::StarEqual => Some(BinaryOpType::Mul),
            Token::SlashEqual => Some(BinaryOpType::Div),
            Token::PercentEqual => Some(BinaryOpType::Mod),
            Token::AmpEqual => Some(BinaryOpType::Concat),
            _ => None,
        }
    }

    /// Turn a STATIC cfinvoke `returnVariable` name ("msg", "local.rv",
    /// "variables.x") into an assignable lvalue expression by sub-parsing it, so
    /// the normal assignment path handles scope prefixes and dotted paths. Returns
    /// None if the name doesn't parse to a simple lvalue (Identifier / member /
    /// index access), in which case the caller falls back to setVariable.
    fn returnvar_lvalue(&self, name: &str) -> Option<Expression> {
        let mut sub = Parser::new(format!("{};", name));
        let program = sub.parse().ok()?;
        let expr = program.statements.into_iter().find_map(|node| match node {
            CfmlNode::Statement(Statement::Expression(es)) => Some(es.expr),
            CfmlNode::Expression(e) => Some(e),
            _ => None,
        })?;
        match expr {
            Expression::Identifier(_)
            | Expression::MemberAccess(_)
            | Expression::ArrayAccess(_) => Some(expr),
            _ => None,
        }
    }

    fn expression_to_assign_target(&self, expr: &Expression) -> Result<AssignTarget, ParseError> {
        match expr {
            Expression::Identifier(id) => Ok(AssignTarget::Variable(id.name.clone())),
            Expression::ArrayAccess(acc) => Ok(AssignTarget::ArrayAccess(
                acc.array.clone(),
                acc.index.clone(),
            )),
            Expression::MemberAccess(acc) => {
                Ok(AssignTarget::StructAccess(acc.object.clone(), acc.member.clone()))
            }
            _ => Err(self.parse_error("Invalid assignment target")),
        }
    }

    fn parse_access_modifier(&mut self) -> AccessModifier {
        let tok = self.advance().token.clone();
        match tok {
            Token::Public => AccessModifier::Public,
            Token::Private => AccessModifier::Private,
            Token::Remote => AccessModifier::Remote,
            Token::Package => AccessModifier::Package,
            _ => AccessModifier::Public,
        }
    }

    fn parse_var(&mut self) -> Result<Var, ParseError> {
        let loc = self.current_location();
        let mut name = self.extract_identifier()?;
        // CFML allows dotted var declarations like: var local.x = 1
        while self.match_token(&Token::Dot) {
            let part = self.extract_identifier()?;
            name.push('.');
            name.push_str(&part);
        }
        let value = if self.match_token(&Token::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.match_token(&Token::Semicolon);

        Ok(Var {
            name,
            value,
            location: loc,
        })
    }

    fn parse_if(&mut self) -> Result<If, ParseError> {
        let loc = self.current_location();
        self.consume(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(&Token::RParen)?;

        let then_branch = if self.check(&Token::LBrace) {
            self.parse_block()?
        } else {
            // Single statement without braces
            let stmt = self.parse_statement()?;
            if let CfmlNode::Statement(s) = stmt {
                vec![s]
            } else {
                Vec::new()
            }
        };

        let mut else_if = Vec::new();
        let mut else_branch = None;

        // Handle else if / elseif chains
        while self.match_token(&Token::Else) {
            if self.match_token(&Token::If) || self.match_token(&Token::ElseIf) {
                // else if
                self.consume(&Token::LParen)?;
                let cond = self.parse_expression()?;
                self.consume(&Token::RParen)?;
                let body = if self.check(&Token::LBrace) {
                    self.parse_block()?
                } else {
                    let stmt = self.parse_statement()?;
                    if let CfmlNode::Statement(s) = stmt {
                        vec![s]
                    } else {
                        Vec::new()
                    }
                };
                else_if.push(ElseIf {
                    condition: cond,
                    body,
                });
            } else if self.match_token(&Token::ElseIf) {
                // elseif (single keyword)
                self.consume(&Token::LParen)?;
                let cond = self.parse_expression()?;
                self.consume(&Token::RParen)?;
                let body = if self.check(&Token::LBrace) {
                    self.parse_block()?
                } else {
                    let stmt = self.parse_statement()?;
                    if let CfmlNode::Statement(s) = stmt {
                        vec![s]
                    } else {
                        Vec::new()
                    }
                };
                else_if.push(ElseIf {
                    condition: cond,
                    body,
                });
            } else {
                // else
                else_branch = Some(if self.check(&Token::LBrace) {
                    self.parse_block()?
                } else {
                    let stmt = self.parse_statement()?;
                    if let CfmlNode::Statement(s) = stmt {
                        vec![s]
                    } else {
                        Vec::new()
                    }
                });
                break;
            }
        }

        // Handle standalone elseif (without else keyword prefix)
        while self.match_token(&Token::ElseIf) {
            self.consume(&Token::LParen)?;
            let cond = self.parse_expression()?;
            self.consume(&Token::RParen)?;
            let body = if self.check(&Token::LBrace) {
                self.parse_block()?
            } else {
                let stmt = self.parse_statement()?;
                if let CfmlNode::Statement(s) = stmt {
                    vec![s]
                } else {
                    Vec::new()
                }
            };
            else_if.push(ElseIf {
                condition: cond,
                body,
            });
        }

        Ok(If {
            condition,
            then_branch,
            else_if,
            else_branch,
            location: loc,
        })
    }

    fn parse_for_statement(&mut self) -> Result<CfmlNode, ParseError> {
        let loc = self.current_location();
        self.consume(&Token::LParen)?;

        // Check for for-in: for (var x in collection) or for (x in collection)
        let has_var = self.match_token(&Token::Var);

        // Lookahead to detect for-in: scan past a (possibly dotted) identifier to find 'in'
        {
            let mut la = 0;
            // First token must be an identifier, soft keyword, or `this`
            // (Wheels-style `for (this.x.y in arr)` writes through the
            // component instance — Lucee/ACF/BoxLang all accept it.)
            let is_ident_start =
                self.is_identifier_like_at(la) || matches!(self.peek(la), Token::This);
            if is_ident_start {
                la += 1;
                // Skip dotted parts: .ident .ident ... — keyword-like member
                // names (package, default, ...) are valid here, so reuse the
                // canonical identifier-like check.
                while matches!(self.peek(la), Token::Dot)
                    && self.is_identifier_like_at(la + 1)
                {
                    la += 2;
                }
                if matches!(self.peek(la), Token::In) {
                    // It's a for-in loop — consume the dotted name
                    let mut name = if matches!(self.peek(0), Token::This) {
                        self.advance();
                        "this".to_string()
                    } else {
                        self.extract_identifier()?
                    };
                    while self.match_token(&Token::Dot) {
                        let part = self.extract_identifier()?;
                        name.push('.');
                        name.push_str(&part);
                    }
                    self.advance(); // consume 'in'
                    let iterable = self.parse_expression()?;
                    self.consume(&Token::RParen)?;
                    let body = self.parse_block_or_statement()?;
                    return Ok(CfmlNode::Statement(Statement::ForIn(ForIn {
                        variable: name,
                        iterable,
                        body,
                        location: loc,
                    })));
                }
            }
        }

        // Standard C-style for loop: for (init; condition; increment)
        let init = if has_var {
            Some(Box::new(Statement::Var(self.parse_var_no_semicolon()?)))
        } else if !self.check(&Token::Semicolon) {
            let expr = self.parse_expression()?;
            // Check if it's an assignment
            if self.match_token(&Token::Equal) {
                let value = self.parse_expression()?;
                if let Expression::Identifier(ident) = &expr {
                    Some(Box::new(Statement::Var(Var {
                        name: ident.name.clone(),
                        value: Some(value),
                        location: self.current_location(),
                    })))
                } else {
                    Some(Box::new(Statement::Expression(ExpressionStatement {
                        expr,
                        location: self.current_location(),
                    })))
                }
            } else {
                Some(Box::new(Statement::Expression(ExpressionStatement {
                    expr,
                    location: self.current_location(),
                })))
            }
        } else {
            None
        };

        self.consume(&Token::Semicolon)?;

        let condition = if !self.check(&Token::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon)?;

        let increment = if !self.check(&Token::RParen) {
            let expr = self.parse_expression()?;
            // Support compound assignment in the increment clause (`i += 2`).
            // Statement-level compound assignment is handled in the statement
            // parser, but the for-increment is parsed as an expression, so
            // desugar `lhs OP= rhs` to `lhs = (lhs OP rhs)` here for assignable
            // targets — reusing the existing Assign binary op.
            let expr = if let Some(bin_op) = self.compound_assign_binop() {
                if matches!(
                    expr,
                    Expression::Identifier(_)
                        | Expression::MemberAccess(_)
                        | Expression::ArrayAccess(_)
                ) {
                    self.advance(); // consume the compound operator
                    let value = self.parse_expression()?;
                    let combined = Expression::BinaryOp(Box::new(BinaryOp {
                        left: Box::new(expr.clone()),
                        operator: bin_op,
                        right: Box::new(value),
                        location: self.current_location(),
                    }));
                    Expression::BinaryOp(Box::new(BinaryOp {
                        left: Box::new(expr),
                        operator: BinaryOpType::Assign,
                        right: Box::new(combined),
                        location: self.current_location(),
                    }))
                } else {
                    expr
                }
            } else {
                expr
            };
            Some(Box::new(expr))
        } else {
            None
        };

        self.consume(&Token::RParen)?;

        let body = if self.check(&Token::LBrace) {
            self.parse_block()?
        } else {
            let stmt = self.parse_statement()?;
            if let CfmlNode::Statement(s) = stmt {
                vec![s]
            } else {
                Vec::new()
            }
        };

        Ok(CfmlNode::Statement(Statement::For(For {
            init,
            condition,
            increment,
            body,
            location: loc,
        })))
    }

    fn parse_var_no_semicolon(&mut self) -> Result<Var, ParseError> {
        let loc = self.current_location();
        let mut name = self.extract_identifier()?;
        // CFML allows dotted var declarations like: var local.i = 1
        while self.match_token(&Token::Dot) {
            let part = self.extract_identifier()?;
            name.push('.');
            name.push_str(&part);
        }
        let value = if self.match_token(&Token::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Var {
            name,
            value,
            location: loc,
        })
    }

    fn parse_while(&mut self) -> Result<While, ParseError> {
        let loc = self.current_location();
        self.consume(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(&Token::RParen)?;

        let body = if self.check(&Token::LBrace) {
            self.parse_block()?
        } else {
            let stmt = self.parse_statement()?;
            if let CfmlNode::Statement(s) = stmt {
                vec![s]
            } else {
                Vec::new()
            }
        };

        Ok(While {
            condition,
            body,
            location: loc,
        })
    }

    fn parse_do(&mut self) -> Result<Do, ParseError> {
        let loc = self.current_location();
        let body = self.parse_block()?;
        self.consume(&Token::While)?;
        self.consume(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(&Token::RParen)?;
        self.match_token(&Token::Semicolon);

        Ok(Do {
            body,
            condition,
            location: loc,
        })
    }

    fn parse_switch(&mut self) -> Result<Switch, ParseError> {
        let loc = self.current_location();
        self.consume(&Token::LParen)?;
        let expression = self.parse_expression()?;
        self.consume(&Token::RParen)?;
        self.consume(&Token::LBrace)?;

        let mut cases = Vec::new();
        let mut default_case = None;

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.match_token(&Token::Case) {
                let mut values = vec![self.parse_expression()?];
                while self.match_token(&Token::Comma) {
                    values.push(self.parse_expression()?);
                }
                self.consume(&Token::Colon)?;

                let mut body = Vec::new();
                while !self.check(&Token::Case)
                    && !self.check(&Token::Default)
                    && !self.check(&Token::RBrace)
                    && !self.is_at_end()
                {
                    // A case body may be wrapped in braces (`case "x": { … }`).
                    // A bare `{` at statement position would otherwise be misread
                    // as a struct literal, so treat it as a block and flatten its
                    // statements (CFML blocks introduce no scope of their own).
                    if self.check(&Token::LBrace) {
                        body.extend(self.parse_block()?);
                    } else {
                        let node = self.parse_statement()?;
                        if let CfmlNode::Statement(s) = node {
                            body.push(s);
                        }
                    }
                }

                cases.push(SwitchCase { values, body });
            } else if self.match_token(&Token::Default) {
                self.consume(&Token::Colon)?;

                let mut body = Vec::new();
                while !self.check(&Token::Case)
                    && !self.check(&Token::RBrace)
                    && !self.is_at_end()
                {
                    // See the `case` body above: a braced default body would be
                    // misread as a struct literal, so flatten a leading block.
                    if self.check(&Token::LBrace) {
                        body.extend(self.parse_block()?);
                    } else {
                        let node = self.parse_statement()?;
                        if let CfmlNode::Statement(s) = node {
                            body.push(s);
                        }
                    }
                }

                default_case = Some(body);
            } else {
                self.advance(); // skip unknown token
            }
        }

        self.consume(&Token::RBrace)?;

        Ok(Switch {
            expression,
            cases,
            default_case,
            location: loc,
        })
    }

    fn parse_try(&mut self) -> Result<Try, ParseError> {
        let loc = self.current_location();
        let body = self.parse_block()?;
        let mut catches = Vec::new();
        let mut finally_body = None;

        while self.match_token(&Token::Catch) {
            self.consume(&Token::LParen)?;

            // catch (type varname) or catch (varname) or catch (any e)
            // The exception type may be a bare (optionally dotted) identifier
            // (`catch (FW1.AbortControllerException e)`) OR a quoted string literal
            // (`catch ("My.Custom.Type" e)`) — the idiomatic way to name a dotted,
            // namespaced custom exception. Lucee/Adobe CF/BoxLang accept both forms.
            let mut first = if let Token::String(s) = self.peek(0).clone() {
                self.advance();
                s
            } else {
                let mut id = self.extract_identifier()?;
                while self.check(&Token::Dot) && self.is_identifier_like_at(1) {
                    self.advance(); // consume dot
                    let part = self.extract_identifier()?;
                    id = format!("{}.{}", id, part);
                }
                id
            };

            let (var_type, var_name) = if self.check(&Token::RParen) {
                (None, first)
            } else {
                let name = self.extract_identifier()?;
                (Some(first), name)
            };

            self.consume(&Token::RParen)?;
            let catch_body = self.parse_block()?;

            catches.push(Catch {
                var_type,
                var_name,
                body: catch_body,
            });
        }

        if self.match_token(&Token::Finally) {
            finally_body = Some(self.parse_block()?);
        }

        Ok(Try {
            body,
            catches,
            finally_body,
            location: loc,
        })
    }

    /// Parse cfscript lock block: lock name="x" type="exclusive" timeout="5" { body }
    /// Desugars to: __cflock_start({name:"x", type:"exclusive", timeout:5}); try { body } finally { __cflock_end("x"); }
    /// Parse cfscript `param` statement:
    ///   param name="varName" default="value" type="string";
    ///   param varName = defaultValue;
    /// Converts to: if (!isDefined("varName")) varName = defaultValue;
    fn parse_param_statement(&mut self, loc: SourceLocation) -> Result<CfmlNode, ParseError> {
        // Check if it's the named-attribute form: param name="..." default="..."
        let is_named_form = matches!(self.peek(0), Token::Identifier(ref s) if s.to_lowercase() == "name")
            && matches!(self.peek(1), Token::Equal);

        if is_named_form {
            // Parse name=value attributes and emit __cfparam(name, default) call
            let mut name_expr: Option<Expression> = None;
            let mut default_expr: Option<Expression> = None;
            while (self.is_identifier_like() || matches!(self.peek(0), Token::Identifier(_)))
                && matches!(self.peek(1), Token::Equal) {
                let attr_name = self.extract_identifier()?.to_lowercase();
                self.advance(); // consume =
                let attr_value = self.parse_expression()?;
                match attr_name.as_str() {
                    "name" => name_expr = Some(attr_value),
                    "default" => default_expr = Some(attr_value),
                    _ => {} // ignore type, etc.
                }
            }
            self.match_token(&Token::Semicolon);

            let name_val = name_expr.unwrap_or(Expression::Literal(Literal {
                value: LiteralValue::String(String::new()),
                location: loc,
            }));

            // For simple string literal names, try to do compile-time expansion
            if let Expression::Literal(ref lit) = name_val {
                if let LiteralValue::String(ref var_name) = lit.value {
                    if !var_name.is_empty() {
                        let default_val = default_expr.unwrap_or(Expression::Literal(Literal {
                            value: LiteralValue::String(String::new()),
                            location: loc,
                        }));
                        let condition = Expression::FunctionCall(Box::new(FunctionCall {
                            name: Box::new(Expression::Identifier(Identifier {
                                name: "isDefined".to_string(),
                                location: loc,
                            })),
                            arguments: vec![Expression::Literal(Literal {
                                value: LiteralValue::String(var_name.clone()),
                                location: loc,
                            })],
                            location: loc,
                        }));

                        let assign_stmt = if let Some(dot_pos) = var_name.find('.') {
                            let root = var_name[..dot_pos].to_string();
                            let rest = &var_name[dot_pos + 1..];
                            let parts: Vec<&str> = rest.split('.').collect();
                            let mut expr = Expression::Identifier(Identifier {
                                name: root.clone(),
                                location: loc,
                            });
                            for (i, part) in parts.iter().enumerate() {
                                if i < parts.len() - 1 {
                                    expr = Expression::MemberAccess(Box::new(MemberAccess {
                                        object: Box::new(expr),
                                        member: part.to_string(),
                                        null_safe: false,
                                        location: loc,
                                    }));
                                }
                            }
                            let last_part = parts.last().unwrap().to_string();
                            Statement::Assignment(Assignment {
                                target: if parts.len() == 1 {
                                    AssignTarget::StructAccess(Box::new(Expression::Identifier(Identifier {
                                        name: root,
                                        location: loc,
                                    })), last_part)
                                } else {
                                    AssignTarget::StructAccess(Box::new(expr), last_part)
                                },
                                value: default_val,
                                operator: AssignOp::Equal,
                                location: loc,
                            })
                        } else {
                            Statement::Assignment(Assignment {
                                target: AssignTarget::Variable(var_name.clone()),
                                value: default_val,
                                operator: AssignOp::Equal,
                                location: loc,
                            })
                        };

                        return Ok(CfmlNode::Statement(Statement::If(If {
                            condition: Expression::UnaryOp(Box::new(UnaryOp {
                                operator: UnaryOpType::Not,
                                operand: Box::new(condition),
                                location: loc,
                            })),
                            then_branch: vec![assign_stmt],
                            else_if: vec![],
                            else_branch: None,
                            location: loc,
                        })));
                    }
                }
            }

            let default_val = default_expr.unwrap_or(Expression::Literal(Literal {
                value: LiteralValue::String(String::new()),
                location: loc,
            }));

            // Narrow lowering: recognise the common pattern
            //   param name="a.b.c['#expr#']" default=...;
            // and emit a structKeyExists guard + bracket assignment so the
            // mutation flows through normal codegen (which propagates Arc
            // mutations back to the caller's locals — same path as
            // `arguments.obj.foo = bar`). Falls through to the generic
            // __cfparam dispatch for anything more exotic.
            if let Expression::StringInterpolation(ref interp) = name_val {
                if let Some(stmt) =
                    try_lower_dynamic_param(interp, &default_val, loc)
                {
                    return Ok(CfmlNode::Statement(stmt));
                }
            }

            // Dynamic name (e.g., string interpolation) — emit __cfparam(nameExpr, defaultExpr)
            let call = Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "__cfparam".to_string(),
                    location: loc,
                })),
                arguments: vec![name_val, default_val],
                location: loc,
            }));
            return Ok(CfmlNode::Statement(Statement::Expression(ExpressionStatement {
                expr: call,
                location: loc,
            })));
        }

        // Shorthand form: param varName = defaultValue;
        // or: param type varName = defaultValue;
        let _type = if self.is_identifier_like() && !matches!(self.peek(1), Token::Equal | Token::Semicolon) {
            Some(self.extract_identifier()?)
        } else {
            None
        };
        let var_name = self.extract_identifier()?;
        let default_value = if self.match_token(&Token::Equal) {
            self.parse_expression()?
        } else {
            Expression::Literal(Literal {
                value: LiteralValue::String(String::new()),
                location: loc,
            })
        };
        self.match_token(&Token::Semicolon);

        let condition = Expression::FunctionCall(Box::new(FunctionCall {
            name: Box::new(Expression::Identifier(Identifier {
                name: "isDefined".to_string(),
                location: loc,
            })),
            arguments: vec![Expression::Literal(Literal {
                value: LiteralValue::String(var_name.clone()),
                location: loc,
            })],
            location: loc,
        }));

        Ok(CfmlNode::Statement(Statement::If(If {
            condition: Expression::UnaryOp(Box::new(UnaryOp {
                operator: UnaryOpType::Not,
                operand: Box::new(condition),
                location: loc,
            })),
            then_branch: vec![Statement::Assignment(Assignment {
                target: AssignTarget::Variable(var_name),
                value: default_value,
                operator: AssignOp::Equal,
                location: loc,
            })],
            else_if: vec![],
            else_branch: None,
            location: loc,
        })))
    }

    /// Parse a comma-separated `key = expr` list inside the parens of a
    /// script-call-with-body tag invocation, e.g. `variable="x", timeout=10`.
    /// Caller has already consumed the opening `(`; this stops at (but does
    /// not consume) the matching `)`.
    fn parse_tag_call_attrs(&mut self) -> Result<Vec<(String, Expression)>, ParseError> {
        let mut attrs = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            if !(self.is_identifier_like() && matches!(self.peek(1), Token::Equal)) {
                return Err(ParseError {
                    message: "expected key=value in tag call attributes".to_string(),
                    line: 0, column: 0,
                });
            }
            let key = self.extract_identifier()?;
            self.consume(&Token::Equal)?;
            let value = self.parse_expression()?;
            attrs.push((key, value));
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(attrs)
    }

    /// Lower a script-call-with-body form (`cfsavecontent(...) { ... }`,
    /// `cflock(...) { ... }`, `cftransaction(...) { ... }`, `cfmail(...) { ... }`)
    /// into the same AST shape the angle-bracket tag forms produce.
    fn lower_script_body_tag(
        &self,
        tag: &str,
        attrs: Vec<(String, Expression)>,
        body: Vec<Statement>,
        loc: SourceLocation,
    ) -> CfmlNode {
        match tag {
            "cfsavecontent" => {
                let var_name = attrs.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("variable"))
                    .and_then(|(_, v)| match v {
                        Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => Some(s.clone()),
                        Expression::Identifier(id) => Some(id.name.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "__savecontent_result".to_string());
                let mut stmts = vec![Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfsavecontent_start".to_string(),
                            location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    location: loc,
                })];
                stmts.extend(body);
                stmts.push(Statement::Assignment(Assignment {
                    target: AssignTarget::Variable(var_name),
                    value: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfsavecontent_end".to_string(),
                            location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    operator: AssignOp::Equal,
                    location: loc,
                }));
                CfmlNode::Statement(Statement::Output(Output { body: stmts, location: loc }))
            }
            "cflock" => {
                let attrs_struct = Expression::Struct(Struct {
                    pairs: attrs.iter().map(|(k, v)| (
                        Expression::Literal(Literal {
                            value: LiteralValue::String(k.clone()),
                            location: loc,
                        }),
                        v.clone(),
                    )).collect(),
                    ordered: false,
                    location: loc,
                });
                let lock_name_expr = attrs.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("name"))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Expression::Literal(Literal {
                        value: LiteralValue::String("default".to_string()),
                        location: loc,
                    }));
                let lock_start = Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cflock_start".to_string(), location: loc,
                        })),
                        arguments: vec![attrs_struct],
                        location: loc,
                    })),
                    location: loc,
                });
                let lock_end = Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cflock_end".to_string(), location: loc,
                        })),
                        arguments: vec![lock_name_expr],
                        location: loc,
                    })),
                    location: loc,
                });
                let try_stmt = Statement::Try(Try {
                    body,
                    catches: vec![],
                    finally_body: Some(vec![lock_end]),
                    location: loc,
                });
                CfmlNode::Statement(Statement::Output(Output {
                    body: vec![lock_start, try_stmt],
                    location: loc,
                }))
            }
            "cftransaction" => {
                // __cftransaction_start("begin"); try { body; __cftransaction_commit(); }
                // catch (any e) { __cftransaction_rollback(); throw e; }
                let action_expr = attrs.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("action"))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Expression::Literal(Literal {
                        value: LiteralValue::String("begin".to_string()),
                        location: loc,
                    }));
                let start = Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cftransaction_start".to_string(), location: loc,
                        })),
                        arguments: vec![action_expr],
                        location: loc,
                    })),
                    location: loc,
                });
                let commit = Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cftransaction_commit".to_string(), location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    location: loc,
                });
                let rollback = Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cftransaction_rollback".to_string(), location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    location: loc,
                });
                let rethrow = Statement::Throw(Throw {
                    message: Some(Expression::Identifier(Identifier {
                        name: "__txn_e".to_string(), location: loc,
                    })),
                    type_: None,
                    location: loc,
                });
                let mut try_body = body;
                try_body.push(commit);
                let try_stmt = Statement::Try(Try {
                    body: try_body,
                    catches: vec![Catch {
                        var_type: Some("any".to_string()),
                        var_name: "__txn_e".to_string(),
                        body: vec![rollback, rethrow],
                    }],
                    finally_body: None,
                    location: loc,
                });
                CfmlNode::Statement(Statement::Output(Output {
                    body: vec![start, try_stmt],
                    location: loc,
                }))
            }
            "cfmail" => {
                // Capture the body text via savecontent, then call __cfmail({...attrs, body: captured}).
                let capture_var = "__cfmail_body_capture".to_string();
                let mut stmts: Vec<Statement> = Vec::new();
                stmts.push(Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfsavecontent_start".to_string(), location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    location: loc,
                }));
                stmts.extend(body);
                stmts.push(Statement::Assignment(Assignment {
                    target: AssignTarget::Variable(capture_var.clone()),
                    value: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfsavecontent_end".to_string(), location: loc,
                        })),
                        arguments: vec![],
                        location: loc,
                    })),
                    operator: AssignOp::Equal,
                    location: loc,
                }));
                // Build {attrs..., body: __cfmail_body_capture}
                let mut pairs: Vec<(Expression, Expression)> = attrs.iter().map(|(k, v)| (
                    Expression::Literal(Literal {
                        value: LiteralValue::String(k.clone()),
                        location: loc,
                    }),
                    v.clone(),
                )).collect();
                pairs.push((
                    Expression::Literal(Literal {
                        value: LiteralValue::String("body".to_string()),
                        location: loc,
                    }),
                    Expression::Identifier(Identifier {
                        name: capture_var, location: loc,
                    }),
                ));
                let opts_struct = Expression::Struct(Struct {
                    pairs, ordered: false, location: loc,
                });
                stmts.push(Statement::Expression(ExpressionStatement {
                    expr: Expression::FunctionCall(Box::new(FunctionCall {
                        name: Box::new(Expression::Identifier(Identifier {
                            name: "__cfmail".to_string(), location: loc,
                        })),
                        arguments: vec![opts_struct],
                        location: loc,
                    })),
                    location: loc,
                }));
                CfmlNode::Statement(Statement::Output(Output { body: stmts, location: loc }))
            }
            _ => unreachable!("unknown body tag: {}", tag),
        }
    }

    fn parse_lock(&mut self, loc: SourceLocation) -> Result<CfmlNode, ParseError> {
        // Parse key=value attributes before the block
        let mut attrs: Vec<(String, Expression)> = Vec::new();
        while let Token::Identifier(_) = self.peek(0) {
            if matches!(self.peek(1), Token::Equal) {
                let key = self.extract_identifier()?;
                self.consume(&Token::Equal)?;
                let value = self.parse_expression()?;
                attrs.push((key, value));
            } else {
                break;
            }
        }

        // Parse the block body
        let body = self.parse_block()?;

        // Extract lock name for __cflock_end
        let lock_name_expr = attrs.iter()
            .find(|(k, _)| k.to_lowercase() == "name")
            .map(|(_, v)| v.clone())
            .unwrap_or(Expression::Literal(Literal {
                value: LiteralValue::String("default".to_string()),
                location: loc,
            }));

        // Build struct literal for __cflock_start argument
        let struct_pairs: Vec<(Expression, Expression)> = attrs.iter().map(|(k, v)| {
            (Expression::Literal(Literal {
                value: LiteralValue::String(k.clone()),
                location: loc,
            }), v.clone())
        }).collect();

        let attrs_struct = Expression::Struct(Struct {
            pairs: struct_pairs,
            ordered: false,
            location: loc,
        });

        // __cflock_start(attrs)
        let lock_start = Statement::Expression(ExpressionStatement {
            expr: Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "__cflock_start".to_string(),
                    location: loc,
                })),
                arguments: vec![attrs_struct],
                location: loc,
            })),
            location: loc,
        });

        // __cflock_end(name)
        let lock_end = Statement::Expression(ExpressionStatement {
            expr: Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "__cflock_end".to_string(),
                    location: loc,
                })),
                arguments: vec![lock_name_expr],
                location: loc,
            })),
            location: loc,
        });

        // try { body } finally { __cflock_end(name) }
        let try_stmt = Statement::Try(Try {
            body,
            catches: vec![],
            finally_body: Some(vec![lock_end]),
            location: loc,
        });

        // Wrap as Output block: __cflock_start; try { ... } finally { __cflock_end }
        let output = Statement::Output(Output {
            body: vec![lock_start, try_stmt],
            location: loc,
        });

        Ok(CfmlNode::Statement(output))
    }

    fn parse_throw(&mut self) -> Result<Throw, ParseError> {
        let loc = self.current_location();
        let message = if !self.check(&Token::Semicolon) && !self.is_at_end() {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.match_token(&Token::Semicolon);

        Ok(Throw {
            message,
            type_: None,
            location: loc,
        })
    }

    fn parse_return(&mut self) -> Result<Return, ParseError> {
        let loc = self.current_location();
        let value = if !self.check(&Token::Semicolon)
            && !self.check(&Token::RBrace)
            && !self.is_at_end()
        {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.match_token(&Token::Semicolon);

        Ok(Return {
            value,
            location: loc,
        })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let loc = self.current_location();
        // Optional return type before function name
        let mut return_type = None;
        let name;

        // Parse a (possibly dotted) run. A function name is always immediately
        // followed by `(`, so if `(` does not follow the first run, that run was
        // a return type and the real name is the next run. Names accept soft
        // keywords and other keyword identifiers (e.g. a method named `new`);
        // in tag-based CFCs they may also be dotted (`upload.profile_id`).
        let first = self.extract_function_name()?;
        let mut dotted = first;
        while self.match_token(&Token::Dot) {
            dotted.push('.');
            dotted.push_str(&self.extract_function_name()?);
        }

        if self.check(&Token::LParen) {
            name = dotted;
        } else {
            return_type = Some(dotted);
            let mut n = self.extract_function_name()?;
            while self.match_token(&Token::Dot) {
                n.push('.');
                n.push_str(&self.extract_function_name()?);
            }
            name = n;
        }

        self.consume(&Token::LParen)?;
        let params = self.parse_param_list()?;
        self.consume(&Token::RParen)?;

        // Parse function metadata attributes (e.g., httpmethod="GET" restpath="/users",
        // output="false", hint="..."). Accepts both identifiers and keyword tokens as keys.
        let mut metadata = Vec::new();
        loop {
            let is_attr_key = matches!(self.peek(1), Token::Equal)
                && (matches!(self.peek(0), Token::Identifier(_))
                    || self.token_as_string(&self.peek(0).clone()).is_some());
            if !is_attr_key {
                break;
            }
            let key = if let Token::Identifier(ref s) = self.peek(0) {
                let s = s.clone();
                self.advance();
                s
            } else if let Some(s) = self.token_as_string(&self.peek(0).clone()) {
                self.advance();
                s
            } else {
                break;
            };
            self.consume(&Token::Equal)?;
            // Attribute values may be quoted OR unquoted (e.g. `output=true`),
            // matching component/interface headers.
            let val = match self.parse_decl_attr_value() {
                Some(v) => v,
                None => break,
            };
            metadata.push((key, val));
        }

        let body = if self.check(&Token::LBrace) {
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Function {
            name,
            params,
            return_type,
            access: AccessModifier::Public,
            is_static: false,
            is_abstract: false,
            body,
            location: loc,
            metadata,
        })
    }

    fn parse_component(&mut self) -> Result<Component, ParseError> {
        let loc = self.current_location();
        // Parse component name: can be `component Name` or `component name="Name"`
        // Only consume an identifier as the name if it's NOT followed by '=' (which
        // would indicate a metadata attribute like output="false" or hint="...").
        let mut name = if matches!(self.peek(0), Token::Identifier(_))
            && !matches!(self.peek(1), Token::Equal)
            && !matches!(self.peek(0), Token::Extends | Token::Implements)
        {
            self.extract_identifier().unwrap_or_else(|_| "Anonymous".to_string())
        } else {
            "Anonymous".to_string()
        };

        // Check for name="..." attribute before other metadata
        if name == "Anonymous" && matches!(self.peek(0), Token::Identifier(_))
            && self.peek(0).to_string() == "name"
            && matches!(self.peek(1), Token::Equal)
        {
            self.advance(); // consume "name"
            self.consume(&Token::Equal)?;
            if let Token::String(val) = self.peek(0).clone() {
                self.advance();
                name = val.clone();
            }
        }

        let mut extends = None;
        let mut implements = Vec::new();

        if self.match_token(&Token::Extends) {
            // Handle both `extends Animal` and `extends="Animal"` syntax
            if self.match_token(&Token::Equal) {
                if let Token::String(val) = self.peek(0).clone() {
                    self.advance();
                    extends = Some(val);
                }
            } else {
                extends = self.extract_dotted_identifier().ok();
            }
        }

        if self.match_token(&Token::Implements) {
            // Handle both `implements IFoo` and `implements="IFoo"` syntax
            if self.match_token(&Token::Equal) {
                if let Token::String(val) = self.peek(0).clone() {
                    self.advance();
                    // May be comma-separated: "IFoo,IBar"
                    for iface in val.split(',') {
                        let trimmed = iface.trim().to_string();
                        if !trimmed.is_empty() {
                            implements.push(trimmed);
                        }
                    }
                }
            } else {
                loop {
                    if let Ok(iface) = self.extract_dotted_identifier() {
                        implements.push(iface);
                    }
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }
        }

        // Parse component metadata attributes (e.g., taffy_uri="/users/{id}", output="false", hint="...", accessors="true")
        // Accepts both identifiers and keyword tokens as attribute keys, plus
        // namespaced colon-separated keys like `taffy:uri` (lex'd as three
        // tokens: ident, `:`, ident).
        let mut metadata = Vec::new();
        loop {
            // Determine whether the upcoming tokens form a metadata attribute.
            // Patterns we recognise as a key:
            //   <ident>            =
            //   <ident> : <ident>  =       (namespaced, e.g. taffy:uri)
            // `extends`/`implements` are accepted here too: component header
            // attributes are order-independent on Lucee/ACF/BoxLang, so they may
            // appear AFTER another attribute (e.g. `component output="false"
            // extends="Foo" {`) and still populate the dedicated fields.
            let head_is_keyish =
                matches!(self.peek(0), Token::Identifier(_) | Token::Extends | Token::Implements)
                    || self.token_as_string(&self.peek(0).clone()).is_some();
            if !head_is_keyish {
                break;
            }
            let is_simple = matches!(self.peek(1), Token::Equal);
            let is_namespaced = matches!(self.peek(1), Token::Colon)
                && (matches!(self.peek(2), Token::Identifier(_))
                    || self.token_as_string(&self.peek(2).clone()).is_some())
                && matches!(self.peek(3), Token::Equal);
            if !is_simple && !is_namespaced {
                break;
            }
            let mut key = match self.peek(0).clone() {
                Token::Identifier(s) => {
                    self.advance();
                    s
                }
                Token::Extends => {
                    self.advance();
                    "extends".to_string()
                }
                Token::Implements => {
                    self.advance();
                    "implements".to_string()
                }
                ref t => {
                    if let Some(s) = self.token_as_string(t) {
                        self.advance();
                        s
                    } else {
                        break;
                    }
                }
            };
            if is_namespaced {
                self.advance(); // consume ':'
                let suffix = if let Token::Identifier(ref s) = self.peek(0) {
                    let s = s.clone();
                    self.advance();
                    s
                } else if let Some(s) = self.token_as_string(&self.peek(0).clone()) {
                    self.advance();
                    s
                } else {
                    break;
                };
                key.push('_');
                key.push_str(&suffix);
            }
            self.consume(&Token::Equal)?;
            // The value may be quoted OR unquoted (e.g. `output=false`).
            let val = match self.parse_decl_attr_value() {
                Some(v) => v,
                None => break,
            };
            if key.eq_ignore_ascii_case("extends") {
                // Only fill from the attribute form if a leading `extends` (handled
                // above) didn't already set it.
                if extends.is_none() {
                    extends = Some(val);
                }
            } else if key.eq_ignore_ascii_case("implements") {
                for iface in val.split(',') {
                    let trimmed = iface.trim().to_string();
                    if !trimmed.is_empty() {
                        implements.push(trimmed);
                    }
                }
            } else {
                // If this is the name attribute and we have an Anonymous component, use it as the name
                if key.eq_ignore_ascii_case("name") && name == "Anonymous" {
                    name = val.clone();
                }
                metadata.push((key, val));
            }
        }

        self.consume(&Token::LBrace)?;

        let mut properties = Vec::new();
        let mut functions = Vec::new();
        let mut body = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Check for access modifiers — only consume if followed by function/property/static
            let access = if matches!(
                self.peek(0),
                Token::Public | Token::Private | Token::Remote | Token::Package
            ) && self.is_access_modifier_for_member()
            {
                self.parse_access_modifier()
            } else {
                AccessModifier::Public
            };

            let is_static = self.match_token(&Token::Static);

            // Skip optional return type annotation (e.g. "array function ...")
            if matches!(self.peek(0), Token::Identifier(_)) && matches!(self.peek(1), Token::Function) {
                self.advance(); // skip return type
            }

            if self.match_token(&Token::Property) {
                properties.push(self.parse_property()?);
            } else if self.match_token(&Token::Function) {
                let mut func = self.parse_function()?;
                func.access = access;
                func.is_static = is_static;
                functions.push(func);
            } else if self.match_token(&Token::Var) {
                body.push(Statement::Var(self.parse_var()?));
            } else {
                let node = self.parse_statement()?;
                if let CfmlNode::Statement(s) = node {
                    body.push(s);
                }
            }
        }

        self.consume(&Token::RBrace)?;

        // Check for accessors="true" attribute
        let accessors = metadata.iter()
            .any(|(key, value)| key.to_lowercase() == "accessors" && value.to_lowercase() == "true");

        Ok(Component {
            name,
            extends,
            implements,
            properties,
            functions,
            body,
            location: loc,
            metadata,
            accessors,
        })
    }

    fn parse_interface(&mut self) -> Result<Interface, ParseError> {
        let loc = self.current_location();
        // Optional name (same logic as component — skip if followed by '=')
        let name = if matches!(self.peek(0), Token::Identifier(_))
            && !matches!(self.peek(1), Token::Equal)
            && !matches!(self.peek(0), Token::Extends)
        {
            self.extract_identifier().unwrap_or_else(|_| "Anonymous".to_string())
        } else {
            "Anonymous".to_string()
        };

        // interfaces can extend multiple other interfaces, in either the
        // bareword form (`interface extends Base, Other`) or the attribute form
        // (`interface extends="Base"` / `extends="A,B"`), matching Lucee/ACF.
        let mut extends = Vec::new();
        if self.match_token(&Token::Extends) {
            if self.match_token(&Token::Equal) {
                if let Some(val) = self.parse_decl_attr_value() {
                    for parent in val.split(',') {
                        let trimmed = parent.trim().to_string();
                        if !trimmed.is_empty() {
                            extends.push(trimmed);
                        }
                    }
                }
            } else {
                loop {
                    if let Ok(parent) = self.extract_dotted_identifier() {
                        extends.push(parent);
                    }
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }
        }

        // Parse metadata attributes (same order-independent rules as component:
        // values may be quoted or unquoted, and `extends` may appear here too —
        // e.g. `interface displayname="x" extends="Base" {`).
        let mut metadata = Vec::new();
        loop {
            let head_is_keyish = matches!(self.peek(0), Token::Identifier(_) | Token::Extends)
                || self.token_as_string(&self.peek(0).clone()).is_some();
            if !matches!(self.peek(1), Token::Equal) || !head_is_keyish {
                break;
            }
            let key = match self.peek(0).clone() {
                Token::Identifier(s) => {
                    self.advance();
                    s
                }
                Token::Extends => {
                    self.advance();
                    "extends".to_string()
                }
                ref t => {
                    if let Some(s) = self.token_as_string(t) {
                        self.advance();
                        s
                    } else {
                        break;
                    }
                }
            };
            self.consume(&Token::Equal)?;
            let val = match self.parse_decl_attr_value() {
                Some(v) => v,
                None => break,
            };
            if key.eq_ignore_ascii_case("extends") {
                for parent in val.split(',') {
                    let trimmed = parent.trim().to_string();
                    if !trimmed.is_empty() {
                        extends.push(trimmed);
                    }
                }
            } else {
                metadata.push((key, val));
            }
        }

        self.consume(&Token::LBrace)?;

        let mut functions = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Consume optional semicolons between signatures
            if self.match_token(&Token::Semicolon) {
                continue;
            }

            // Parse access modifier — only if followed by function declaration
            let access = if matches!(
                self.peek(0),
                Token::Public | Token::Private | Token::Remote | Token::Package
            ) && self.is_access_modifier_for_function()
            {
                self.parse_access_modifier()
            } else {
                AccessModifier::Public
            };

            // Skip optional return type annotation
            if matches!(self.peek(0), Token::Identifier(_)) && matches!(self.peek(1), Token::Function) {
                self.advance();
            }

            if self.match_token(&Token::Function) {
                let mut func = self.parse_function()?;
                func.access = access;
                functions.push(func);
            } else {
                // Skip unexpected tokens
                self.advance();
            }
        }

        self.consume(&Token::RBrace)?;

        Ok(Interface {
            name,
            extends,
            functions,
            metadata,
            location: loc,
        })
    }

    fn parse_property(&mut self) -> Result<Property, ParseError> {
        let loc = self.current_location();

        // Detect key-value syntax: property name="x" [type="y"] [inject="z"] ...;
        // Key-value syntax is detected when an identifier is followed by = and a string
        let is_kv = {
            let has_ident = matches!(self.peek(0), Token::Identifier(_))
                || self.token_as_string(&self.peek(0).clone()).is_some();
            has_ident && matches!(self.peek(1), Token::Equal) && matches!(self.peek(2), Token::String(_))
        };

        if is_kv {
            return self.parse_property_kv(loc);
        }

        // Positional syntax: property [required] [type] name [= default];
        let mut prop_type = None;
        let mut required = false;

        if self.match_token(&Token::Required) {
            required = true;
        }

        let first = self
            .extract_identifier()
            .unwrap_or_else(|_| "unknown".to_string());

        let name = if let Token::Identifier(_) = self.peek(0) {
            prop_type = Some(first);
            self.extract_identifier()
                .unwrap_or_else(|_| "unknown".to_string())
        } else {
            first
        };

        let default = if self.match_token(&Token::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.match_token(&Token::Semicolon);

        Ok(Property {
            name,
            prop_type,
            default,
            required,
            attributes: Vec::new(),
            location: loc,
        })
    }

    /// Parse key-value property syntax: property name="x" type="string" inject="Service" default="val";
    fn parse_property_kv(&mut self, loc: SourceLocation) -> Result<Property, ParseError> {
        let mut name = String::new();
        let mut prop_type = None;
        let mut default = None;
        let mut required = false;
        let mut attributes = Vec::new();

        loop {
            // Check for key="value" or key=value pattern — key can be an identifier or a keyword token
            // For name and type, we require string values; for default, we accept any expression
            let key_str = if let Token::Identifier(ref s) = self.peek(0) {
                if matches!(self.peek(1), Token::Equal) {
                    Some(s.clone())
                } else {
                    None
                }
            } else if let Some(s) = self.token_as_string(&self.peek(0).clone()) {
                if matches!(self.peek(1), Token::Equal) {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            };

            let key = match key_str {
                Some(k) => k,
                None => break,
            };

            self.advance(); // consume key
            self.advance(); // consume =

            // For "default", parse as an expression (can be number, string, boolean, etc.)
            // For other keys, expect a string value
            if key.to_lowercase() == "default" {
                // Parse the default value as an expression
                let expr = self.parse_expression()?;
                default = Some(expr);
            } else {
                // For name, type, required, etc., expect a string
                let val = if let Token::String(v) = self.peek(0).clone() {
                    self.advance();
                    v
                } else {
                    break;
                };

                match key.to_lowercase().as_str() {
                    "name" => name = val,
                    "type" => prop_type = Some(val),
                    "required" => required = val.eq_ignore_ascii_case("true"),
                    _ => attributes.push((key.to_lowercase(), val)),
                }
            }
        }

        self.match_token(&Token::Semicolon);

        Ok(Property {
            name,
            prop_type,
            default,
            required,
            attributes,
            location: loc,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        if self.check(&Token::RParen) {
            return Ok(params);
        }

        loop {
            let required = self.match_token(&Token::Required);
            let mut param_type = None;

            // The leading token(s) are EITHER the parameter name, OR a type
            // annotation followed by the name. A type may be a dotted FQN
            // (`wheels.system.TestResult`), so parse it as a dotted identifier;
            // the name itself is always a single identifier.
            let first = self
                .extract_dotted_identifier()
                .unwrap_or_else(|_| "arg".to_string());

            // If next is also an identifier (or soft keyword usable as identifier),
            // then first was the type annotation and next is the param name.
            let name = if self.is_identifier_like() {
                param_type = Some(first);
                self.extract_identifier()
                    .unwrap_or_else(|_| "arg".to_string())
            } else {
                first
            };

            let default = if self.match_token(&Token::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            // Consume and discard optional per-parameter attributes (e.g. `hint="..."`).
            // These are accepted for source compatibility but not stored on Param.
            loop {
                let is_attr_key = matches!(self.peek(1), Token::Equal)
                    && (matches!(self.peek(0), Token::Identifier(_))
                        || self.token_as_string(&self.peek(0).clone()).is_some());
                if !is_attr_key {
                    break;
                }
                self.advance(); // key
                self.consume(&Token::Equal)?;
                if matches!(self.peek(0), Token::String(_)) {
                    self.advance();
                } else {
                    // Tolerate bare identifier / number / etc. as attribute value
                    let _ = self.parse_expression();
                }
            }

            params.push(Param {
                name,
                param_type,
                default,
                required,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.consume(&Token::LBrace)?;
        let mut statements = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let node = self.parse_statement()?;
            if let CfmlNode::Statement(s) = node {
                statements.push(s);
            }
        }

        self.consume(&Token::RBrace)?;
        Ok(statements)
    }

    /// Parse either a braced block or a single statement (CFML allows braceless for/if/while bodies)
    fn parse_block_or_statement(&mut self) -> Result<Vec<Statement>, ParseError> {
        if self.check(&Token::LBrace) {
            self.parse_block()
        } else {
            let node = self.parse_statement()?;
            if let CfmlNode::Statement(s) = node {
                Ok(vec![s])
            } else {
                Ok(Vec::new())
            }
        }
    }

    /// Check whether an access modifier (public/private/remote/package) at peek(0)
    /// is used as a function declaration prefix vs. a plain identifier.
    /// Returns true if the modifier is followed by: function, static, or a return-type + function.
    fn is_access_modifier_for_function(&self) -> bool {
        // peek(0) is the access modifier token itself; check what follows at peek(1)+
        let mut la = 1;
        // Skip optional "static"
        if matches!(self.peek(la), Token::Static) {
            la += 1;
        }
        // Direct "function" after modifier (or static)
        if matches!(self.peek(la), Token::Function) {
            return true;
        }
        // Return-type annotation: Identifier(.Identifier)* followed by "function"
        if matches!(self.peek(la), Token::Identifier(_)) {
            la += 1;
            while matches!(self.peek(la), Token::Dot) && matches!(self.peek(la + 1), Token::Identifier(_)) {
                la += 2;
            }
            if matches!(self.peek(la), Token::Function) {
                return true;
            }
        }
        false
    }

    /// Check whether an access modifier at peek(0) precedes a component member
    /// (function, property, static) vs. being used as a plain identifier.
    fn is_access_modifier_for_member(&self) -> bool {
        let mut la = 1;
        // Skip optional "static"
        if matches!(self.peek(la), Token::Static) {
            la += 1;
        }
        // Direct function/property after modifier
        if matches!(self.peek(la), Token::Function | Token::Property) {
            return true;
        }
        // Return-type annotation: Identifier(.Identifier)* followed by "function"
        if matches!(self.peek(la), Token::Identifier(_)) {
            la += 1;
            while matches!(self.peek(la), Token::Dot) && matches!(self.peek(la + 1), Token::Identifier(_)) {
                la += 2;
            }
            if matches!(self.peek(la), Token::Function) {
                return true;
            }
        }
        false
    }

    /// Check if the next token can be used as an identifier (true Identifier or soft keyword).
    fn is_identifier_like(&self) -> bool {
        self.is_identifier_like_at(0)
    }

    /// Decide whether a `component` token at the current position begins a CFC
    /// declaration (vs. being used as an ordinary identifier). A declaration is
    /// `component`, optionally followed by a name and/or metadata attributes,
    /// then a `{` body. The token immediately after `component` is decisive:
    ///   - `{`                       -> anonymous component body
    ///   - `extends` / `implements`  -> inheritance clause
    ///   - an identifier-like token  -> the CFC name, or a metadata key such as
    ///                                  `output=` / `displayname=` / `hint=`
    ///                                  (note `output` and friends lex as keyword
    ///                                  tokens, hence is_identifier_like_at, not a
    ///                                  bare Identifier check).
    /// Anything else (`=`, `.`, `[`, `(`, `;`, an operator, EOF) means `component`
    /// is an identifier — e.g. `component = "x"` or a cfinvoke `component="..."`
    /// attribute — so it is NOT a declaration and falls through to expressions.
    fn is_component_declaration(&self) -> bool {
        self.is_identifier_like_at(1)
            || matches!(self.peek(1), Token::LBrace | Token::Extends | Token::Implements)
    }

    /// Check if the token at offset can be used as an identifier.
    fn is_identifier_like_at(&self, offset: usize) -> bool {
        matches!(self.peek(offset),
            Token::Identifier(_) | Token::Local | Token::Param | Token::Output
            | Token::Required | Token::Default | Token::Include | Token::Import
            | Token::Property | Token::Abstract | Token::Final | Token::Static | Token::Lock
            | Token::Function | Token::Var | Token::Throw | Token::Component
            | Token::Interface | Token::Package | Token::Remote
            | Token::Public | Token::Private | Token::Extends | Token::Implements
        )
    }

    fn extract_identifier(&mut self) -> Result<String, ParseError> {
        match self.peek(0) {
            Token::Identifier(_) => {
                if let Token::Identifier(id) = self.advance().token {
                    Ok(id)
                } else {
                    unreachable!()
                }
            }
            // CFML soft keywords — can be used as identifiers in most contexts
            Token::Local => { self.advance(); Ok("local".to_string()) }
            Token::Param => { self.advance(); Ok("param".to_string()) }
            Token::Output => { self.advance(); Ok("output".to_string()) }
            Token::Required => { self.advance(); Ok("required".to_string()) }
            Token::Default => { self.advance(); Ok("default".to_string()) }
            Token::Include => { self.advance(); Ok("include".to_string()) }
            Token::Import => { self.advance(); Ok("import".to_string()) }
            Token::Property => { self.advance(); Ok("property".to_string()) }
            Token::Abstract => { self.advance(); Ok("abstract".to_string()) }
            Token::Final => { self.advance(); Ok("final".to_string()) }
            Token::Static => { self.advance(); Ok("static".to_string()) }
            Token::Lock => { self.advance(); Ok("lock".to_string()) }
            Token::Function => { self.advance(); Ok("function".to_string()) }
            Token::Var => { self.advance(); Ok("var".to_string()) }
            Token::Throw => { self.advance(); Ok("throw".to_string()) }
            Token::Component => { self.advance(); Ok("component".to_string()) }
            Token::Interface => { self.advance(); Ok("interface".to_string()) }
            Token::Package => { self.advance(); Ok("package".to_string()) }
            Token::Remote => { self.advance(); Ok("remote".to_string()) }
            Token::Public => { self.advance(); Ok("public".to_string()) }
            Token::Private => { self.advance(); Ok("private".to_string()) }
            // `extends` / `implements` are declaration keywords but, like the
            // other soft keywords above, are legal ordinary identifiers (e.g.
            // function parameter names) on Lucee/Adobe CF/BoxLang. Component and
            // interface headers match these tokens explicitly before reaching
            // here, so accepting them as identifiers does not shadow inheritance.
            Token::Extends => { self.advance(); Ok("extends".to_string()) }
            Token::Implements => { self.advance(); Ok("implements".to_string()) }
            _ => Err(self.parse_error("Expected identifier")),
        }
    }

    /// Extract a function/method name. Accepts identifiers, soft keywords, and
    /// other keywords that are legal member names in CFML (e.g. a method named
    /// `new`), mirroring property-name rules.
    fn extract_function_name(&mut self) -> Result<String, ParseError> {
        self.extract_property_name()
    }

    /// Extract a property name after a dot — any keyword or identifier is valid in CFML.
    fn extract_property_name(&mut self) -> Result<String, ParseError> {
        // First try normal identifier extraction (handles identifiers + soft keywords)
        if let Ok(name) = self.extract_identifier() {
            return Ok(name);
        }
        // After a dot, any keyword can be used as a property name in CFML
        let name = match self.peek(0) {
            Token::If => "if", Token::Else => "else", Token::ElseIf => "elseif",
            Token::For => "for", Token::In => "in", Token::While => "while",
            Token::Do => "do", Token::Break => "break", Token::Continue => "continue",
            Token::Return => "return", Token::Switch => "switch", Token::Case => "case",
            Token::Try => "try", Token::Catch => "catch", Token::Finally => "finally",
            Token::Throw => "throw", Token::Rethrow => "rethrow", Token::Function => "function", Token::Var => "var",
            Token::New => "new", Token::This => "this", Token::Super => "super",
            Token::Component => "component", Token::Extends => "extends",
            Token::Implements => "implements", Token::Interface => "interface",
            Token::Public => "public", Token::Private => "private",
            Token::Remote => "remote", Token::Package => "package",
            Token::True => "true", Token::False => "false", Token::Null => "null",
            Token::Contains => "contains", Token::NotKeyword => "not",
            Token::AndKeyword => "and", Token::OrKeyword => "or",
            Token::EqKeyword => "eq", Token::NeqKeyword => "neq",
            Token::GtKeyword => "gt", Token::GteKeyword => "gte",
            Token::LtKeyword => "lt", Token::LteKeyword => "lte",
            Token::ModKeyword => "mod", Token::IsKeyword => "is",
            _ => return Err(self.parse_error("Expected property name")),
        };
        self.advance();
        Ok(name.to_string())
    }

    /// Extract a component/interface declaration attribute VALUE. CFML accepts
    /// the value quoted OR unquoted: a string literal, a boolean/number keyword,
    /// or a bare (optionally dotted) identifier are all legal value positions on
    /// Lucee/Adobe CF/BoxLang. Returns the value rendered as a string, or None if
    /// the next token cannot begin a value.
    fn parse_decl_attr_value(&mut self) -> Option<String> {
        match self.peek(0).clone() {
            Token::String(s) => {
                self.advance();
                Some(s)
            }
            Token::True => {
                self.advance();
                Some("true".to_string())
            }
            Token::False => {
                self.advance();
                Some("false".to_string())
            }
            Token::Integer(n) => {
                self.advance();
                Some(n.to_string())
            }
            Token::Double(d) => {
                self.advance();
                Some(d.to_string())
            }
            Token::Identifier(_) => self.extract_dotted_identifier().ok(),
            _ => None,
        }
    }

    /// Convert a keyword token to its string representation for use as metadata keys.
    fn token_as_string(&self, token: &Token) -> Option<String> {
        match token {
            Token::Output => Some("output".to_string()),
            Token::Public => Some("public".to_string()),
            Token::Private => Some("private".to_string()),
            Token::Remote => Some("remote".to_string()),
            Token::Package => Some("package".to_string()),
            Token::Static => Some("static".to_string()),
            Token::Abstract => Some("abstract".to_string()),
            Token::Final => Some("final".to_string()),
            Token::Required => Some("required".to_string()),
            Token::Default => Some("default".to_string()),
            Token::Lock => Some("lock".to_string()),
            _ => None,
        }
    }

    fn extract_dotted_identifier(&mut self) -> Result<String, ParseError> {
        let mut path = self.extract_identifier()?;
        while self.match_token(&Token::Dot) {
            let next = self.extract_property_name()?;
            path.push('.');
            path.push_str(&next);
        }
        Ok(path)
    }

    fn consume(&mut self, token: &Token) -> Result<(), ParseError> {
        if self.check(token) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!("Expected {:?}, found {:?}", token, self.peek(0)),
                line: self.current_location().start.line,
                column: self.current_location().start.column,
            })
        }
    }

    // ---- Expression Parsing (Pratt-style precedence climbing) ----

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_ternary()?;

        if self.check(&Token::Equal) {
            if let Expression::Identifier(ref ident) = expr {
                let name = ident.name.clone();
                self.advance(); // consume =
                let value = self.parse_assignment_rhs()?;
                return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(Expression::Identifier(Identifier {
                        name,
                        location: self.current_location(),
                    })),
                    operator: BinaryOpType::Assign,
                    right: Box::new(value),
                    location: self.current_location(),
                })));
            } else if let Expression::MemberAccess(_) | Expression::ArrayAccess(_) = &expr {
                self.advance(); // consume =
                let value = self.parse_assignment_rhs()?;
                return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(expr),
                    operator: BinaryOpType::Assign,
                    right: Box::new(value),
                    location: self.current_location(),
                })));
            }
        }

        Ok(expr)
    }

    /// Parse the right-hand side of an assignment. Unlike `parse_assignment_expr`,
    /// this also accepts a *compound* assignment operator (`&=`, `+=`, …) so a
    /// chained assignment like `local.sql = local.sql &= ";"` parses as
    /// `local.sql = (local.sql = local.sql & ";")`. Top-level statement
    /// compound-assignment is intentionally NOT routed here: the statement parser
    /// handles `lhs OP= rhs` directly (check_assignment_op) to keep the
    /// single-evaluation semantics of struct/array targets in codegen
    /// (Statement::Assignment / emit_load_current_target).
    fn parse_assignment_rhs(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_ternary()?;

        if self.check(&Token::Equal) {
            if let Expression::Identifier(_) | Expression::MemberAccess(_) | Expression::ArrayAccess(_) = &expr {
                self.advance(); // consume =
                let value = self.parse_assignment_rhs()?;
                return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(expr),
                    operator: BinaryOpType::Assign,
                    right: Box::new(value),
                    location: self.current_location(),
                })));
            }
        } else if let Some(bin_op) = self.compound_assign_binop() {
            if matches!(
                expr,
                Expression::Identifier(_)
                    | Expression::MemberAccess(_)
                    | Expression::ArrayAccess(_)
            ) {
                self.advance(); // consume the compound operator
                let value = self.parse_assignment_rhs()?;
                let combined = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: bin_op,
                    right: Box::new(value),
                    location: self.current_location(),
                }));
                return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(expr),
                    operator: BinaryOpType::Assign,
                    right: Box::new(combined),
                    location: self.current_location(),
                })));
            }
        }

        Ok(expr)
    }

    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_imp()?;

        if self.match_token(&Token::Question) {
            let then_expr = Box::new(self.parse_expression()?);
            self.consume(&Token::Colon)?;
            let else_expr = Box::new(self.parse_expression()?);

            return Ok(Expression::Ternary(Box::new(Ternary {
                condition: Box::new(expr),
                then_expr,
                else_expr,
                location: self.current_location(),
            })));
        }

        // Elvis operator ?: (null coalescing) and ?? (null coalescing alias)
        if self.match_token(&Token::QuestionColon) || self.match_token(&Token::QuestionQuestion) {
            let right = Box::new(self.parse_expression()?);
            return Ok(Expression::Elvis(Box::new(Elvis {
                left: Box::new(expr),
                right,
                location: self.current_location(),
            })));
        }

        Ok(expr)
    }

    fn parse_imp(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_eqv()?;

        while self.match_token(&Token::ImpKeyword) {
            let right = Box::new(self.parse_eqv()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Imp,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_eqv(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_xor()?;

        while self.match_token(&Token::EqvKeyword) {
            let right = Box::new(self.parse_xor()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Eqv,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_or()?;

        while self.match_token(&Token::XorKeyword) {
            let right = Box::new(self.parse_or()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Xor,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;

        while self.match_token(&Token::BarBar) || self.match_token(&Token::OrKeyword) {
            let right = Box::new(self.parse_and()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Or,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_not()?;

        while self.match_token(&Token::AmpAmp) || self.match_token(&Token::AndKeyword) {
            let right = Box::new(self.parse_not()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::And,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expression, ParseError> {
        if self.match_token(&Token::NotKeyword) || self.match_token(&Token::Bang) {
            let operand = Box::new(self.parse_not()?);
            return Ok(Expression::UnaryOp(Box::new(UnaryOp {
                operator: UnaryOpType::Not,
                operand,
                location: self.current_location(),
            })));
        }

        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            // `EQUAL` / `EQUALS` (verbose alias for EQ) and `==`/`EQ`.
            if self.match_token(&Token::EqualEqual)
                || self.match_token(&Token::EqKeyword)
                || self.match_word("equal")
                || self.match_word("equals")
            {
                let right = Box::new(self.parse_comparison()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::Equal,
                    right,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::IsKeyword) {
                // `IS NOT` is the verbose alias for NEQ; bare `IS` is EQ.
                let operator = if self.match_token(&Token::NotKeyword) {
                    BinaryOpType::NotEqual
                } else {
                    BinaryOpType::Equal
                };
                let right = Box::new(self.parse_comparison()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator,
                    right,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::BangEqual) || self.match_token(&Token::NeqKeyword) {
                let right = Box::new(self.parse_comparison()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::NotEqual,
                    right,
                    location: self.current_location(),
                }));
            } else if matches!(self.peek(0), Token::NotKeyword)
                && (self.peek_word(1, "equal") || self.peek_word(1, "equals"))
            {
                // `NOT EQUAL` / `NOT EQUALS` — verbose alias for NEQ. Only consume
                // the `NOT` once the following word confirms the operator, so a
                // leading unary `NOT` elsewhere is untouched.
                self.advance(); // NOT
                self.advance(); // EQUAL / EQUALS
                let right = Box::new(self.parse_comparison()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::NotEqual,
                    right,
                    location: self.current_location(),
                }));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_contains()?;

        loop {
            if self.match_token(&Token::Greater) || self.match_token(&Token::GtKeyword) {
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::Greater,
                    right,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::GreaterEqual) || self.match_token(&Token::GteKeyword) {
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::GreaterEqual,
                    right,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::Less) || self.match_token(&Token::LtKeyword) {
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::Less,
                    right,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::LessEqual) || self.match_token(&Token::LteKeyword) {
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::LessEqual,
                    right,
                    location: self.current_location(),
                }));
            } else if self.peek_word(0, "greater") && self.peek_word(1, "than") {
                // `GREATER THAN` (GT) and `GREATER THAN OR EQUAL TO` (GTE).
                self.advance(); // greater
                self.advance(); // than
                let operator = self.match_or_equal_to_suffix(
                    BinaryOpType::Greater,
                    BinaryOpType::GreaterEqual,
                );
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator,
                    right,
                    location: self.current_location(),
                }));
            } else if self.peek_word(0, "less") && self.peek_word(1, "than") {
                // `LESS THAN` (LT) and `LESS THAN OR EQUAL TO` (LTE).
                self.advance(); // less
                self.advance(); // than
                let operator = self.match_or_equal_to_suffix(
                    BinaryOpType::Less,
                    BinaryOpType::LessEqual,
                );
                let right = Box::new(self.parse_contains()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator,
                    right,
                    location: self.current_location(),
                }));
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// After a `GREATER THAN` / `LESS THAN`, consume an optional `OR EQUAL TO`
    /// suffix. Returns `with_suffix` if it was present, else `base`. (`OR` lexes
    /// as the logical-or keyword; `equal`/`to` are plain identifiers.)
    fn match_or_equal_to_suffix(
        &mut self,
        base: BinaryOpType,
        with_suffix: BinaryOpType,
    ) -> BinaryOpType {
        if matches!(self.peek(0), Token::OrKeyword)
            && self.peek_word(1, "equal")
            && self.peek_word(2, "to")
        {
            self.advance(); // or
            self.advance(); // equal
            self.advance(); // to
            with_suffix
        } else {
            base
        }
    }

    fn parse_contains(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_concatenation()?;

        if self.match_token(&Token::Contains) {
            let right = Box::new(self.parse_concatenation()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Contains,
                right,
                location: self.current_location(),
            }));
        } else if self.peek_word(0, "does")
            && matches!(self.peek(1), Token::NotKeyword)
            && (self.peek_word(2, "contain")
                || self.peek_word(2, "contains")
                || matches!(self.peek(2), Token::Contains))
        {
            // `DOES NOT CONTAIN` — verbose negated CONTAINS. The positive form is
            // the `CONTAINS` keyword, but the negated form's `CONTAIN` (singular)
            // lexes as a plain identifier.
            self.advance(); // does
            self.advance(); // not
            self.advance(); // contain(s)
            let right = Box::new(self.parse_concatenation()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::DoesNotContain,
                right,
                location: self.current_location(),
            }));
        } else if self.match_token(&Token::NotKeyword) {
            // "NOT CONTAINS" as two-word operator
            if self.match_token(&Token::Contains) {
                let right = Box::new(self.parse_concatenation()?);
                left = Expression::BinaryOp(Box::new(BinaryOp {
                    left: Box::new(left),
                    operator: BinaryOpType::DoesNotContain,
                    right,
                    location: self.current_location(),
                }));
            } else {
                // It was just NOT used as unary, put it back
                self.current -= 1;
            }
        }

        Ok(left)
    }

    fn parse_concatenation(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_term()?;

        while self.match_token(&Token::Amp) {
            let right = Box::new(self.parse_term()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Concat,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_factor()?;

        while self.match_token(&Token::Plus) || self.match_token(&Token::Minus) {
            let operator = match self.previous().token {
                Token::Plus => BinaryOpType::Add,
                _ => BinaryOpType::Sub,
            };
            let right = Box::new(self.parse_factor()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_power()?;

        while self.match_token(&Token::Star)
            || self.match_token(&Token::Slash)
            || self.match_token(&Token::Percent)
            || self.match_token(&Token::ModKeyword)
            || self.match_token(&Token::Backslash)
        {
            let operator = match self.previous().token {
                Token::Star => BinaryOpType::Mul,
                Token::Slash => BinaryOpType::Div,
                Token::Backslash => BinaryOpType::IntDiv,
                _ => BinaryOpType::Mod,
            };
            let right = Box::new(self.parse_power()?);
            left = Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator,
                right,
                location: self.current_location(),
            }));
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_unary()?;

        if self.match_token(&Token::Caret) {
            let right = Box::new(self.parse_unary()?);
            return Ok(Expression::BinaryOp(Box::new(BinaryOp {
                left: Box::new(left),
                operator: BinaryOpType::Pow,
                right,
                location: self.current_location(),
            })));
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if self.match_token(&Token::Minus) {
            let operand = Box::new(self.parse_unary()?);
            return Ok(Expression::UnaryOp(Box::new(UnaryOp {
                operator: UnaryOpType::Minus,
                operand,
                location: self.current_location(),
            })));
        }

        // Prefix ++ / --
        if self.match_token(&Token::PlusPlus) {
            let operand = Box::new(self.parse_call()?);
            return Ok(Expression::UnaryOp(Box::new(UnaryOp {
                operator: UnaryOpType::PrefixIncrement,
                operand,
                location: self.current_location(),
            })));
        }
        if self.match_token(&Token::MinusMinus) {
            let operand = Box::new(self.parse_call()?);
            return Ok(Expression::UnaryOp(Box::new(UnaryOp {
                operator: UnaryOpType::PrefixDecrement,
                operand,
                location: self.current_location(),
            })));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_call()?;

        // Postfix ++ / --
        if self.match_token(&Token::PlusPlus) {
            expr = Expression::PostfixOp(Box::new(PostfixOp {
                operand: Box::new(expr),
                operator: PostfixOpType::Increment,
                location: self.current_location(),
            }));
        } else if self.match_token(&Token::MinusMinus) {
            expr = Expression::PostfixOp(Box::new(PostfixOp {
                operand: Box::new(expr),
                operator: PostfixOpType::Decrement,
                location: self.current_location(),
            }));
        }

        Ok(expr)
    }

    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&Token::Dot) {
                let method = self.extract_property_name().unwrap_or_default();
                if self.match_token(&Token::LParen) {
                    let args = self.parse_arguments()?;
                    self.consume(&Token::RParen)?;
                    expr = Expression::MethodCall(Box::new(MethodCall {
                        object: Box::new(expr),
                        method,
                        arguments: args,
                        null_safe: false,
                        location: self.current_location(),
                    }));
                } else {
                    expr = Expression::MemberAccess(Box::new(MemberAccess {
                        object: Box::new(expr),
                        member: method,
                        null_safe: false,
                        location: self.current_location(),
                    }));
                }
            } else if self.match_token(&Token::LParen) {
                let args = self.parse_arguments()?;
                self.consume(&Token::RParen)?;
                expr = Expression::FunctionCall(Box::new(FunctionCall {
                    name: Box::new(expr),
                    arguments: args,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::LBracket) {
                let index = Box::new(self.parse_expression()?);
                self.consume(&Token::RBracket)?;
                expr = Expression::ArrayAccess(Box::new(ArrayAccess {
                    array: Box::new(expr),
                    index,
                    location: self.current_location(),
                }));
            } else if self.match_token(&Token::QuestionDot) {
                // Null-safe navigation: obj?.method() or obj?.property
                let member = self.extract_property_name().unwrap_or_default();
                if self.match_token(&Token::LParen) {
                    let args = self.parse_arguments()?;
                    self.consume(&Token::RParen)?;
                    expr = Expression::MethodCall(Box::new(MethodCall {
                        object: Box::new(expr),
                        method: member,
                        arguments: args,
                        null_safe: true,
                        location: self.current_location(),
                    }));
                } else {
                    expr = Expression::MemberAccess(Box::new(MemberAccess {
                        object: Box::new(expr),
                        member,
                        null_safe: true,
                        location: self.current_location(),
                    }));
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();

        if self.check(&Token::RParen) {
            return Ok(args);
        }

        loop {
            if self.match_token(&Token::DotDotDot) {
                let expr = self.parse_expression()?;
                args.push(Expression::Spread(Box::new(expr)));
            } else {
                // Check for named argument: identifier = value or identifier : value
                // CFML supports foo(name = value, name2 = value2) and foo(name : value)
                // We must detect this before parse_expression consumes `=` as assignment.
                let is_named_arg = (matches!(self.peek(0), Token::Identifier(_)) || self.is_identifier_like())
                    && (matches!(self.peek(1), Token::Equal | Token::Colon));
                if is_named_arg {
                    let name = self.extract_identifier()?;
                    // Consume either = or :
                    if !self.match_token(&Token::Equal) {
                        self.consume(&Token::Colon)?;
                    }
                    let value = self.parse_expression()?;
                    // Encode named arg as a struct entry: argumentCollection-style
                    // Use a NamedArgument expression node or encode as key:value
                    args.push(Expression::NamedArgument(Box::new(NamedArgument {
                        name,
                        value: Box::new(value),
                        location: self.current_location(),
                    })));
                } else {
                    args.push(self.parse_expression()?);
                }
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance().token.clone();

        match token {
            Token::True => Ok(Expression::Literal(Literal {
                value: LiteralValue::Bool(true),
                location: self.current_location(),
            })),
            Token::False => Ok(Expression::Literal(Literal {
                value: LiteralValue::Bool(false),
                location: self.current_location(),
            })),
            Token::Null => Ok(Expression::Literal(Literal {
                value: LiteralValue::Null,
                location: self.current_location(),
            })),
            Token::Integer(i) => Ok(Expression::Literal(Literal {
                value: LiteralValue::Int(i),
                location: self.current_location(),
            })),
            Token::Double(d) => Ok(Expression::Literal(Literal {
                value: LiteralValue::Double(d),
                location: self.current_location(),
            })),
            Token::String(s) => Ok(Expression::Literal(Literal {
                value: LiteralValue::String(s),
                location: self.current_location(),
            })),
            Token::InterpolatedStringStart => {
                let mut parts: Vec<Expression> = Vec::new();
                while !self.is_at_end() && !self.check(&Token::InterpolatedStringEnd) {
                    let part_token = self.advance().token.clone();
                    match part_token {
                        Token::String(s) => {
                            parts.push(Expression::Literal(Literal {
                                value: LiteralValue::String(s),
                                location: self.current_location(),
                            }));
                        }
                        Token::InterpolatedExpr(expr_str) => {
                            // Add semicolon so sub-parser can parse as a statement
                            let mut sub_parser = Parser::new(format!("{};", expr_str));
                            if let Ok(program) = sub_parser.parse() {
                                let expr = program.statements.into_iter().next().and_then(|node| {
                                    match node {
                                        CfmlNode::Statement(Statement::Expression(es)) => Some(es.expr),
                                        CfmlNode::Expression(expr) => Some(expr),
                                        _ => None,
                                    }
                                });
                                parts.push(expr.unwrap_or(Expression::Empty));
                            } else {
                                // Fallback: treat as identifier
                                parts.push(Expression::Identifier(Identifier {
                                    name: expr_str.trim().to_string(),
                                    location: self.current_location(),
                                }));
                            }
                        }
                        _ => break,
                    }
                }
                self.match_token(&Token::InterpolatedStringEnd);
                Ok(Expression::StringInterpolation(StringInterpolation {
                    parts,
                    location: self.current_location(),
                }))
            }
            Token::Identifier(id) => Ok(Expression::Identifier(Identifier {
                name: id,
                location: self.current_location(),
            })),
            // CFML soft keywords used as variables in expressions
            Token::Component => Ok(Expression::Identifier(Identifier {
                name: "component".to_string(),
                location: self.current_location(),
            })),
            Token::Local => Ok(Expression::Identifier(Identifier {
                name: "local".to_string(),
                location: self.current_location(),
            })),
            Token::Param => Ok(Expression::Identifier(Identifier {
                name: "param".to_string(),
                location: self.current_location(),
            })),
            Token::Output => Ok(Expression::Identifier(Identifier {
                name: "output".to_string(),
                location: self.current_location(),
            })),
            Token::Required => Ok(Expression::Identifier(Identifier {
                name: "required".to_string(),
                location: self.current_location(),
            })),
            Token::Default => Ok(Expression::Identifier(Identifier {
                name: "default".to_string(),
                location: self.current_location(),
            })),
            Token::Include => Ok(Expression::Identifier(Identifier {
                name: "include".to_string(),
                location: self.current_location(),
            })),
            Token::Import => Ok(Expression::Identifier(Identifier {
                name: "import".to_string(),
                location: self.current_location(),
            })),
            Token::Property => Ok(Expression::Identifier(Identifier {
                name: "property".to_string(),
                location: self.current_location(),
            })),
            Token::Abstract => Ok(Expression::Identifier(Identifier {
                name: "abstract".to_string(),
                location: self.current_location(),
            })),
            Token::Final => Ok(Expression::Identifier(Identifier {
                name: "final".to_string(),
                location: self.current_location(),
            })),
            Token::Static => Ok(Expression::Identifier(Identifier {
                name: "static".to_string(),
                location: self.current_location(),
            })),
            Token::Lock => Ok(Expression::Identifier(Identifier {
                name: "lock".to_string(),
                location: self.current_location(),
            })),
            Token::Private => Ok(Expression::Identifier(Identifier {
                name: "private".to_string(),
                location: self.current_location(),
            })),
            Token::Public => Ok(Expression::Identifier(Identifier {
                name: "public".to_string(),
                location: self.current_location(),
            })),
            Token::Remote => Ok(Expression::Identifier(Identifier {
                name: "remote".to_string(),
                location: self.current_location(),
            })),
            Token::Extends => Ok(Expression::Identifier(Identifier {
                name: "extends".to_string(),
                location: self.current_location(),
            })),
            Token::Implements => Ok(Expression::Identifier(Identifier {
                name: "implements".to_string(),
                location: self.current_location(),
            })),
            Token::This => Ok(Expression::This(This {
                location: self.current_location(),
            })),
            Token::Super => Ok(Expression::Super(Super {
                location: self.current_location(),
            })),
            Token::New => {
                // After `new`, collect a dotted class name: Ident(.Ident)*
                // e.g. `new framework.one()`, `new com.myapp.Service()`
                // Then parse arguments in parens.
                if let Token::Identifier(_) = self.peek(0) {
                    let mut name_parts = Vec::new();
                    if let Token::Identifier(first) = self.advance().token.clone() {
                        name_parts.push(first);
                    }
                    while self.check(&Token::Dot) {
                        if let Token::Identifier(_) = self.peek(1) {
                            self.advance(); // consume dot
                            if let Token::Identifier(part) = self.advance().token.clone() {
                                name_parts.push(part);
                            }
                        } else {
                            break;
                        }
                    }
                    let class_name = name_parts.join(".");
                    let class = Box::new(Expression::Identifier(Identifier {
                        name: class_name,
                        location: self.current_location(),
                    }));
                    let args = if self.match_token(&Token::LParen) {
                        let a = self.parse_arguments()?;
                        self.consume(&Token::RParen)?;
                        a
                    } else {
                        Vec::new()
                    };
                    Ok(Expression::New(Box::new(NewExpression {
                        class,
                        arguments: args,
                        location: self.current_location(),
                    })))
                } else {
                    // Fallback for non-identifier new (e.g. new (expr)())
                    let class = Box::new(self.parse_call()?);
                    let args = if self.match_token(&Token::LParen) {
                        let a = self.parse_arguments()?;
                        self.consume(&Token::RParen)?;
                        a
                    } else {
                        Vec::new()
                    };
                    Ok(Expression::New(Box::new(NewExpression {
                        class,
                        arguments: args,
                        location: self.current_location(),
                    })))
                }
            }
            Token::Function => self.parse_closure(),
            Token::LParen => {
                // Arrow function check: (params) => expr or regular grouping: (expr)
                // We need to peek ahead to see if this is an arrow function
                // Note: parse_primary() already advanced past the LParen, so self.current points to the next token

                // Look ahead to find ) and check if => follows
                let mut is_arrow = false;
                {
                    let mut offset = 0;
                    // Skip past identifiers and commas
                    loop {
                        if offset < self.tokens.len() - self.current {
                            match &self.tokens[self.current + offset].token {
                                Token::Identifier(_) | Token::Comma => {
                                    offset += 1;
                                    continue;
                                }
                                Token::RParen => {
                                    // Check for => after )
                                    if offset + 1 < self.tokens.len() - self.current {
                                        if matches!(&self.tokens[self.current + offset + 1].token, Token::FatArrow) {
                                            is_arrow = true;
                                        }
                                    }
                                    break;
                                }
                                _ => break,
                            }
                        }
                        break;
                    }
                }

                if is_arrow {
                    // Parse as arrow function
                    let mut params = Vec::new();

                    if !self.check(&Token::RParen) {
                        loop {
                            let param_name = match self.peek(0).clone() {
                                Token::Identifier(id) => {
                                    let name = id.clone();
                                    self.advance();
                                    name
                                }
                                _ => "arg".to_string(),
                            };
                            params.push(Param {
                                name: param_name,
                                param_type: None,
                                default: None,
                                required: false,
                            });
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(&Token::RParen)?;
                    self.match_token(&Token::FatArrow); // consume =>

                    // Block-bodied arrow function: `(p) => { stmt; stmt; }`
                    // Compile as a closure so statements are supported.
                    if self.check(&Token::LBrace) {
                        let body = self.parse_block()?;
                        return Ok(Expression::Closure(Box::new(Closure {
                            params,
                            body,
                            location: self.current_location(),
                            metadata: Vec::new(),
                        })));
                    }

                    let body = self.parse_expression()?;
                    return Ok(Expression::ArrowFunction(Box::new(ArrowFunction {
                        params,
                        body: Box::new(body),
                        location: self.current_location(),
                    })));
                }

                // Not an arrow function - parse as grouped expression
                let expr = self.parse_expression()?;
                self.consume(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => self.parse_array_literal(),
            Token::LBrace => self.parse_struct_literal(),
            _ => Ok(Expression::Empty),
        }
    }

    fn parse_closure(&mut self) -> Result<Expression, ParseError> {
        // Optional name for named closures
        let _name = if let Token::Identifier(_) = self.peek(0) {
            Some(self.extract_identifier()?)
        } else {
            None
        };

        self.consume(&Token::LParen)?;
        let params = self.parse_param_list()?;
        self.consume(&Token::RParen)?;

        // Capture optional closure metadata attributes (e.g.,
        // `localmode = "classic"`). These appear between the RParen and
        // LBrace. Only literal-string values are stored — anything else is
        // skipped to preserve forward-compat with future attribute kinds.
        let mut metadata: Vec<(String, String)> = Vec::new();
        while !self.check(&Token::LBrace) && !self.is_at_end() {
            if self.is_identifier_like() && matches!(self.peek(1), Token::Equal) {
                let key = self.extract_identifier()?;
                self.advance(); // skip =
                if let Token::String(val) = self.peek(0).clone() {
                    self.advance();
                    metadata.push((key, val));
                } else {
                    // Non-string value — preserve old behaviour and skip.
                    self.parse_expression()?;
                }
            } else {
                break;
            }
        }

        let body = self.parse_block()?;

        Ok(Expression::Closure(Box::new(Closure {
            params,
            body,
            location: self.current_location(),
            metadata,
        })))
    }

    fn parse_array_literal(&mut self) -> Result<Expression, ParseError> {
        let mut elements = Vec::new();

        if !self.check(&Token::RBracket) {
            loop {
                if self.check(&Token::RBracket) {
                    break; // trailing comma
                }
                if self.match_token(&Token::DotDotDot) {
                    let expr = self.parse_expression()?;
                    elements.push(Expression::Spread(Box::new(expr)));
                } else {
                    let expr = self.parse_expression()?;
                    // `[ key: value, ... ]` is an ordered struct literal (Lucee).
                    // We only learn this once we hit the first colon, so hand off
                    // to the bracket-struct parser carrying the key parsed so far.
                    if elements.is_empty() && self.match_token(&Token::Colon) {
                        return self.parse_bracket_struct_literal(expr);
                    }
                    elements.push(expr);
                }
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        self.consume(&Token::RBracket)?;

        Ok(Expression::Array(Array {
            elements,
            location: self.current_location(),
        }))
    }

    /// Finish parsing a bracket-delimited ordered struct literal
    /// (`[ "key": value, ... ]`). `first_key` is the already-parsed key whose
    /// trailing `:` has just been consumed.
    fn parse_bracket_struct_literal(
        &mut self,
        first_key: Expression,
    ) -> Result<Expression, ParseError> {
        let mut pairs = Vec::new();
        let first_value = self.parse_expression()?;
        pairs.push((first_key, first_value));

        while self.match_token(&Token::Comma) {
            if self.check(&Token::RBracket) {
                break; // trailing comma
            }
            if self.match_token(&Token::DotDotDot) {
                let expr = self.parse_expression()?;
                pairs.push((Expression::Spread(Box::new(expr.clone())), expr));
                continue;
            }
            let is_key_eq =
                self.is_identifier_like_at(0) && matches!(self.peek(1), Token::Equal);
            let key = if is_key_eq {
                self.parse_ternary()?
            } else {
                self.parse_expression()?
            };
            if self.match_token(&Token::Colon) || self.match_token(&Token::Equal) {
                let value = self.parse_expression()?;
                pairs.push((key, value));
            } else {
                // Shorthand [x] means [x: x]
                pairs.push((key.clone(), key));
            }
        }

        self.consume(&Token::RBracket)?;

        Ok(Expression::Struct(Struct {
            pairs,
            ordered: true,
            location: self.current_location(),
        }))
    }

    fn parse_struct_literal(&mut self) -> Result<Expression, ParseError> {
        let mut pairs = Vec::new();

        if !self.check(&Token::RBrace) {
            loop {
                if self.check(&Token::RBrace) {
                    break; // trailing comma
                }
                if self.match_token(&Token::DotDotDot) {
                    // Spread: ...expr merges another struct
                    let expr = self.parse_expression()?;
                    // Use a sentinel key to mark this as a spread entry
                    pairs.push((Expression::Spread(Box::new(expr.clone())), expr));
                } else {
                    // In struct literals, `=` is a key-value separator (like `:`),
                    // NOT an assignment operator. We must parse the key without
                    // consuming `=` as assignment.
                    // Check for simple `identifier =` pattern first (most common case).
                    // is_identifier_like_at covers soft-keyword keys too (component,
                    // output, ...) so `{ component = x }` treats `component` as the
                    // KEY rather than parsing `component = x` as an assignment expr.
                    let is_key_eq = self.is_identifier_like_at(0)
                        && matches!(self.peek(1), Token::Equal);
                    let key = if is_key_eq {
                        // Parse just the identifier, don't let parse_expression consume `=`
                        self.parse_ternary()?
                    } else {
                        self.parse_expression()?
                    };

                    // Support both : and = for struct initialization
                    if self.match_token(&Token::Colon) || self.match_token(&Token::Equal) {
                        let value = self.parse_expression()?;
                        pairs.push((key, value));
                    } else {
                        // Shorthand {x} means {x: x}
                        pairs.push((key.clone(), key));
                    }
                }

                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        self.consume(&Token::RBrace)?;

        Ok(Expression::Struct(Struct {
            pairs,
            ordered: false,
            location: self.current_location(),
        }))
    }
}

/// Try to lower `param name="<dotted.path>['#expr#']" default=<d>;` into:
///
///   if (!structKeyExists(<dotted.path>, <expr>)) {
///       <dotted.path>[<expr>] = <d>;
///   }
///
/// Returns None if the interpolation does not match this exact shape.
/// This is a deliberately narrow optimisation aimed at the Taffy/FW1
/// `mimeExtensions['#ext#']` idiom — anything more general falls through
/// to the runtime `__cfparam` dispatch.
fn try_lower_dynamic_param(
    interp: &StringInterpolation,
    default_val: &Expression,
    loc: SourceLocation,
) -> Option<Statement> {
    // Three accepted shapes:
    //   1. parts.len() == 3 — `<path>['#expr#']` (or "...")
    //   2. parts.len() == 2 — `<path>.#expr#` (interpolated trailing key)
    //   3. parts.len() == 3 — `<path>['#expr#'].<lit>(.<lit>...)` — bracket then dotted literal tail
    let prefix_path: &str;
    let key_part_index: usize;
    let mut trailing_literals: Vec<String> = Vec::new();
    match interp.parts.len() {
        3 => {
            let prefix = match &interp.parts[0] {
                Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => s,
                _ => return None,
            };
            let suffix = match &interp.parts[2] {
                Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => s,
                _ => return None,
            };
            let p = if let Some(p) = prefix.strip_suffix("['") {
                p
            } else if let Some(p) = prefix.strip_suffix("[\"") {
                p
            } else {
                return None;
            };
            // Suffix must start with the matching closing quote+bracket; anything after
            // is a dotted literal tail (shape 3).
            let after_close = if let Some(rest) = suffix.strip_prefix("']") {
                rest
            } else if let Some(rest) = suffix.strip_prefix("\"]") {
                rest
            } else {
                return None;
            };
            if !after_close.is_empty() {
                // Expect `.<ident>(.<ident>)*`
                if !after_close.starts_with('.') {
                    return None;
                }
                for seg in after_close[1..].split('.') {
                    if !is_simple_ident(seg) {
                        return None;
                    }
                    trailing_literals.push(seg.to_string());
                }
            }
            prefix_path = p;
            key_part_index = 1;
        }
        2 => {
            let prefix = match &interp.parts[0] {
                Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => s,
                _ => return None,
            };
            let p = match prefix.strip_suffix('.') {
                Some(p) => p,
                None => return None,
            };
            prefix_path = p;
            key_part_index = 1;
        }
        _ => return None,
    }
    if prefix_path.is_empty() {
        return None;
    }
    // Build a MemberAccess chain from the dotted prefix.
    let segments: Vec<&str> = prefix_path.split('.').collect();
    if segments.iter().any(|s| s.is_empty() || !is_simple_ident(s)) {
        return None;
    }
    let mut base = Expression::Identifier(Identifier {
        name: segments[0].to_string(),
        location: loc,
    });
    for seg in &segments[1..] {
        base = Expression::MemberAccess(Box::new(MemberAccess {
            object: Box::new(base),
            member: (*seg).to_string(),
            null_safe: false,
            location: loc,
        }));
    }
    let key_expr = interp.parts[key_part_index].clone();

    if trailing_literals.is_empty() {
        // Shape 1 & 2: `base[key]` is the leaf.
        // Condition: !structKeyExists(base, key)
        let cond = Expression::UnaryOp(Box::new(UnaryOp {
            operator: UnaryOpType::Not,
            operand: Box::new(Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "structKeyExists".to_string(),
                    location: loc,
                })),
                arguments: vec![base.clone(), key_expr.clone()],
                location: loc,
            }))),
            location: loc,
        }));
        let assign = Statement::Assignment(Assignment {
            target: AssignTarget::ArrayAccess(Box::new(base), Box::new(key_expr)),
            value: default_val.clone(),
            operator: AssignOp::Equal,
            location: loc,
        });
        Some(Statement::If(If {
            condition: cond,
            then_branch: vec![assign],
            else_if: vec![],
            else_branch: None,
            location: loc,
        }))
    } else {
        // Shape 3: `base[key].lit1.lit2...litN` is the leaf.
        // Build expr `base[key].lit1...litN-1` (parent of leaf) and the leaf member name.
        let mut parent = Expression::ArrayAccess(Box::new(ArrayAccess {
            array: Box::new(base),
            index: Box::new(key_expr),
            location: loc,
        }));
        let leaf = trailing_literals.last().unwrap().clone();
        for seg in &trailing_literals[..trailing_literals.len() - 1] {
            parent = Expression::MemberAccess(Box::new(MemberAccess {
                object: Box::new(parent),
                member: seg.clone(),
                null_safe: false,
                location: loc,
            }));
        }
        // Condition: !structKeyExists(parent, "leaf")
        let cond = Expression::UnaryOp(Box::new(UnaryOp {
            operator: UnaryOpType::Not,
            operand: Box::new(Expression::FunctionCall(Box::new(FunctionCall {
                name: Box::new(Expression::Identifier(Identifier {
                    name: "structKeyExists".to_string(),
                    location: loc,
                })),
                arguments: vec![
                    parent.clone(),
                    Expression::Literal(Literal {
                        value: LiteralValue::String(leaf.clone()),
                        location: loc,
                    }),
                ],
                location: loc,
            }))),
            location: loc,
        }));
        // Body: parent.leaf = default
        let assign = Statement::Assignment(Assignment {
            target: AssignTarget::StructAccess(Box::new(parent), leaf),
            value: default_val.clone(),
            operator: AssignOp::Equal,
            location: loc,
        });
        Some(Statement::If(If {
            condition: cond,
            then_branch: vec![assign],
            else_if: vec![],
            else_branch: None,
            location: loc,
        }))
    }
}

fn is_simple_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl TryFrom<CfmlNode> for Statement {
    type Error = ();

    fn try_from(node: CfmlNode) -> Result<Self, Self::Error> {
        match node {
            CfmlNode::Statement(s) => Ok(s),
            _ => Err(()),
        }
    }
}
