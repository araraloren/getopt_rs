use std::mem::take;

use super::NonOpt;
use crate::opt::*;
use crate::set::CreateInfo;
use crate::set::Creator;
use crate::uid::Uid;

pub fn current_type() -> &'static str {
    "m"
}

pub trait Main: NonOpt {}

#[derive(Debug)]
pub struct MainOpt {
    uid: Uid,

    name: String,

    value: OptValue,

    need_invoke: bool,

    help_info: HelpInfo,
}

impl From<CreateInfo> for MainOpt {
    fn from(ci: CreateInfo) -> Self {
        let mut ci = ci;
        let help_info = HelpInfo::from(&mut ci);

        Self {
            uid: ci.get_uid(),
            name: take(ci.get_name_mut()),
            value: OptValue::Null,
            need_invoke: false,
            help_info,
        }
    }
}

impl Main for MainOpt {}

impl Opt for MainOpt {}

impl NonOpt for MainOpt {}

impl Type for MainOpt {
    fn get_type_name(&self) -> &'static str {
        current_type()
    }

    fn is_deactivate_style(&self) -> bool {
        false
    }

    fn match_style(&self, style: Style) -> bool {
        match style {
            Style::Main => true,
            _ => false,
        }
    }

    fn check(&self) -> Result<bool> {
        if !(self.get_optional() || self.has_value()) {
            Err(Error::ForceRequiredOption(self.get_hint().to_owned()))
        } else {
            Ok(true)
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Identifier for MainOpt {
    fn get_uid(&self) -> Uid {
        self.uid
    }

    fn set_uid(&mut self, uid: Uid) {
        self.uid = uid;
    }
}

impl Callback for MainOpt {
    fn is_need_invoke(&self) -> bool {
        self.need_invoke
    }

    fn set_invoke(&mut self, invoke: bool) {
        self.need_invoke = invoke;
    }

    fn is_accept_callback_type(&self, callback_type: CallbackType) -> bool {
        match callback_type {
            CallbackType::Main | CallbackType::MainMut => true,
            _ => false,
        }
    }

    fn set_callback_ret(&mut self, ret: Option<OptValue>) -> Result<()> {
        if let Some(ret) = ret {
            self.set_value(ret);
        }
        Ok(())
    }
}

impl Name for MainOpt {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_prefix(&self) -> &str {
        ""
    }

    fn set_name(&mut self, string: String) {
        self.name = string;
    }

    fn set_prefix(&mut self, _string: String) {}

    fn match_name(&self, _name: &str) -> bool {
        true
    }

    fn match_prefix(&self, _prefix: &str) -> bool {
        false
    }
}

impl Optional for MainOpt {
    fn get_optional(&self) -> bool {
        true
    }

    fn set_optional(&mut self, _optional: bool) {}

    fn match_optional(&self, optional: bool) -> bool {
        self.get_optional() == optional
    }
}

impl Alias for MainOpt {
    fn get_alias(&self) -> Option<&Vec<(String, String)>> {
        None
    }

    fn add_alias(&mut self, _prefix: String, _name: String) {}

    fn rem_alias(&mut self, _prefix: &str, _name: &str) {}

    fn match_alias(&self, _prefix: &str, _name: &str) -> bool {
        false
    }
}

impl Index for MainOpt {
    fn get_index(&self) -> Option<&OptIndex> {
        None
    }

    fn set_index(&mut self, _: OptIndex) {}

    fn match_index(&self, _total: u64, _current: u64) -> bool {
        true
    }
}

impl Value for MainOpt {
    fn get_value(&self) -> &OptValue {
        &self.value
    }

    fn get_default_value(&self) -> &OptValue {
        &OptValue::Null
    }

    fn set_value(&mut self, value: OptValue) {
        self.value = value;
    }

    fn set_default_value(&mut self, _value: OptValue) {}

    fn parse_value(&self, _string: &str) -> Result<OptValue> {
        Ok(OptValue::from(true))
    }

    fn has_value(&self) -> bool {
        !self.get_value().is_null()
    }

    fn reset_value(&mut self) {
        self.value = self.get_default_value().clone();
    }
}

impl Help for MainOpt {
    fn set_hint(&mut self, hint: String) {
        self.help_info.set_hint(hint);
    }

    fn set_help(&mut self, help: String) {
        self.help_info.set_help(help);
    }

    fn get_help_info(&self) -> &HelpInfo {
        &self.help_info
    }
}

#[derive(Debug, Default, Clone)]
pub struct MainCreator;

impl Creator for MainCreator {
    fn get_type_name(&self) -> &'static str {
        current_type()
    }

    fn is_support_deactivate_style(&self) -> bool {
        false
    }

    fn create_with(&self, create_info: CreateInfo) -> Result<Box<dyn Opt>> {
        if create_info.get_support_deactivate_style() {
            if !self.is_support_deactivate_style() {
                return Err(Error::NotSupportDeactivateStyle(
                    create_info.get_name().to_owned(),
                ));
            }
        }

        assert_eq!(create_info.get_type_name(), self.get_type_name());

        let opt: MainOpt = create_info.into();

        Ok(Box::new(opt))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn make_type_main_work() {
        let creator = MainCreator::default();

        assert_eq!(creator.get_type_name(), current_type());
        // main not support deactivate style
        assert_eq!(creator.is_support_deactivate_style(), false);

        let mut ci = CreateInfo::parse("main=m", &[]).unwrap();

        ci.set_uid(1);

        let mut main = creator.create_with(ci).unwrap();

        assert_eq!(main.get_type_name(), current_type());
        assert_eq!(main.is_deactivate_style(), false);
        assert_eq!(main.match_style(Style::Main), true);
        assert_eq!(main.check().is_err(), false);

        assert_eq!(main.get_uid(), 1);
        main.set_uid(42);
        assert_eq!(main.get_uid(), 42);

        assert_eq!(main.is_need_invoke(), false);
        main.set_invoke(true);
        assert_eq!(main.is_need_invoke(), true);
        assert_eq!(main.is_accept_callback_type(CallbackType::Main), true);
        assert_eq!(main.is_accept_callback_type(CallbackType::MainMut), true);

        // main not support alias
        main.add_alias("-".to_owned(), "m".to_owned());
        assert_eq!(main.get_alias(), None);
        assert_eq!(main.match_alias("-", "m"), false);
        main.rem_alias("-", "m");
        assert_eq!(main.get_alias(), None);

        assert_eq!(main.get_index(), None);
        assert_eq!(main.match_index(6, 1), true);
        assert_eq!(main.match_index(6, 3), true);
        main.set_index(OptIndex::forward(1));
        assert_eq!(main.get_index(), None);
        assert_eq!(main.match_index(6, 9), true);

        assert_eq!(main.get_name(), "main");
        assert_eq!(main.get_prefix(), "");
        assert_eq!(main.match_name("www"), true);
        assert_eq!(main.match_name("main"), true);
        assert_eq!(main.match_prefix("--"), false);
        assert_eq!(main.match_prefix(""), false);
        main.set_name(String::from("main1"));
        main.set_prefix(String::from("+"));
        assert_eq!(main.match_name("www"), true);
        assert_eq!(main.match_name("main1"), true);
        assert_eq!(main.get_name(), "main1");
        assert_eq!(main.match_prefix("+"), false);
        assert_eq!(main.match_prefix(""), false);

        assert_eq!(main.get_optional(), true);
        assert_eq!(main.match_optional(true), true);
        main.set_optional(false);
        assert_eq!(main.get_optional(), true);
        assert_eq!(main.match_optional(true), true);
        assert_eq!(main.check().is_err(), false);

        assert_eq!(main.get_value().is_null(), true);
        assert_eq!(main.get_default_value().is_null(), true);
        assert_eq!(main.has_value(), false);
        let value = main.parse_value("");
        assert_eq!(value.is_ok(), true);
        let value = value.unwrap();
        assert_eq!(value.is_bool(), true);
        main.set_value(value);
        assert_eq!(main.get_value().as_bool(), OptValue::from(true).as_bool());
        main.set_default_value(OptValue::from(false));
        assert_eq!(main.get_default_value().is_null(), true);
        main.reset_value();
        assert_eq!(main.get_value().is_null(), true);

        assert_eq!(main.as_ref().as_any().is::<MainOpt>(), true);
    }
}