mod arg;
mod cote;
mod sub;

use std::ops::DerefMut;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::Attribute;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Field;
use syn::Fields;
use syn::GenericArgument;
use syn::Lit;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;
use syn::TypeReference;

use crate::config::Config;
use crate::config::Configs;
use crate::config::CoteKind;
use crate::config::SubKind;

const HELP_OPTION_Q: &str = "-?";
const HELP_OPTION_SHORT: &str = "-h";
const HELP_OPTION_NAME: &str = "--help";
const HELP_OPTION_HELP: &str = "Display help message";
const HELP_OPTION_WIDTH: usize = 40;
const HELP_USAGE_WIDTH: usize = 10;
const POLICY_PRE: &str = "pre";
const POLICY_FWD: &str = "fwd";
const POLICY_DELAY: &str = "delay";
const HELP_OPTION_IDENT: &str = "help_option";
const HELP_OPTION_UID: &str = "help_option_uid";
const MAIN_OPTION_IDENT: &str = "main_option";
const MAIN_OPTION_UID: &str = "main_option_uid";

pub use self::arg::ArgGenerator;
pub use self::cote::CoteGenerator;
pub use self::sub::SubGenerator;

pub type OptUpdate = (
    Option<TokenStream>,
    Option<TokenStream>,
    Option<TokenStream>,
);

#[derive(Debug, Default)]
pub struct Update {
    pub create: Option<TokenStream>,

    pub insert: Option<TokenStream>,

    pub handler: Option<TokenStream>,
}

#[derive(Debug)]
pub struct Analyzer<'a> {
    cote_generator: CoteGenerator<'a>,

    arg_generator: Vec<ArgGenerator<'a>>,

    sub_generator: Vec<SubGenerator<'a>>,
}

impl<'a> Analyzer<'a> {
    pub fn new(input: &'a DeriveInput) -> syn::Result<Self> {
        match input.data {
            syn::Data::Struct(DataStruct {
                fields: Fields::Named(ref fields),
                ..
            }) => {
                let cote_generator = CoteGenerator::new(input)?;
                let mut arg_generator = vec![];
                let mut sub_generator = vec![];

                for field in fields.named.iter() {
                    if check_if_has_sub_cfg(field)? {
                        sub_generator.push(SubGenerator::new(field, &cote_generator)?);
                    } else {
                        arg_generator.push(ArgGenerator::new(field, &cote_generator)?);
                    }
                }
                Ok(Self {
                    arg_generator,
                    cote_generator,
                    sub_generator,
                })
            }
            _ => {
                abort! {
                    input,
                        "cote only support struct format"
                }
            }
        }
    }

    pub fn gen_impl_for_struct(&self) -> syn::Result<TokenStream> {
        let ident = self.cote_generator.get_ident();
        let where_clause = self.cote_generator.gen_where_clause(true);
        let parser_update = self.gen_parser_update()?;

        Ok(quote! {
            #[doc=concat!("Automatic generated by cote-derive for [`", stringify!(#ident), "`].")]
            impl<'zlifetime, P> cote::IntoParserDerive<'zlifetime, P> for #ident #where_clause
            {
                fn update(parser: &mut aopt::prelude::Parser<'zlifetime, P>) -> Result<(), aopt::Error> {
                    #parser_update
                }
            }
        })
    }

