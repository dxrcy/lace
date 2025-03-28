use std::fmt;

use super::{integer::Radix, label};

/// 'Guessed' type of an argument.
///
/// An argument having a naive type of [`NaiveType::Integer`] does not necessarily mean it is a
/// valid integer, only that it is guaranteed to be no other type (i.e. it cannot be a register,
/// label, or pc offset).
///
/// Eg. `NaiveType::try_from("12a")` will return `Some(NaiveType::Integer)`, because a token
/// starting with a decimal digit cannot be any other type.
///
/// The following shows the patterns against which a string is checked in [`NaiveType::try_from`].
///
/// - [`NaiveType::PCOffset`]:
///   * `^\^`
/// - [`NaiveType::Register`]:
///   * `^[rR][0-7]$`
/// - [`NaiveType::Integer`]:
///   * `^[-+#0-9]`
///   * or `^[bB][-+]?[01]+$`
///   * or `^[oO][-+]?[0-7]+$`
///   * or `^[xX][-+]?[0-9a-fA-F]+$`
/// - [`NaiveType::Label`]:
///   * `^[a-zA-Z_]
///
/// [`NaiveType::try_from`] returning `None` indicates that the argument is likely invalid in all
/// contexts. However this diagnosis must not prevent the caller attempting to parse the argument
/// truly, in the case that the preliminary check is incorrect or outdated. This also allows the
/// caller to return a more suitable error message than the misleading 'mismatched type' for
/// an invalid token.
#[derive(Debug, PartialEq)]
pub enum NaiveType {
    Integer,
    Register,
    Label,
    PCOffset,
}

impl TryFrom<&str> for NaiveType {
    type Error = ();
    fn try_from(string: &str) -> Result<Self, Self::Error> {
        // Ensure that this order matches the documentation at the `NaiveType` definition
        // Do not change order unless there is a good reason
        if Self::is_str_pc_offset(string) {
            return Ok(Self::PCOffset);
        }
        if Self::is_str_register(string) {
            return Ok(Self::Register);
        }
        // `Integer` must be checked before `Label` to handle prefixes without preceeding zero
        if Self::is_str_integer(string) {
            return Ok(Self::Integer);
        }
        if Self::is_str_label(string) {
            return Ok(Self::Label);
        }
        Err(())
    }
}

impl NaiveType {
    /// If string does not start with `[-+#0-9]`, then all characters need to be checked, to ensure
    /// labels starting with `[bBoOxX]` don't get classified as integers.
    ///
    /// See [`NaiveType`] for allowed patterns.
    fn is_str_integer(string: &str) -> bool {
        let mut chars = string.chars().peekable();
        if chars
            .peek()
            .is_some_and(|ch| matches!(ch, '-' | '+' | '#' | '0'..='9'))
        {
            return true;
        }
        let radix = match chars.next() {
            Some('b' | 'B') => Radix::Binary,
            Some('o' | 'O') => Radix::Octal,
            Some('x' | 'X') => Radix::Hex,
            _ => return false,
        };
        chars.next_if(|ch| matches!(ch, '-' | '+'));
        if chars.peek().is_none() {
            return false;
        }
        for ch in chars {
            if radix.parse_digit(ch).is_none() {
                return false;
            }
        }
        true
    }

    /// See [`NaiveType`] for allowed patterns.
    fn is_str_register(string: &str) -> bool {
        let mut chars = string.chars();
        chars.next().is_some_and(|ch| matches!(ch, 'r' | 'R'))
            && chars.next().is_some_and(|ch| matches!(ch, '0'..='7'))
            && !chars.next().is_some_and(label::can_contain)
    }

    /// See [`NaiveType`] for allowed patterns.
    fn is_str_label(string: &str) -> bool {
        string.chars().next().is_some_and(label::can_start_with)
    }

    /// See [`NaiveType`] for allowed patterns.
    fn is_str_pc_offset(string: &str) -> bool {
        string.chars().next().is_some_and(|ch| ch == '^')
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            NaiveType::Integer => "integer",
            NaiveType::Register => "register",
            NaiveType::Label => "label",
            NaiveType::PCOffset => "PC offset",
        }
    }
}

impl fmt::Display for NaiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from() {
        fn expect_naive_type(input: &str, expected: Option<NaiveType>) {
            println!("{:?}", input);
            let result = NaiveType::try_from(input).ok();
            assert_eq!(result, expected);
        }

        expect_naive_type("", None);
        expect_naive_type("!@#", None);
        expect_naive_type("!@#", None);

        expect_naive_type("r0", Some(NaiveType::Register));
        expect_naive_type("R6", Some(NaiveType::Register));
        expect_naive_type("R7", Some(NaiveType::Register));

        expect_naive_type("123", Some(NaiveType::Integer));
        expect_naive_type("#1", Some(NaiveType::Integer));
        expect_naive_type("0xdeAD", Some(NaiveType::Integer));
        expect_naive_type("+o17", Some(NaiveType::Integer));
        expect_naive_type("xaf", Some(NaiveType::Integer));
        expect_naive_type("-0x1283", Some(NaiveType::Integer));
        expect_naive_type("12312031283", Some(NaiveType::Integer));

        expect_naive_type("r8", Some(NaiveType::Label));
        expect_naive_type("xag", Some(NaiveType::Label));
        expect_naive_type("foo+1", Some(NaiveType::Label));
        expect_naive_type("foo-0x01", Some(NaiveType::Label));
        expect_naive_type("ra$", Some(NaiveType::Label));
        expect_naive_type("a!", Some(NaiveType::Label));
        expect_naive_type("a--23", Some(NaiveType::Label));
        expect_naive_type("a-x-23", Some(NaiveType::Label));

        expect_naive_type("^-", Some(NaiveType::PCOffset));
        expect_naive_type("^a", Some(NaiveType::PCOffset));
        expect_naive_type("^", Some(NaiveType::PCOffset));
        expect_naive_type("^0x7ffF", Some(NaiveType::PCOffset));
        expect_naive_type("^+#19", Some(NaiveType::PCOffset));
    }
}
