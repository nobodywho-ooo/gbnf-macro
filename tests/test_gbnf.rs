use gbnf_macro::gbnf;

#[test]
fn test_basic_declarations() {
    gbnf! {
        root ::= foobar
        foobar ::= "Abekat"
        okabob ::= other
    }
}

#[test]
fn test_single_declaration() {
    gbnf! {
        single ::= value
    }
}

#[test]
fn test_alternation() {
    gbnf! {
        rule ::= foo | bar | baz
    }
}

#[test]
fn test_sequence() {
    gbnf! {
        rule ::= "hello" " " "world"
    }
}

#[test]
fn test_quantifiers() {
    gbnf! {
        rule ::= foo? bar+ baz*
    }
}

#[test]
fn test_grouping() {
    gbnf! {
        rule ::= (foo | bar) baz
    }
}

#[test]
fn test_character_range() {
    gbnf! {
        digit ::= [0-9]
        letter ::= [a-z]
    }
}

#[test]
fn test_complex_example() {
    gbnf! {
        root ::= chessmove+
        chessmove ::= (pawn | nonpawn | castle) [a-h]?
        pawn ::= [a-h] [1-8]
        nonpawn ::= [N-R]
        castle ::= "O-O" | "O-O-O"
    }
}
