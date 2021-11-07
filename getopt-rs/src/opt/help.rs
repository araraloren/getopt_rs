use std::mem::take;

use super::Opt;
use crate::set::CreateInfo;
use crate::Ustr;

#[derive(Debug, Clone, Default)]
pub struct HelpInfo {
    hint: Ustr,
    help: Ustr,
}

impl HelpInfo {
    pub fn new(hint: Ustr, help: Ustr) -> Self {
        Self { hint, help }
    }

    pub fn get_hint(&self) -> Ustr {
        self.hint.clone()
    }

    pub fn get_help(&self) -> Ustr {
        self.help.clone()
    }

    pub fn get_hint_mut(&mut self) -> &mut Ustr {
        &mut self.hint
    }

    pub fn get_help_mut(&mut self) -> &mut Ustr {
        &mut self.help
    }

    pub fn set_hint<T: Into<Ustr>>(&mut self, hint: T) -> &mut Self {
        self.hint = hint.into();
        self
    }

    pub fn set_help<T: Into<Ustr>>(&mut self, help: T) -> &mut Self {
        self.help = help.into();
        self
    }

    pub fn clone_and_generate_hint(&self, opt: &dyn Opt) -> Self {
        Self {
            help: self.help.clone(),
            hint: format!(
                "{}{}{}={}{}",
                if opt.get_optional() { "[" } else { "<" },
                opt.get_prefix(),
                opt.get_name(),
                opt.get_type_name(),
                if opt.get_optional() { "]" } else { ">" },
            )
            .into(),
        }
    }
}

/// Generate the help info if it not exist in [`CreateInfo`].
impl<'a> From<&'a mut CreateInfo> for HelpInfo {
    fn from(ci: &'a mut CreateInfo) -> Self {
        let mut help_info = take(ci.get_help_info_mut());
        Self {
            help: take(help_info.get_help_mut()),
            hint: if help_info.get_hint().is_empty() {
                create_help_hint(&ci)
            } else {
                take(help_info.get_hint_mut())
            },
        }
    }
}

/// Generate the help like `--Option | -O`
pub fn create_help_hint(ci: &CreateInfo) -> Ustr {
    let mut ret = String::default();

    if let Some(prefix) = ci.get_prefix() {
        ret += prefix.as_ref();
    }
    ret += ci.get_name().as_ref();
    for alias in ci.get_alias() {
        ret += &format!(" | {}", alias);
    }

    ret.into()
}
