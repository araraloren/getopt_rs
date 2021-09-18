pub mod app;
pub mod arg;
pub mod ctx;
pub mod err;
pub mod opt;
pub mod parser;
pub mod proc;
pub mod set;
pub mod uid;

pub(crate) mod pat;

#[macro_use]
extern crate tracing;

use prelude::Parser;
use prelude::Result;
use prelude::Set;

pub struct ReturnValue<'a, P: Parser>(P, &'a mut dyn Set);

pub fn getopt_impl<'a, P: Parser + Default>(
    iter: impl Iterator<Item = String>,
    sets: Vec<&'a mut dyn Set>,
    mut parsers: Vec<P>,
) -> Result<Option<ReturnValue<'a, P>>> {
    assert_eq!(sets.len(), parsers.len());

    let args: Vec<String> = iter.collect();
    let count = parsers.len();

    for (index, set) in sets.into_iter().enumerate() {
        let parser = parsers.get_mut(index).unwrap();

        match parser.parse(set, &mut args.iter().map(|v| v.clone())) {
            Ok(rv) => {
                if rv {
                    return Ok(Some(ReturnValue(std::mem::take(parser), set)));
                }
            }
            Err(e) => {
                if e.is_special() && index + 1 != count {
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
    Ok(None)
}

pub fn getopt_impl_s<'a, P: Parser + Default>(
    mut iter: impl Iterator<Item = String>,
    set: &'a mut dyn Set,
    mut parser: P,
) -> Result<Option<ReturnValue<'a, P>>> {
    if parser.parse(set, &mut iter)? {
        return Ok(Some(ReturnValue(parser, set)));
    } else {
        Ok(None)
    }
}

#[macro_export]
macro_rules! getopt {
    ($iter:expr, $set:expr, $parser:expr ) => {
        getopt_impl_s(
            $iter,
            &mut $set,
            $parser
        )
    };
    ($iter:expr, $($set:expr, $parser:expr),+ ) => {
        getopt_impl(
            $iter,
            vec![$(&mut $set, )+],
            vec![$($parser, )+]
        )
    };
}

pub mod tools {
    use crate::opt::{ArrayCreator, BoolCreator, FltCreator, IntCreator, StrCreator, UintCreator};
    use crate::opt::{CmdCreator, MainCreator, PosCreator};
    use crate::set::Set;

    pub fn initialize_creator<S: Set>(set: &mut S) {
        set.add_creator(Box::new(ArrayCreator::default()));
        set.add_creator(Box::new(BoolCreator::default()));
        set.add_creator(Box::new(FltCreator::default()));
        set.add_creator(Box::new(IntCreator::default()));
        set.add_creator(Box::new(StrCreator::default()));
        set.add_creator(Box::new(UintCreator::default()));
        set.add_creator(Box::new(CmdCreator::default()));
        set.add_creator(Box::new(MainCreator::default()));
        set.add_creator(Box::new(PosCreator::default()));
    }

    pub fn initialize_prefix<S: Set>(set: &mut S) {
        set.add_prefix(String::from("--"));
        set.add_prefix(String::from("-"));
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
}

pub mod prelude {
    pub use crate::ctx::{Context, NonOptContext, OptContext};
    pub use crate::err::{Error, Result};
    pub use crate::opt::callback::{SimpleMainCallback, SimpleMainMutCallback};
    pub use crate::opt::callback::{SimpleOptCallback, SimpleOptMutCallback};
    pub use crate::opt::callback::{SimplePosCallback, SimplePosMutCallback};
    pub use crate::opt::{
        Alias, Callback, Help, HelpInfo, Identifier, Index, Name, Opt, OptCallback, OptIndex,
        OptValue, Optional, Type, Value,
    };
    pub use crate::opt::{
        ArrayCreator, BoolCreator, FltCreator, IntCreator, StrCreator, UintCreator,
    };
    pub use crate::opt::{CmdCreator, MainCreator, PosCreator};
    pub use crate::parser::{DelayParser, Parser, PreParser, SimpleParser};
    pub use crate::proc::{Info, Proc};
    pub use crate::proc::{Matcher, NonOptMatcher, OptMatcher};
    pub use crate::set::{CreatorSet, OptionSet, PrefixSet, Set, SimpleSet};
    pub use crate::tools;
    pub use crate::uid::{Uid, UidGenerator};
    pub use crate::{getopt, getopt_impl, getopt_impl_s, ReturnValue};
    pub use crate::{simple_main_cb, simple_main_mut_cb};
    pub use crate::{simple_opt_cb, simple_opt_mut_cb};
    pub use crate::{simple_pos_cb, simple_pos_mut_cb};
}
