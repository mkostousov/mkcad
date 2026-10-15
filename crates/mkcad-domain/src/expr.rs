//! Expression lexer, parser, and evaluator.
//!
//! Supports arithmetic operators (+, -, *, /, ^), functions (max, min, abs, sqrt, sin, cos, tan, etc.),
//! constants (PI, E), parameter references, unit parsing, and dimensional analysis.
//! Function argument separator is "," (comma, locale-independent per N.2a-6).

use crate::units::{DimensionKind, Quantity, UnitTable};
use std::collections::HashMap;

/// Expression parse error types.
#[derive(Clone, Debug, PartialEq)]
pub enum ExprError {
    /// Unexpected token during parsing.
    UnexpectedToken(String),
    /// Division by zero.
    DivisionByZero,
    /// Unit mismatch (e.g., 40 kg for a length field).
    UnitMismatch {
        /// Expected dimension kind.
        expected: DimensionKind,
        /// Got dimension kind.
        got: DimensionKind,
    },
    /// Unknown function name.
    UnknownFunction(String),
    /// Unknown identifier (parameter not found).
    UnknownIdentifier(String),
    /// Syntax error (e.g., malformed expression).
    SyntaxError(String),
}

impl std::fmt::Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprError::UnexpectedToken(s) => write!(f, "Unexpected token: {}", s),
            ExprError::DivisionByZero => write!(f, "Division by zero"),
            ExprError::UnitMismatch { expected, got } => {
                write!(f, "Unit mismatch: expected {:?}, got {:?}", expected, got)
            }
            ExprError::UnknownFunction(s) => write!(f, "Unknown function: {}", s),
            ExprError::UnknownIdentifier(s) => write!(f, "Unknown identifier: {}", s),
            ExprError::SyntaxError(s) => write!(f, "Syntax error: {}", s),
        }
    }
}

/// Lexer token types.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LeftParen,
    RightParen,
    Comma,
    Eof,
}

/// Lexer for expression strings.
struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    /// Creates a new lexer from a string.
    fn new(input: &str) -> Self {
        Lexer { input: input.chars().collect(), pos: 0 }
    }

    /// Returns the current character without advancing.
    fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    /// Advances and returns the current character.
    fn next(&mut self) -> Option<char> {
        if self.pos < self.input.len() {
            let c = self.input[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    /// Skips whitespace.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    /// Parses a number (integer or float).
    fn read_number(&mut self) -> f64 {
        let mut num_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                num_str.push(c);
                self.next();
            } else {
                break;
            }
        }
        num_str.parse().unwrap_or(0.0)
    }

    /// Parses an identifier (name or unit).
    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.next();
            } else {
                break;
            }
        }
        ident
    }

    /// Returns the next token.
    fn next_token(&mut self) -> Result<Token, ExprError> {
        self.skip_whitespace();

        match self.peek() {
            None => Ok(Token::Eof),
            Some('+') => {
                self.next();
                Ok(Token::Plus)
            }
            Some('-') => {
                self.next();
                Ok(Token::Minus)
            }
            Some('*') => {
                self.next();
                Ok(Token::Star)
            }
            Some('/') => {
                self.next();
                Ok(Token::Slash)
            }
            Some('^') => {
                self.next();
                Ok(Token::Caret)
            }
            Some('(') => {
                self.next();
                Ok(Token::LeftParen)
            }
            Some(')') => {
                self.next();
                Ok(Token::RightParen)
            }
            Some(',') => {
                self.next();
                Ok(Token::Comma)
            }
            Some(c) if c.is_ascii_digit() => Ok(Token::Number(self.read_number())),
            Some(c) if c.is_alphabetic() || c == '_' => Ok(Token::Identifier(self.read_identifier())),
            Some(c) => Err(ExprError::SyntaxError(format!("Unexpected character: {}", c))),
        }
    }
}

/// Abstract syntax tree node for expressions.
#[derive(Clone, Debug)]
enum AstNode {
    /// Numeric literal with optional unit.
    Literal { value: f64, unit: Option<String> },
    /// Identifier (parameter name or constant).
    Identifier(String),
    /// Binary operation.
    BinOp { op: char, left: Box<AstNode>, right: Box<AstNode> },
    /// Unary operation (e.g., -expr).
    UnaryOp { op: char, operand: Box<AstNode> },
    /// Function call with arguments.
    FuncCall { name: String, args: Vec<AstNode> },
}

