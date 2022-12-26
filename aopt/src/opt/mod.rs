pub(crate) mod action;
pub(crate) mod aopt;
pub(crate) mod config;
pub(crate) mod creator;
pub(crate) mod help;
pub(crate) mod index;
pub(crate) mod info;
pub(crate) mod initiator;
pub(crate) mod parser;
pub(crate) mod serde;
pub(crate) mod style;
pub(crate) mod valid;
pub(crate) mod value;

pub use self::action::Action;
pub use self::action::Assoc;
pub use self::aopt::AOpt;
pub use self::config::Config;
pub use self::config::ConfigValue;
pub use self::config::OptConfig;
pub use self::creator::Creator;
pub use self::help::Help;
pub use self::index::Index;
pub use self::info::ConstrctInfo;
pub use self::info::Information;
pub use self::initiator::ValInitialize;
pub use self::initiator::ValInitiator;
pub use self::parser::StrParser;
pub use self::serde::Deserialize;
pub use self::serde::Serde;
pub use self::serde::Serialize;
pub use self::style::Style;
pub use self::valid::RawValValidator;
pub use self::valid::ValValidator;
pub use self::valid::ValValidatorExt;
pub use self::valid::ValValidatorExt2;
pub use self::value::RawValParser;

use std::fmt::Debug;

use crate::ser::Services;
use crate::Error;
use crate::RawVal;
use crate::Str;
use crate::Uid;

pub const BOOL_TRUE: &str = "true";

pub const BOOL_FALSE: &str = "false";

/// Option parser using for parsing option constructor string.
pub trait OptParser {
    type Output;
    type Error: Into<Error>;

    fn parse(&self, pattern: Str) -> Result<Self::Output, Self::Error>;
}

pub trait Opt: Debug {
    fn reset(&mut self);

    fn uid(&self) -> Uid;

    /// The name of option.
    fn name(&self) -> &Str;

    fn r#type(&self) -> Str;

    fn hint(&self) -> &Str;

    fn help(&self) -> &Str;

    fn valid(&self) -> bool;

    fn setted(&self) -> bool;

    /// If the option is optional.
    fn force(&self) -> bool;

    fn assoc(&self) -> &Assoc;

    fn action(&self) -> &Action;

    /// The index of option.
    fn idx(&self) -> Option<&Index>;

    /// The alias the option.
    fn alias(&self) -> Option<&Vec<Str>>;

    fn set_uid(&mut self, uid: Uid);

    fn set_setted(&mut self, setted: bool);

    fn mat_style(&self, style: Style) -> bool;

    fn mat_force(&self, force: bool) -> bool;

    fn mat_name(&self, name: Option<&Str>) -> bool;

    fn mat_alias(&self, name: &Str) -> bool;

    fn mat_idx(&self, index: Option<(usize, usize)>) -> bool;

    fn init(&mut self, ser: &mut Services) -> Result<(), Error>;

    fn check_val(&mut self, val: Option<&RawVal>, index: (usize, usize)) -> Result<bool, Error>;
}
