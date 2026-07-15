//! One module per command, plus the shared output helpers they print through.

pub mod address;
pub mod blockchain;
pub mod generic;
pub mod wallet;

use std::io::IsTerminal;

/// Print a section heading, underlined when stdout is a terminal.
pub(crate) fn heading(title: &str) {
    if std::io::stdout().is_terminal() {
        println!("\n\x1b[1m{title}\x1b[0m");
    } else {
        println!("\n{title}");
    }
}

/// Print one aligned `label: value` row.
pub(crate) fn row(label: &str, value: impl std::fmt::Display) {
    println!("  {:<22} {}", format!("{label}:"), value);
}

/// Format an amount the way Bitcoin Core reports it: eight decimal places.
///
/// Core returns amounts as JSON numbers, so they arrive here as `f64` and are
/// only ever displayed. Anything that did arithmetic on balances should carry
/// them as integer satoshis instead.
pub(crate) fn btc(amount: f64) -> String {
    format!("{amount:.8} BTC")
}

/// Render a wallet name for display, since the default wallet's name is empty.
pub(crate) fn wallet_name(name: &str) -> String {
    if name.is_empty() {
        "<default>".to_owned()
    } else {
        name.to_owned()
    }
}

/// Format a difficulty the way Bitcoin Core renders it.
///
/// Regtest difficulty is around 4.7e-10, which spelled out in full is a wall of
/// leading zeroes. Core switches to scientific notation for values that small,
/// so this does too; mainnet-scale difficulties still print plainly.
pub(crate) fn difficulty(value: f64) -> String {
    if value != 0.0 && value.abs() < 1e-4 {
        format!("{value:e}")
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regtest_difficulty_uses_scientific_notation() {
        assert_eq!(difficulty(4.6565423739069247e-10), "4.6565423739069247e-10");
    }

    #[test]
    fn ordinary_difficulties_print_plainly() {
        assert_eq!(difficulty(1.0), "1");
        assert_eq!(difficulty(126411437451912.0), "126411437451912");
    }

    #[test]
    fn zero_difficulty_does_not_become_scientific() {
        assert_eq!(difficulty(0.0), "0");
    }

    #[test]
    fn the_default_wallet_gets_a_readable_name() {
        assert_eq!(wallet_name(""), "<default>");
        assert_eq!(wallet_name("alice"), "alice");
    }

    #[test]
    fn amounts_always_show_eight_decimals() {
        assert_eq!(btc(50.0), "50.00000000 BTC");
        assert_eq!(btc(0.0), "0.00000000 BTC");
    }
}