    pub fn gen_parser_update(&self) -> syn::Result<TokenStream> {
        let mut ret = quote! {
            let set = parser.optset_mut();
            let ctor_name = aopt::prelude::ctor_default_name();
            let ctor = set.ctor_mut(&ctor_name)?;
        };
        let mut create = vec![];
        let mut insert = vec![];
        let mut handler = vec![];
        let mut option_id = 0;

        let mut append = |(c, i, h): OptUpdate| {
            c.into_iter().for_each(|v| create.push(v));
            i.into_iter().for_each(|v| insert.push(v));
            h.into_iter().for_each(|v| handler.push(v));
        };

        if let Some(update) = self.cote_generator.gen_main_option_update(option_id) {
            append(update);
            option_id += 1;
        }
        if let Some(update) = self.cote_generator.gen_help_option_update(option_id) {
            append(update);
            option_id += 1;
        }
        for field in self.arg_generator.iter() {
            append(field.gen_option_update(option_id)?);
            option_id += 1;
        }
        for field in self.sub_generator.iter() {
            append(field.gen_option_update(option_id)?);
            option_id += 1;
        }
        ret.extend(create.into_iter());
        ret.extend(insert.into_iter());
        ret.extend(handler.into_iter());
        ret.extend(quote! { Ok(()) });
        Ok(ret)
    }
}

pub fn gen_option_ident(idx: usize, span: Span) -> Ident {
    Ident::new(&format!("option_{}", idx), span)
}

pub fn gen_option_uid_ident(idx: usize, span: Span) -> Ident {
    Ident::new(&format!("option_uid_{}", idx), span)
}

pub fn gen_elision_lifetime_ident(span: Span) -> Ident {
    Ident::new("_", span)
}

pub fn check_if_has_sub_cfg(field: &Field) -> syn::Result<bool> {
    let attrs = &field.attrs;
    let has_sub_cfg = attrs.iter().any(|v| v.path.is_ident("sub"));
    let has_arg_cfg = attrs.iter().any(|v| v.path.is_ident("arg"));

    if has_arg_cfg && has_sub_cfg {
        abort! {
            field,
            "can not have both `sub` and `arg` configuration on same field"
        }
    } else {
        Ok(has_sub_cfg)
    }
}

pub fn gen_default_policy_ty(policy_name: &str) -> Option<TokenStream> {
    match policy_name {
        POLICY_PRE => Some(quote! {
            aopt::prelude::APrePolicy
        }),
        POLICY_FWD => Some(quote! {
            aopt::prelude::AFwdPolicy
        }),
        POLICY_DELAY => Some(quote! {
            aopt::prelude::ADelayPolicy
        }),
        _ => None,
    }
}

