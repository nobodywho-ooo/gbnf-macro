use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token};

// here are the gnbf docs I reference
// https://github.com/ggml-org/llama.cpp/blob/e9fd8dcab45d6cd147874e32565923bdfd0efbdb/grammars/README.md

/// from docs:
/// ## Example
/// ```text
/// # `root` specifies the pattern for the overall output
/// root ::= (
///     # it must start with the characters "1. " followed by a sequence
///     # of characters that match the `move` rule, followed by a space, followed
///     # by another move, and then a newline
///     "1. " move " " move "\n"
///
///     # it's followed by one or more subsequent moves, numbered with one or two digits
///     ([1-9] [0-9]? ". " move " " move "\n")+
/// )
///
/// # `move` is an abstract representation, which can be a pawn, nonpawn, or castle.
/// # The `[+#]?` denotes the possibility of checking or mate signs after moves
/// move ::= (pawn | nonpawn | castle) [+#]?
///
/// pawn ::= ...
/// nonpawn ::= ...
/// castle ::= ...
/// ```

/// from docs:
/// > Non-terminal symbols (rule names) stand for a pattern of terminals and other non-terminals. They are required to be a dashed lowercase word, like `move`, `castle`, or `check-mate`.

#[derive(Debug, Clone, PartialEq, Eq)]
struct NonTerminalSymbol(Ident);

impl NonTerminalSymbol {
    fn new(ident: Ident) -> Result<Self> {
        let name = ident.to_string();

        // Validate: must be lowercase letters and dashes only
        if !name.chars().all(|c| c.is_lowercase() || c == '-') {
            return Err(syn::Error::new_spanned(
                ident,
                "non-terminal symbols must be dashed lowercase words (e.g., 'move', 'castle', 'check-mate')",
            ));
        }

        Ok(NonTerminalSymbol(ident))
    }

    fn as_ident(&self) -> &Ident {
        &self.0
    }
}

impl Parse for NonTerminalSymbol {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        NonTerminalSymbol::new(ident)
    }
}

/// from docs:
///
/// ## Characters and character ranges
///
/// > Terminals support the full range of Unicode. Unicode characters can be specified directly in the grammar, for example `hiragana ::= [ぁ-ゟ]`, or with escapes: 8-bit (`\xXX`), 16-bit (`\uXXXX`) or 32-bit (`\UXXXXXXXX`).
///
/// > Character ranges can be negated with `^`:
/// ```text
/// single-line ::= [^\n]+ "\n"
/// ```

enum CharacterRange {
    StartEndRange {
        begin: char,
        end: char,
        negated: bool,
    },
    CharacterSet {
        chars: Vec<char>,
        negated: bool,
    },
}

impl Parse for CharacterRange {
    fn parse(input: ParseStream) -> Result<Self> {
        use syn::bracketed;

        let content;
        bracketed!(content in input);

        // Check for negation
        let negated = if content.peek(Token![^]) {
            content.parse::<Token![^]>()?;
            true
        } else {
            false
        };

        // Parse the first character (could be an ident like 'a' or a literal)
        let first_char = parse_char(&content)?;

        // Check if this is a range (char-char) or a set
        if content.peek(Token![-]) {
            // This is a range: [a-z] or [^a-z]
            content.parse::<Token![-]>()?;
            let end_char = parse_char(&content)?;

            Ok(CharacterRange::StartEndRange {
                begin: first_char,
                end: end_char,
                negated,
            })
        } else {
            // This is a character set: [abc] or [^abc]
            let mut chars = vec![first_char];

            while !content.is_empty() {
                chars.push(parse_char(&content)?);
            }

            Ok(CharacterRange::CharacterSet { chars, negated })
        }
    }
}

