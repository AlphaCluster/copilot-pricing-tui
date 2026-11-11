//! Compare GitHub Copilot model pricing in the terminal.

pub mod app;
pub mod calc;
pub mod columns;
pub mod data;
pub mod fetch;
pub mod session;
pub mod ui;

pub use app::run;