pub fn gen_help_display_call(
    name: &TokenStream,
    global_configs: &Configs<CoteKind>,
    sub_configs: Option<&Configs<SubKind>>,
) -> TokenStream {
    let mut head = if let Some(head_cfg) = global_configs.find_cfg(CoteKind::Head) {
        let value = head_cfg.value();

        quote! {
            String::from(#value)
        }
    } else {
        quote! {
            String::from(env!("CARGO_PKG_DESCRIPTION"))
        }
    };
    let mut foot = if let Some(foot_cfg) = global_configs.find_cfg(CoteKind::Foot) {
        let value = foot_cfg.value();

        quote! {
            String::from(#value)
        }
    } else {
        quote! {
            format!("Create by {} v{}", env!("CARGO_PKG_AUTHORS"), env!("CARGO_PKG_VERSION"))
        }
    };
    if let Some(sub_configs) = sub_configs {
        if let Some(head_cfg) = sub_configs.find_cfg(SubKind::Head) {
            let value = head_cfg.value();

            head = quote! {
                String::from(#value)
            };
        }
        if let Some(foot_cfg) = sub_configs.find_cfg(SubKind::Foot) {
            let value = foot_cfg.value();

            foot = quote! {
                String::from(#value)
            };
        }
    }
    let width = if let Some(head_cfg) = global_configs.find_cfg(CoteKind::HelpWidth) {
        let value = head_cfg.value();

        quote! {
            #value
        }
    } else {
        quote! { #HELP_OPTION_WIDTH }
    };
    let usage_width = if let Some(head_cfg) = global_configs.find_cfg(CoteKind::UsageWidth) {
        let value = head_cfg.value();

        quote! {
            #value
        }
    } else {
        quote! { #HELP_USAGE_WIDTH }
    };
    let name = if sub_configs.is_none() {
        name.clone()
    } else {
        quote! {
            ser_names.join(" ")
        }
    };

    if global_configs.has_cfg(CoteKind::AbortHelp) || global_configs.has_cfg(CoteKind::Help) {
        quote! {
            cote::simple_display_set_help(parser.optset(), #name, #head, #foot, #width, #usage_width)
                        .map_err(|e| aopt::Error::raise_error(format!("Can not display help message: {:?}", e)))?;
        }
    } else {
        quote! {}
    }
}

pub fn filter_comment_doc(attrs: &[Attribute]) -> Vec<Lit> {
    let attrs = attrs.iter().filter(|v| v.path.is_ident("doc"));
    let mut ret = vec![];

    for attr in attrs {
        if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
            if let syn::Lit::Str(_) = &meta.lit {
                ret.push(meta.lit);
            }
        }
    }
    ret
}

pub fn check_in_path(ty: &Type, name: &str) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident = segment.ident.to_string();

            if ident == name {
                return true;
            } else if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                for arg in ab.args.iter() {
                    if let GenericArgument::Type(next_ty) = arg {
                        return check_in_path(next_ty, name);
                    }
                }
            }
        }
    } else if let Type::Reference(reference) = ty {
        return check_in_path(reference.elem.as_ref(), name);
    }
    false
}

pub fn gen_ty_without_option(ty: &Type) -> syn::Result<Type> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident_str = segment.ident.to_string();

            if ident_str == "Option" {
                match &segment.arguments {
                    PathArguments::AngleBracketed(ab) => {
                        if let Some(GenericArgument::Type(next_ty)) = ab.args.first().as_ref() {
                            return Ok(next_ty.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    abort! {
        ty,
        "`sub` configuration only support `Option<T>`"
    }
}

/// Change all lifetime ident to '_
pub fn gen_elision_lifetime_ty(cote_meta: &CoteGenerator, ty: &Type) -> (bool, Type) {
    let mut ty = ty.clone();
    let is_reference;

    if let Type::Reference(reference) = &mut ty {
        is_reference = true;
        remove_reference_lifetime(cote_meta, reference);
    } else {
        is_reference = is_reference_type(&ty);
        if let Type::Path(path) = &mut ty {
            remove_path_lifetime(cote_meta, path);
        }
    }
    (is_reference, ty)
}

pub fn is_reference_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                    for arg in ab.args.iter() {
                        if let GenericArgument::Type(next_ty) = arg {
                            return is_reference_type(next_ty);
                        }
                    }
                }
            }
            false
        }
        Type::Reference(_) => true,
        _ => false,
    }
}

pub fn remove_reference_lifetime(cote_meta: &CoteGenerator, ty: &mut TypeReference) {
    if let Some(lifetime) = &mut ty.lifetime {
        if cote_meta.has_lifetime_ident(&lifetime.ident) {
            lifetime.ident = gen_elision_lifetime_ident(lifetime.span().clone());
        }
    }
    match ty.elem.deref_mut() {
        Type::Path(path) => remove_path_lifetime(cote_meta, path),
        Type::Reference(tyref) => remove_reference_lifetime(cote_meta, tyref),
        _ => {
            // do nothing
        }
    }
}

pub fn remove_path_lifetime(cote_meta: &CoteGenerator, ty: &mut TypePath) {
    if let Some(segment) = ty.path.segments.last_mut() {
        if let PathArguments::AngleBracketed(ab) = &mut segment.arguments {
            for arg in ab.args.iter_mut() {
                if let GenericArgument::Type(ty) = arg {
                    match ty {
                        Type::Path(path) => remove_path_lifetime(cote_meta, path),
                        Type::Reference(tyref) => remove_reference_lifetime(cote_meta, tyref),
                        _ => {}
                    };
                } else if let GenericArgument::Lifetime(lifetime) = arg {
                    if cote_meta.has_lifetime_ident(&lifetime.ident) {
                        lifetime.ident = gen_elision_lifetime_ident(lifetime.span().clone());
                    }
                }
            }
        }
    }
}