// Helper function to parse a single character from various token forms
fn parse_char(input: ParseStream) -> Result<char> {
    // Try to parse as an identifier (like 'a', 'z', etc.)
    if let Ok(ident) = input.parse::<Ident>() {
        let s = ident.to_string();
        if s.len() == 1 {
            return Ok(s.chars().next().unwrap());
        }
        return Err(syn::Error::new_spanned(
            ident,
            "expected a single character",
        ));
    }

    // Try to parse as a literal (for numbers like '0', '9', or escaped chars)
    if let Ok(lit) = input.parse::<syn::LitInt>() {
        let s = lit.to_string();
        if s.len() == 1 {
            return Ok(s.chars().next().unwrap());
        }
        return Err(syn::Error::new_spanned(lit, "expected a single digit"));
    }

    Err(input.error("expected a character"))
}

// Quantifiers for repetition
enum Quantifier {
    Optional,            // ?
    OneOrMore,           // +
    ZeroOrMore,          // *
    Exact(usize),        // {m}
    AtLeast(usize),      // {m,}
    Range(usize, usize), // {m,n}
}

impl Parse for Quantifier {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            Ok(Quantifier::Optional)
        } else if input.peek(Token![+]) {
            input.parse::<Token![+]>()?;
            Ok(Quantifier::OneOrMore)
        } else if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            Ok(Quantifier::ZeroOrMore)
        } else if input.peek(syn::token::Brace) {
            use syn::braced;
            let content;
            braced!(content in input);

            let first: syn::LitInt = content.parse()?;
            let first_val: usize = first.base10_parse()?;

            if content.is_empty() {
                // {m} - exactly m
                Ok(Quantifier::Exact(first_val))
            } else {
                // Expect comma
                content.parse::<Token![,]>()?;

                if content.is_empty() {
                    // {m,} - at least m
                    Ok(Quantifier::AtLeast(first_val))
                } else {
                    // {m,n} - between m and n
                    let second: syn::LitInt = content.parse()?;
                    let second_val: usize = second.base10_parse()?;
                    Ok(Quantifier::Range(first_val, second_val))
                }
            }
        } else {
            Err(input.error("expected quantifier: ?, +, *, or {n}, {n,}, {n,m}"))
        }
    }
}

// Token references (for matching tokenizer tokens)
enum TokenRef {
    ById { id: usize, negated: bool },           // <[1000]> or !<[1000]>
    ByString { name: String, negated: bool },    // <think> or !<think>
}

// The main expression type for rule right-hand sides
enum Expr {
    // Terminals (match input directly)
    Characters(String),
    CharacterRange(CharacterRange),
    Token(TokenRef),
    // Non-terminal (reference to another rule)
    NonTerminal(NonTerminalSymbol),
    // Combinators
    Group(Box<Expr>),
    Sequence(Vec<Expr>),
    Alternation(Vec<Expr>),
    Quantified {
        expr: Box<Expr>,
        quantifier: Quantifier,
    },
}

impl Expr {
    // Parse an atom: terminal, non-terminal, group, or token
    fn parse_atom(input: ParseStream) -> Result<Self> {
        // Check for grouped expression (...)
        if input.peek(syn::token::Paren) {
            use syn::parenthesized;
            let content;
            parenthesized!(content in input);
            let inner: Expr = content.parse()?;
            return Ok(Expr::Group(Box::new(inner)));
        }

        // Check for token !<...> or <...>
        if (input.peek(Token![!]) && input.peek2(Token![<])) || input.peek(Token![<]) {
            let negated = if input.peek(Token![!]) {
                input.parse::<Token![!]>()?;
                true
            } else {
                false
            };
            let token_ref = parse_token_ref(input, negated)?;
            return Ok(Expr::Token(token_ref));
        }

        // Try to parse as string literal
        if input.peek(syn::LitStr) {
            let lit_str: syn::LitStr = input.parse()?;
            return Ok(Expr::Characters(lit_str.value()));
        }

        // Try to parse as character range
        if input.peek(syn::token::Bracket) {
            let range: CharacterRange = input.parse()?;
            return Ok(Expr::CharacterRange(range));
        }

        // Otherwise, try to parse as non-terminal
        let non_terminal: NonTerminalSymbol = input.parse()?;
        Ok(Expr::NonTerminal(non_terminal))
    }

