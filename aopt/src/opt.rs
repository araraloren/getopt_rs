mod help;
mod index;
mod parser;
mod style;
mod value;

pub mod nonopt;
pub mod opt;

use std::fmt::Debug;
use ustr::Ustr;

use crate::err::Result;
use crate::uid::Uid;

cfg_if::cfg_if! {
    if #[cfg(feature = "sync")] {
        mod callback_sync;
        pub use self::callback_sync::Callback as OptCallback;
        pub use self::callback_sync::CallbackType;
        pub use self::callback_sync::MainCallback as MainFn;
        pub use self::callback_sync::MainMutCallback as MainFnMut;
        pub use self::callback_sync::OptCallback as OptFn;
        pub use self::callback_sync::OptMutCallback as OptFnMut;
        pub use self::callback_sync::PosCallback as PosFn;
        pub use self::callback_sync::PosMutCallback as PosFnMut;
        pub use self::callback_sync::SimpleMainCallback;
        pub use self::callback_sync::SimpleMainMutCallback;
        pub use self::callback_sync::SimpleOptCallback;
        pub use self::callback_sync::SimpleOptMutCallback;
        pub use self::callback_sync::SimplePosCallback;
        pub use self::callback_sync::SimplePosMutCallback;
    }
    else {
        mod callback;
        pub use self::callback::Callback as OptCallback;
        pub use self::callback::CallbackType;
        pub use self::callback::MainCallback as MainFn;
        pub use self::callback::MainMutCallback as MainFnMut;
        pub use self::callback::OptCallback as OptFn;
        pub use self::callback::OptMutCallback as OptFnMut;
        pub use self::callback::PosCallback as PosFn;
        pub use self::callback::PosMutCallback as PosFnMut;
        pub use self::callback::SimpleMainCallback;
        pub use self::callback::SimpleMainMutCallback;
        pub use self::callback::SimpleOptCallback;
        pub use self::callback::SimpleOptMutCallback;
        pub use self::callback::SimplePosCallback;
        pub use self::callback::SimplePosMutCallback;
    }
}

pub use self::help::create_help_hint;
pub use self::help::HelpInfo;
pub use self::index::Index as OptIndex;
pub use self::nonopt::CmdCreator;
pub use self::nonopt::MainCreator;
pub use self::nonopt::NonOpt;
pub use self::nonopt::PosCreator;
pub use self::opt::ArrayCreator;
pub use self::opt::BoolCreator;
pub use self::opt::FltCreator;
pub use self::opt::IntCreator;
pub use self::opt::StrCreator;
pub use self::opt::UintCreator;
pub use self::parser::parse_option_str;
pub use self::parser::DataKeeper;
pub use self::style::Style;
pub use self::value::CloneHelper;
pub use self::value::Value as OptValue;

/// The Type trait of option.
pub trait Type {
    /// Get the unique type name string of option type.
    fn get_type_name(&self) -> Ustr;

    /// Indicate if the option support deactivate style such as `--/boolean`.
    /// In defult is false.
    fn is_deactivate_style(&self) -> bool {
        false
    }

    /// Check if the option type support given style.
    fn match_style(&self, style: style::Style) -> bool;

    /// It will be called by [`Parser`](crate::parser::Parser) check the option validity.
    fn check(&self) -> Result<()>;

    fn as_any(&self) -> &dyn std::any::Any;
}

/// The Identifier trait of option.
pub trait Identifier {
    /// Get the unique identifier of current option.
    fn get_uid(&self) -> Uid;

    /// Set the unique identifier of current option.
    fn set_uid(&mut self, uid: Uid);
}

/// The Callback trait of option.
pub trait Callback {
    /// Check if we need invoke the callback of current option.
    fn is_need_invoke(&self) -> bool;

    /// The [`Context`](crate::ctx::Context) will set the value to true if user set an invalid value.
    fn set_invoke(&mut self, invoke: bool);

    /// Check if the option support given callback type.
    fn is_accept_callback_type(&self, callback_type: CallbackType) -> bool;

    /// Set the callback return value to option.
    fn set_callback_ret(&mut self, ret: Option<OptValue>) -> Result<()>;
}

/// The Name trait of option.
pub trait Name {
    /// Get the name of current option.
    fn get_name(&self) -> Ustr;

    /// Get the prefix of current option.
    fn get_prefix(&self) -> Ustr;