/// Expression parser.
struct Parser {
    lexer: Lexer,
    current_token: Token,
}

impl Parser {
    /// Creates a new parser from an input string.
    fn new(input: &str) -> Result<Self, ExprError> {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token()?;
        Ok(Parser { lexer, current_token })
    }

    /// Advances to the next token.
    fn advance(&mut self) -> Result<(), ExprError> {
        self.current_token = self.lexer.next_token()?;
        Ok(())
    }

    /// Parses the entire expression.
    fn parse(&mut self) -> Result<AstNode, ExprError> {
        self.parse_additive()
    }

    /// Parses addition and subtraction (lowest precedence).
    fn parse_additive(&mut self) -> Result<AstNode, ExprError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            match &self.current_token {
                Token::Plus => {
                    self.advance()?;
                    let right = self.parse_multiplicative()?;
                    left = AstNode::BinOp { op: '+', left: Box::new(left), right: Box::new(right) };
                }
                Token::Minus => {
                    self.advance()?;
                    let right = self.parse_multiplicative()?;
                    left = AstNode::BinOp { op: '-', left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parses multiplication and division.
    fn parse_multiplicative(&mut self) -> Result<AstNode, ExprError> {
        let mut left = self.parse_power()?;

        loop {
            match &self.current_token {
                Token::Star => {
                    self.advance()?;
                    let right = self.parse_power()?;
                    left = AstNode::BinOp { op: '*', left: Box::new(left), right: Box::new(right) };
                }
                Token::Slash => {
                    self.advance()?;
                    let right = self.parse_power()?;
                    left = AstNode::BinOp { op: '/', left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parses exponentiation (right-associative).
    fn parse_power(&mut self) -> Result<AstNode, ExprError> {
        let left = self.parse_unary()?;

        if matches!(self.current_token, Token::Caret) {
            self.advance()?;
            let right = self.parse_power()?; // Right-associative
            Ok(AstNode::BinOp { op: '^', left: Box::new(left), right: Box::new(right) })
        } else {
            Ok(left)
        }
    }

    /// Parses unary operations and primary expressions.
    fn parse_unary(&mut self) -> Result<AstNode, ExprError> {
        match &self.current_token {
            Token::Minus => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(AstNode::UnaryOp { op: '-', operand: Box::new(operand) })
            }
            Token::Plus => {
                self.advance()?;
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    /// Parses primary expressions (literals, identifiers, function calls, parentheses).
    fn parse_primary(&mut self) -> Result<AstNode, ExprError> {
        match &self.current_token.clone() {
            Token::Number(n) => {
                let value = *n;
                self.advance()?;

                // Check for unit suffix
                let unit = if let Token::Identifier(ident) = &self.current_token {
                    let unit = Some(ident.clone());
                    self.advance()?;
                    unit
                } else {
                    None
                };

                Ok(AstNode::Literal { value, unit })
            }
            Token::Identifier(ident) => {
                let name = ident.clone();
                self.advance()?;

                // Check if it's a function call
                if matches!(self.current_token, Token::LeftParen) {
                    self.advance()?;
                    let args = self.parse_function_args()?;
                    if !matches!(self.current_token, Token::RightParen) {
                        return Err(ExprError::SyntaxError("Expected ')'".to_string()));
                    }
                    self.advance()?;
                    Ok(AstNode::FuncCall { name, args })
                } else {
                    // It's a parameter reference or constant
                    Ok(AstNode::Identifier(name))
                }
            }
            Token::LeftParen => {
                self.advance()?;
                let expr = self.parse_additive()?;
                if !matches!(self.current_token, Token::RightParen) {
                    return Err(ExprError::SyntaxError("Expected ')'".to_string()));
                }
                self.advance()?;
                Ok(expr)
            }
            _ => Err(ExprError::UnexpectedToken(format!("{:?}", self.current_token))),
        }
    }

    /// Parses function arguments (comma-separated).
    fn parse_function_args(&mut self) -> Result<Vec<AstNode>, ExprError> {
        let mut args = Vec::new();

        if !matches!(self.current_token, Token::RightParen) {
            loop {
                args.push(self.parse_additive()?);
                if matches!(self.current_token, Token::Comma) {
                    self.advance()?;
                } else {
                    break;
                }
            }
        }

        Ok(args)
    }
}

/// Expression evaluator context.
pub struct EvalContext {
    parameters: HashMap<String, Quantity>,
    unit_table: UnitTable,
}

impl EvalContext {
    /// Creates a new evaluation context with an empty parameter table.
    pub fn new() -> Self {
        EvalContext {
            parameters: HashMap::new(),
            unit_table: UnitTable::new(),
        }
    }

    /// Registers a parameter in the context.
    pub fn set_parameter(&mut self, name: String, value: Quantity) {
        self.parameters.insert(name, value);
    }

    /// Parses and evaluates an expression string.
    /// Returns the result quantity or an error.
    pub fn eval(&self, expr: &str) -> Result<Quantity, ExprError> {
        let mut parser = Parser::new(expr)?;
        let ast = parser.parse()?;
        self.eval_node(&ast)
    }

    /// Evaluates an AST node with dimensional analysis.
    fn eval_node(&self, node: &AstNode) -> Result<Quantity, ExprError> {
        match node {
            AstNode::Literal { value, unit } => {
                let q = if let Some(u) = unit {
                    let (dim, factor) = self.unit_table.parse(u)
                        .ok_or_else(|| ExprError::UnknownFunction(u.clone()))?;
                    Quantity { value: value * factor, dimension: dim }
                } else {
                    Quantity::unitless(*value)
                };
                Ok(q)
            }
            AstNode::Identifier(name) => {
                if name == "PI" {
                    Ok(Quantity::unitless(std::f64::consts::PI))
                } else if name == "E" {
                    Ok(Quantity::unitless(std::f64::consts::E))
                } else if let Some(&q) = self.parameters.get(name) {
                    Ok(q)
                } else {
                    Err(ExprError::UnknownIdentifier(name.clone()))
                }
            }
            AstNode::BinOp { op, left, right } => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;

                match op {
                    '+' | '-' => {
                        if !left_val.compatible_with(right_val) {
                            return Err(ExprError::UnitMismatch {
                                expected: left_val.dimension,
                                got: right_val.dimension,
                            });
                        }
                        let result = if *op == '+' {
                            left_val.value + right_val.value
                        } else {
                            left_val.value - right_val.value
                        };
                        Ok(Quantity { value: result, dimension: left_val.dimension })
                    }
                    '*' => {
                        // Multiply dimensions: length * length = area (unitless for now)
                        let result_value = left_val.value * right_val.value;
                        let result_dim = match (left_val.dimension, right_val.dimension) {
                            (DimensionKind::Unitless, d) | (d, DimensionKind::Unitless) => d,
                            (DimensionKind::Length, DimensionKind::Length) => DimensionKind::Unitless,
                            (DimensionKind::Angle, DimensionKind::Angle) => DimensionKind::Unitless,
                            (DimensionKind::Mass, DimensionKind::Mass) => DimensionKind::Unitless,
                            _ => DimensionKind::Unitless, // Simplified; full dimensional analysis later
                        };
                        Ok(Quantity { value: result_value, dimension: result_dim })
                    }
                    '/' => {
                        if right_val.value == 0.0 {
                            return Err(ExprError::DivisionByZero);
                        }
                        let result_value = left_val.value / right_val.value;
                        let result_dim = if right_val.dimension == DimensionKind::Unitless {
                            left_val.dimension
                        } else {
                            // Reciprocal, same units cancel, or different units: all simplified to unitless
                            DimensionKind::Unitless
                        };
                        Ok(Quantity { value: result_value, dimension: result_dim })
                    }
                    '^' => {
                        if right_val.dimension != DimensionKind::Unitless {
                            return Err(ExprError::SyntaxError("Exponent must be unitless".to_string()));
                        }
                        let result = left_val.value.powf(right_val.value);
                        Ok(Quantity { value: result, dimension: left_val.dimension })
                    }
                    _ => Err(ExprError::SyntaxError(format!("Unknown operator: {}", op))),
                }
            }
            AstNode::UnaryOp { op, operand } => {
                let val = self.eval_node(operand)?;
                match op {
                    '-' => Ok(Quantity { value: -val.value, dimension: val.dimension }),
                    _ => Err(ExprError::SyntaxError(format!("Unknown unary operator: {}", op))),
                }
            }
            AstNode::FuncCall { name, args } => {
                match name.as_str() {
                    "max" => {
                        if args.is_empty() {
                            return Err(ExprError::SyntaxError("max requires at least 1 argument".to_string()));
                        }
                        let mut max_val = self.eval_node(&args[0])?;
                        for arg in &args[1..] {
                            let val = self.eval_node(arg)?;
                            if !val.compatible_with(max_val) {
                                return Err(ExprError::UnitMismatch {
                                    expected: max_val.dimension,
                                    got: val.dimension,
                                });
                            }
                            if val.value > max_val.value {
                                max_val = val;
                            }
                        }
                        Ok(max_val)
                    }
                    "min" => {
                        if args.is_empty() {
                            return Err(ExprError::SyntaxError("min requires at least 1 argument".to_string()));
                        }
                        let mut min_val = self.eval_node(&args[0])?;
                        for arg in &args[1..] {
                            let val = self.eval_node(arg)?;
                            if !val.compatible_with(min_val) {
                                return Err(ExprError::UnitMismatch {
                                    expected: min_val.dimension,
                                    got: val.dimension,
                                });
                            }
                            if val.value < min_val.value {
                                min_val = val;
                            }
                        }
                        Ok(min_val)
                    }
                    "abs" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("abs requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        Ok(Quantity { value: val.value.abs(), dimension: val.dimension })
                    }
                    "sqrt" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("sqrt requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        Ok(Quantity { value: val.value.sqrt(), dimension: val.dimension })
                    }
                    "sin" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("sin requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        // sin expects radians; input must be angle dimension
                        if val.dimension != DimensionKind::Angle {
                            return Err(ExprError::UnitMismatch {
                                expected: DimensionKind::Angle,
                                got: val.dimension,
                            });
                        }
                        Ok(Quantity::unitless(val.value.sin()))
                    }
                    "cos" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("cos requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        if val.dimension != DimensionKind::Angle {
                            return Err(ExprError::UnitMismatch {
                                expected: DimensionKind::Angle,
                                got: val.dimension,
                            });
                        }
                        Ok(Quantity::unitless(val.value.cos()))
                    }
                    "tan" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("tan requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        if val.dimension != DimensionKind::Angle {
                            return Err(ExprError::UnitMismatch {
                                expected: DimensionKind::Angle,
                                got: val.dimension,
                            });
                        }
                        Ok(Quantity::unitless(val.value.tan()))
                    }
                    "sin_deg" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("sin_deg requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        if val.dimension != DimensionKind::Unitless {
                            return Err(ExprError::UnitMismatch {
                                expected: DimensionKind::Unitless,
                                got: val.dimension,
                            });
                        }
                        let rad = val.value * std::f64::consts::PI / 180.0;
                        Ok(Quantity::unitless(rad.sin()))
                    }
                    "cos_deg" => {
                        if args.len() != 1 {
                            return Err(ExprError::SyntaxError("cos_deg requires 1 argument".to_string()));
                        }
                        let val = self.eval_node(&args[0])?;
                        if val.dimension != DimensionKind::Unitless {
                            return Err(ExprError::UnitMismatch {
                                expected: DimensionKind::Unitless,
                                got: val.dimension,
                            });
                        }
                        let rad = val.value * std::f64::consts::PI / 180.0;
                        Ok(Quantity::unitless(rad.cos()))
                    }
                    _ => Err(ExprError::UnknownFunction(name.clone())),
                }
            }
        }
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple_number() {
        let ctx = EvalContext::new();
        let result = ctx.eval("42").unwrap();
        assert_eq!(result.value, 42.0);
        assert_eq!(result.dimension, DimensionKind::Unitless);
    }

    #[test]
    fn test_eval_with_unit() {
        let ctx = EvalContext::new();
        let result = ctx.eval("40 mm").unwrap();
        assert_eq!(result.value, 40.0);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    #[test]
    fn test_eval_addition() {
        let ctx = EvalContext::new();
        let result = ctx.eval("1 + 1").unwrap();
        assert_eq!(result.value, 2.0);
    }

    #[test]
    fn test_eval_addition_with_units() {
        let ctx = EvalContext::new();
        let result = ctx.eval("40 mm + 1 mm").unwrap();
        assert_eq!(result.value, 41.0);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    #[test]
    fn test_eval_unit_mismatch() {
        let ctx = EvalContext::new();
        let result = ctx.eval("40 kg + 1 mm");
        assert!(matches!(result, Err(ExprError::UnitMismatch { .. })));
    }

    #[test]
    fn test_eval_unary_minus() {
        let ctx = EvalContext::new();
        let result = ctx.eval("-5").unwrap();
        assert_eq!(result.value, -5.0);
    }

    #[test]
    fn test_eval_multiplication() {
        let ctx = EvalContext::new();
        let result = ctx.eval("3 * 4").unwrap();
        assert_eq!(result.value, 12.0);
    }

    #[test]
    fn test_eval_division() {
        let ctx = EvalContext::new();
        let result = ctx.eval("10 / 2").unwrap();
        assert_eq!(result.value, 5.0);
    }

    #[test]
    fn test_eval_division_by_zero() {
        let ctx = EvalContext::new();
        let result = ctx.eval("10 / 0");
        assert!(matches!(result, Err(ExprError::DivisionByZero)));
    }

    #[test]
    fn test_eval_power() {
        let ctx = EvalContext::new();
        let result = ctx.eval("2 ^ 3").unwrap();
        assert_eq!(result.value, 8.0);
    }

    #[test]
    fn test_eval_pi_constant() {
        let ctx = EvalContext::new();
        let result = ctx.eval("PI").unwrap();
        assert!((result.value - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_eval_pi_lowercase_error() {
        let ctx = EvalContext::new();
        let result = ctx.eval("pi");
        assert!(matches!(result, Err(ExprError::UnknownIdentifier(_))));
    }

    #[test]
    fn test_eval_max_function() {
        let ctx = EvalContext::new();
        let result = ctx.eval("max(10, 20, 5)").unwrap();
        assert_eq!(result.value, 20.0);
    }

    #[test]
    fn test_eval_min_function() {
        let ctx = EvalContext::new();
        let result = ctx.eval("min(10, 20, 5)").unwrap();
        assert_eq!(result.value, 5.0);
    }

    #[test]
    fn test_eval_abs_function() {
        let ctx = EvalContext::new();
        let result = ctx.eval("abs(-5)").unwrap();
        assert_eq!(result.value, 5.0);
    }

    #[test]
    fn test_eval_sqrt_function() {
        let ctx = EvalContext::new();
        let result = ctx.eval("sqrt(9)").unwrap();
        assert_eq!(result.value, 3.0);
    }

    #[test]
    fn test_eval_parameter_reference() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("Len".to_string(), Quantity::length_mm(40.0));
        let result = ctx.eval("Len").unwrap();
        assert_eq!(result.value, 40.0);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    #[test]
    fn test_eval_expression_with_parameter() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("Len".to_string(), Quantity::length_mm(40.0));
        let result = ctx.eval("Len + 1 in").unwrap();
        assert_eq!(result.value, 40.0 + 25.4);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    #[test]
    fn test_eval_sin_function() {
        let ctx = EvalContext::new();
        let result = ctx.eval("sin(0 rad)").unwrap();
        // sin(0 rad) should be 0 (checking with tolerance for floating point)
        assert!(result.value.abs() < 1e-10, "sin(0 rad) = {}", result.value);
        assert_eq!(result.dimension, DimensionKind::Unitless);
    }

    #[test]
    fn test_eval_division_unitless() {
        let ctx = EvalContext::new();
        let result = ctx.eval("40 mm / 1 mm").unwrap();
        assert_eq!(result.value, 40.0);
        assert_eq!(result.dimension, DimensionKind::Unitless);
    }

    #[test]
    fn test_eval_parentheses() {
        let ctx = EvalContext::new();
        let result = ctx.eval("(10 + 20) * 2").unwrap();
        assert_eq!(result.value, 60.0);
    }

    #[test]
    fn test_eval_unit_conversion_inch_to_mm() {
        let ctx = EvalContext::new();
        let result = ctx.eval("1 in").unwrap();
        assert!((result.value - 25.4).abs() < 1e-10);
    }
}