    // Parse a quantified atom: atom followed by optional quantifier
    fn parse_quantified(input: ParseStream) -> Result<Self> {
        let atom = Self::parse_atom(input)?;

        // Check for quantifier
        if input.peek(Token![?])
            || input.peek(Token![+])
            || input.peek(Token![*])
            || input.peek(syn::token::Brace)
        {
            let quantifier: Quantifier = input.parse()?;
            Ok(Expr::Quantified {
                expr: Box::new(atom),
                quantifier,
            })
        } else {
            Ok(atom)
        }
    }

    // Parse a sequence: one or more quantified atoms
    fn parse_sequence(input: ParseStream) -> Result<Self> {
        let mut items = vec![Self::parse_quantified(input)?];

        // Keep parsing while we see more atoms (not | or end)
        while !input.is_empty() && !input.peek(Token![|]) && !is_at_new_declaration(input) {
            items.push(Self::parse_quantified(input)?);
        }

        if items.len() == 1 {
            Ok(items.pop().unwrap())
        } else {
            Ok(Expr::Sequence(items))
        }
    }
}

impl Parse for Expr {
    // Parse alternation: sequences separated by |
    fn parse(input: ParseStream) -> Result<Self> {
        let mut alternatives = vec![Expr::parse_sequence(input)?];

        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
            alternatives.push(Expr::parse_sequence(input)?);
        }

        if alternatives.len() == 1 {
            Ok(alternatives.pop().unwrap())
        } else {
            Ok(Expr::Alternation(alternatives))
        }
    }
}

// Helper to parse token references: <[1000]> or <think>
fn parse_token_ref(input: ParseStream, negated: bool) -> Result<TokenRef> {
    input.parse::<Token![<]>()?;

    let token_ref = if input.peek(syn::token::Bracket) {
        // <[1000]> - by ID
        use syn::bracketed;
        let content;
        bracketed!(content in input);
        let id: syn::LitInt = content.parse()?;
        TokenRef::ById { id: id.base10_parse()?, negated }
    } else {
        // <think> - by string
        let ident: Ident = input.parse()?;
        TokenRef::ByString { name: ident.to_string(), negated }
    };

    input.parse::<Token![>]>()?;
    Ok(token_ref)
}

// Helper to check if we're at the start of a new declaration
fn is_at_new_declaration(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.parse::<Ident>().is_ok()
        && fork.parse::<Token![:]>().is_ok()
        && fork.parse::<Token![:]>().is_ok()
        && fork.parse::<Token![=]>().is_ok()
}

//
struct GbnfDeclaration {
    lhs: NonTerminalSymbol,
    rhs: Expr,
}

impl Parse for GbnfDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        // non terminal identifier
        let lhs: NonTerminalSymbol = input.parse()?;

        // `::=` assignment
        input.parse::<Token![:]>()?;
        input.parse::<Token![:]>()?;
        input.parse::<Token![=]>()?;

        // Parse the right-hand side expression
        let rhs: Expr = input.parse()?;

        Ok(GbnfDeclaration { lhs, rhs })
    }
}

struct GbnfInput {
    declarations: Vec<GbnfDeclaration>,
}

impl Parse for GbnfInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut declarations = Vec::new();

        while !input.is_empty() {
            declarations.push(input.parse()?);
        }

        Ok(GbnfInput { declarations })
    }
}

#[proc_macro]
pub fn gbnf(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as GbnfInput);

    // For now, just output the number of declarations parsed
    let count = parsed.declarations.len();
    let names: Vec<_> = parsed
        .declarations
        .iter()
        .map(|decl| {
            let lhs = decl.lhs.as_ident();
            quote! { stringify!(#lhs) }
        })
        .collect();

    let expanded = quote! {
        {
            println!("Parsed {} declarations: {:?}", #count, vec![#(#names),*]);
        }
    };

    TokenStream::from(expanded)
}
