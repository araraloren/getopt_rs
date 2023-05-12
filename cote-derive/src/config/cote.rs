use proc_macro2::Ident;

use super::Kind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoteKind {
    Policy,

    Name,

    Hint,

    Help,

    Head,

    Foot,

    HelpWidth,

    UsageWidth,

    AbortHelp,

    Ref,

    Mut,

    On,

    Fallback,

    Then,

    Strict,

    Combine,

    EmbeddedPlus,

    Flag,

    RawCall(String),
}

impl Kind for CoteKind {
    fn parse(input: &mut syn::parse::ParseStream) -> syn::Result<(Self, bool)> {
        let ident: Ident = input.parse()?;
        let kind_str = ident.to_string();

        Ok(match kind_str.as_str() {
            "policy" => (Self::Policy, true),
            "name" => (Self::Name, true),
            "hint" => (Self::Hint, true),
            "help" => (Self::Help, false),
            "head" => (Self::Head, true),
            "foot" => (Self::Foot, true),
            "width" => (Self::HelpWidth, true),
            "usagew" => (Self::UsageWidth, true),
            "aborthelp" => (Self::AbortHelp, false),
            "refopt" => (Self::Ref, false),
            "mutopt" => (Self::Mut, false),
            "on" => (Self::On, true),
            "fallback" => (Self::Fallback, true),
            "then" => (Self::Then, true),
            "strict" => (Self::Strict, true),
            "combine" => (Self::Combine, false),
            "embedded" => (Self::EmbeddedPlus, false),
            "flag" => (Self::Flag, false),
            call => (Self::RawCall(call.to_owned()), true),
        })
    }
}