    /// Set the name of current option.
    fn set_name(&mut self, string: Ustr);

    /// Set the prefix of current option.
    fn set_prefix(&mut self, string: Ustr);

    /// Check if the option matched given name.
    fn match_name(&self, name: Ustr) -> bool;

    /// Check if the option matched given prefix.
    fn match_prefix(&self, prefix: Ustr) -> bool;
}

/// The Alias trait of option.
pub trait Alias {
    /// Get all the alias of current option.
    fn get_alias(&self) -> Option<&Vec<(Ustr, Ustr)>>;

    /// Add an alias to current option.
    fn add_alias(&mut self, prefix: Ustr, name: Ustr);

    /// Remove an alias of current option.
    fn rem_alias(&mut self, prefix: Ustr, name: Ustr);

    /// Check if any alias of the option matched given prefix and name.
    fn match_alias(&self, prefix: Ustr, name: Ustr) -> bool;
}

/// The Optional trait of option.
pub trait Optional {
    /// Get if the option is optional.
    fn get_optional(&self) -> bool;

    /// Set if the option is optional.
    fn set_optional(&mut self, optional: bool);

    /// Check if the option matched given optional value.
    fn match_optional(&self, optional: bool) -> bool;
}

/// The Value trait of option.
pub trait Value {
    /// Get value reference of current option.
    fn get_value(&self) -> &OptValue;

    /// Get mutable value reference of current option.
    fn get_value_mut(&mut self) -> &mut OptValue;

    /// Get default value reference of current option.
    fn get_default_value(&self) -> &OptValue;

    /// Set value of current option.
    fn set_value(&mut self, value: OptValue);

    /// Get default value of current option.
    fn set_default_value(&mut self, value: OptValue);

    /// Parse command line item and return an [`OptValue`].
    fn parse_value(&self, string: Ustr) -> Result<OptValue>;

    /// Check if the option has a valid value.
    fn has_value(&self) -> bool;

    /// Reset the value to default value.
    fn reset_value(&mut self);
}

/// The Index trait of option.
pub trait Index {
    /// Get the index of current option.
    fn get_index(&self) -> Option<&OptIndex>;

    /// Set the index of current option.
    fn set_index(&mut self, index: OptIndex);

    /// Check if current option matched given [`NonOpt`](crate::opt::NonOpt) position.
    fn match_index(&self, total: u64, current: u64) -> bool;
}

/// The Help trait of option.
pub trait Help {
    /// Set the hint of current option.
    fn set_hint(&mut self, hint: Ustr);

    /// Set the help message of current option.
    fn set_help(&mut self, help: Ustr);

    /// Get the hint of current option.
    fn get_hint(&self) -> Ustr {
        self.get_help_info().get_hint()
    }

    /// Get the help message of current option.
    fn get_help(&self) -> Ustr {
        self.get_help_info().get_help()
    }

    /// Get help information of current option.
    fn get_help_info(&self) -> &HelpInfo;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "sync")] {
        /// The option trait.
        pub trait Opt:
        Type + Identifier + Name + Callback + Alias + Optional + Value + Index + Help + Debug + Send + Sync
        { }
    }
    else {
        /// The option trait.
        pub trait Opt:
        Type + Identifier + Name + Callback + Alias + Optional + Value + Index + Help + Debug
        { }
    }
}

#[macro_export]
macro_rules! simple_main_cb {
    ($block:expr) => {
        OptCallback::Main(Box::new(SimpleMainCallback::new($block)))
    };
}

#[macro_export]
macro_rules! simple_main_mut_cb {
    ($block:expr) => {
        OptCallback::MainMut(Box::new(SimpleMainMutCallback::new($block)))
    };
}

#[macro_export]
macro_rules! simple_pos_cb {
    ($block:expr) => {
        OptCallback::Pos(Box::new(SimplePosCallback::new($block)))
    };
}

#[macro_export]
macro_rules! simple_pos_mut_cb {
    ($block:expr) => {
        OptCallback::PosMut(Box::new(SimplePosMutCallback::new($block)))
    };
}

#[macro_export]
macro_rules! simple_opt_cb {
    ($block:expr) => {
        OptCallback::Opt(Box::new(SimpleOptCallback::new($block)))
    };
}

#[macro_export]
macro_rules! simple_opt_mut_cb {
    ($block:expr) => {
        OptCallback::OptMut(Box::new(SimpleOptMutCallback::new($block)))
    };
}
