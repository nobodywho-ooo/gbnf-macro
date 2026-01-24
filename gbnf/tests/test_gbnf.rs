use gbnf::{gbnf, Expr, GbnfGrammar, Quantifier};

#[test]
fn test_basic_declarations() {
    let grammar: GbnfGrammar = gbnf! {
        root ::= foobar
        foobar ::= "Abekat"
        okabob ::= other
    };

    assert_eq!(grammar.declarations.len(), 3);
    assert_eq!(grammar.declarations[0].name, "root");
    assert_eq!(grammar.declarations[1].name, "foobar");
    assert_eq!(grammar.declarations[2].name, "okabob");

    // Verify GBNF string output
    assert!(grammar.as_str().contains("root ::= foobar"));
    assert!(grammar.as_str().contains("foobar ::= \"Abekat\""));
    assert!(grammar.as_str().contains("okabob ::= other"));
}

#[test]
fn test_single_declaration() {
    let grammar = gbnf! {
        single ::= value
    };

    assert_eq!(grammar.declarations.len(), 1);
    assert_eq!(grammar.declarations[0].name, "single");
    assert_eq!(grammar.as_str(), "single ::= value");
}

#[test]
fn test_alternation() {
    let grammar = gbnf! {
        rule ::= foo | bar | baz
    };

    assert_eq!(grammar.declarations.len(), 1);
    assert!(matches!(
        &grammar.declarations[0].expr,
        Expr::Alternation(alts) if alts.len() == 3
    ));
    assert_eq!(grammar.as_str(), "rule ::= foo | bar | baz");
}

#[test]
fn test_sequence() {
    let grammar = gbnf! {
        rule ::= "hello" " " "world"
    };

    assert_eq!(grammar.declarations.len(), 1);
    assert!(matches!(
        &grammar.declarations[0].expr,
        Expr::Sequence(items) if items.len() == 3
    ));
    assert_eq!(grammar.as_str(), "rule ::= \"hello\" \" \" \"world\"");
}

#[test]
fn test_quantifiers() {
    let grammar = gbnf! {
        rule ::= foo? bar+ baz*
    };

    assert_eq!(grammar.declarations.len(), 1);
    if let Expr::Sequence(items) = &grammar.declarations[0].expr {
        assert_eq!(items.len(), 3);

        // Check first item is Optional quantifier
        assert!(matches!(
            &items[0],
            Expr::Quantified { quantifier: Quantifier::Optional, .. }
        ));

        // Check second item is OneOrMore quantifier
        assert!(matches!(
            &items[1],
            Expr::Quantified { quantifier: Quantifier::OneOrMore, .. }
        ));

        // Check third item is ZeroOrMore quantifier
        assert!(matches!(
            &items[2],
            Expr::Quantified { quantifier: Quantifier::ZeroOrMore, .. }
        ));
    } else {
        panic!("Expected sequence");
    }

    assert_eq!(grammar.as_str(), "rule ::= foo? bar+ baz*");
}

#[test]
fn test_grouping() {
    let grammar = gbnf! {
        rule ::= (foo | bar) baz
    };

    assert_eq!(grammar.declarations.len(), 1);
    assert!(matches!(
        &grammar.declarations[0].expr,
        Expr::Sequence(items) if items.len() == 2
    ));
    assert_eq!(grammar.as_str(), "rule ::= (foo | bar) baz");
}

#[test]
fn test_character_range() {
    let grammar = gbnf! {
        digit ::= [0-9]
        letter ::= [a-z]
    };

    assert_eq!(grammar.declarations.len(), 2);
    assert_eq!(grammar.declarations[0].name, "digit");
    assert_eq!(grammar.declarations[1].name, "letter");

    assert!(grammar.as_str().contains("digit ::= [0-9]"));
    assert!(grammar.as_str().contains("letter ::= [a-z]"));
}

#[test]
fn test_complex_example() {
    let grammar = gbnf! {
        root ::= chessmove+
        chessmove ::= (pawn | nonpawn | castle) [a-h]?
        pawn ::= [a-h] [1-8]
        nonpawn ::= [N-R]
        castle ::= "O-O" | "O-O-O"
    };

    assert_eq!(grammar.declarations.len(), 5);

    // Verify we can get the complete GBNF string
    let gbnf_str = grammar.as_str();
    assert!(gbnf_str.contains("root ::= chessmove+"));
    assert!(gbnf_str.contains("castle ::= \"O-O\" | \"O-O-O\""));
}

#[test]
fn test_grammar_string_output() {
    let grammar = gbnf! {
        root ::= "hello" " " "world"
    };

    // The grammar should provide a valid GBNF string
    let expected = "root ::= \"hello\" \" \" \"world\"";
    assert_eq!(grammar.as_str(), expected);
    assert_eq!(grammar.gbnf_string, expected);
}

#[test]
fn test_ast_structure() {
    let grammar = gbnf! {
        greeting ::= "hello"
    };

    assert_eq!(grammar.declarations.len(), 1);
    let decl = &grammar.declarations[0];
    assert_eq!(decl.name, "greeting");

    if let Expr::Characters(s) = &decl.expr {
        assert_eq!(s, "hello");
    } else {
        panic!("Expected Characters expression");
    }
}
