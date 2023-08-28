use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::DeriveInput;
use syn::GenericParam;
use syn::Generics;
use syn::Token;
use syn::WherePredicate;

use crate::config::Configs;
use crate::config::CoteKind;
use crate::error;

use super::gen_option_ident;
use super::gen_option_uid_ident;
use super::gen_ret_default_policy_ty;
use super::gen_ret_policy_ty_generics;
use super::OptUpdate;
use super::APP_POSTFIX;
use super::HELP_OPTION_HELP;
use super::HELP_OPTION_NAME;
use super::HELP_OPTION_SHORT;
use super::POLICY_FWD;
use super::POLICY_PRE;

#[derive(Debug)]
pub struct CoteGenerator<'a> {
    name: TokenStream,

    ident: &'a Ident,

    configs: Configs<CoteKind>,

    generics: &'a Generics,

    has_sub_command: bool,
}

impl<'a> CoteGenerator<'a> {
    pub fn new(input: &'a DeriveInput) -> syn::Result<Self> {
        let ident = &input.ident;
        let generics = &input.generics;
        let params = &generics.params;
        let configs = Configs::<CoteKind>::parse_attrs("cote", &input.attrs);
        let name = if let Some(cfg) = configs.find_cfg(CoteKind::Name) {
            let value = cfg.value();

            quote! {
                String::from(#value)
            }
        } else {
            quote! {
                String::from(env!("CARGO_PKG_NAME"))
            }
        };
        // Check the lifetime in type parameters
        for param in params {
            match param {
                GenericParam::Type(_) => {}
                GenericParam::Lifetime(lifetime) => {
                    return error(
                        input.span(),
                        format!(
                            "Cote not support struct with lifetime `{}`",
                            lifetime.to_token_stream()
                        ),
                    )
                }
                GenericParam::Const(const_param) => {
                    return error(
                        input.span(),
                        format!(
                            "Parsing struct failed: Cote not support const parameter `{:?}`",
                            const_param
                        ),
                    )
                }
            }
        }

        Ok(Self {
            name,
            ident,
            configs,
            generics,
            has_sub_command: false,
        })
    }

    pub fn get_generics_params(
        &self,
    ) -> (
        &Punctuated<GenericParam, Token![,]>,
        Option<&Punctuated<WherePredicate, Token![,]>>,
    ) {
        let params = &self.generics.params;
        let where_predicate = self.generics.where_clause.as_ref().map(|v| &v.predicates);

        (params, where_predicate)
    }

    pub fn set_has_sub_command(&mut self, sub_command: bool) -> &mut Self {
        self.has_sub_command = sub_command;
        self
    }

    pub fn has_sub_command(&self) -> bool {
        self.has_sub_command
    }

    pub fn is_process_help(&self) -> bool {
        self.configs.has_cfg(CoteKind::Help) || self.configs.has_cfg(CoteKind::AbortHelp)
    }

    pub fn get_ident(&self) -> &Ident {
        self.ident
    }

    pub fn get_name(&self) -> &TokenStream {
        &self.name
    }

    pub fn define_helper_ty(&self, ident: &Ident) -> TokenStream {
        quote! {
            #[doc=concat!("Automatic generated by cote-derive for [`", stringify!(#ident), "`].")]
            #[derive(Debug)]
            pub struct #ident<'a, Parser, Policy> {
                pub parser: Option<&'a mut Parser>,
                pub policy: Option<&'a mut Policy>,
            }

            impl<'a, Parser, Policy> std::default::Default for #ident<'a, Parser, Policy> {
                fn default() -> Self {
                    Self {
                        parser: None,
                        policy: None,
                    }
                }
            }

            impl<'a, Parser, Policy> std::ops::Deref for #ident<'a, Parser, Policy> {
                type Target = Parser;

                fn deref(&self) -> &Self::Target {
                    self.inner_parser()
                }
            }

            impl<'a, Parser, Policy> std::ops::DerefMut for #ident<'a, Parser, Policy> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.inner_parser_mut()
                }
            }

            impl<'a, Parser, Policy> #ident<'a, Parser, Policy> {
                pub fn set_inner_parser(&mut self, parser: &'a mut Parser) {
                    self.parser = Some(parser);
                }

                pub fn set_inner_policy(&mut self, policy: &'a mut Policy) {
                    self.policy = Some(policy);
                }

                pub fn clr_inner_parser(&mut self) {
                    self.parser = None;
                }

                pub fn clr_inner_policy(&mut self) {
                    self.policy = None;
                }

                pub fn inner_parser(&self) -> &Parser {
                    if self.parser.is_none() {
                        panic!("Got error here, should set parser for {} before using", stringify!(#ident))
                    }
                    self.parser.as_ref().unwrap()
                }

                pub fn inner_parser_mut(&mut self) -> &mut Parser {
                    if self.parser.is_none() {
                        panic!("Got error here, should set parser for {} before using", stringify!(#ident))
                    }
                    self.parser.as_mut().unwrap()
                }

                pub fn inner_policy(&self) -> &Policy {
                    if self.policy.is_none() {
                        panic!("Got error here, should set policy for {} before using", stringify!(#ident))
                    }
                    self.policy.as_ref().unwrap()
                }

                pub fn inner_policy_mut(&mut self) -> &mut Policy {
                    if self.policy.is_none() {
                        panic!("Got error here, should set policy for {} before using", stringify!(#ident))
                    }
                    self.policy.as_mut().unwrap()
                }
            }
        }
    }

    pub fn gen_internal_ty(&self) -> Ident {
        let ident = self.ident;

        Ident::new(&format!("{}{}", ident, APP_POSTFIX), ident.span())
    }

    pub fn policy_settings_modifier(&self) -> Option<TokenStream> {
        let has_combine = self.configs.has_cfg(CoteKind::Combine);
        let has_embedded = self.configs.has_cfg(CoteKind::EmbeddedPlus);
        let has_flag = self.configs.has_cfg(CoteKind::Flag);
        let has_overload = self.configs.has_cfg(CoteKind::Overload);
        let for_combine =
            has_combine.then_some(quote! { style_manager.push(cote::UserStyle::CombinedOption);});
        let for_embedded_plus = has_embedded
            .then_some(quote! { style_manager.push(cote::UserStyle::EmbeddedValuePlus);});
        let for_flag = has_flag.then_some(quote! { style_manager.push(cote::UserStyle::Flag); });
        let for_overload =
            has_overload.then_some(quote! { cote::PolicySettings::set_overload(policy, true); });
        let for_strict = self.configs.find_cfg(CoteKind::Strict).map(|v| {
            let value = v.value();
            quote! {
                cote::PolicySettings::set_strict(policy, #value);
            }
        });

        if for_combine.is_none()
            && for_embedded_plus.is_none()
            && for_strict.is_none()
            && for_flag.is_none()
            && for_overload.is_none()
        {
            None
        } else {
            let mut ret = quote! {};

            ret.extend(for_combine.into_iter());
            ret.extend(for_embedded_plus.into_iter());
            ret.extend(for_flag.into_iter());
            ret.extend(for_strict.into_iter());
            ret.extend(for_overload.into_iter());
            Some(ret)
        }
    }

    pub fn gen_method_call(&self) -> syn::Result<TokenStream> {
        let mut ret = quote! {};

        for config in self.configs.iter() {
            if let CoteKind::MethodCall(method) = config.kind() {
                let method = Ident::new(method, self.ident.span());
                let value = config.value().clone();
                let (var, args) = value.split_call_args(self.ident.span())?;
                let var_name = var.to_token_stream().to_string();

                match var_name.as_str() {
                    "parser" | "policy" => {
                        ret.extend(quote! {
                            #var.#method(#args);
                        });
                    }
                    _ => {
                        let args = config.value();

                        ret.extend(quote! {
                            #method(#args);
                        });
                    }
                }
            }
        }
        Ok(ret)
    }

    pub fn gen_sync_ret_value(&self) -> TokenStream {
        let mut ret = quote! {};

        if self.configs.has_cfg(CoteKind::AbortHelp) {
            ret.extend(quote! {
                if ret.is_err() ||
                    !ret.as_ref().map(|v|cote::Status::status(v)).unwrap_or(true) {
                    let running_ctx = self.rctx_mut()?;
                    if sub_parser {
                        running_ctx.set_display_sub_help(true);
                        running_ctx.set_exit_sub(false);
                    }
                    else {
                        running_ctx.set_display_help(true);
                        running_ctx.set_exit(false);
                    }
                }
            })
        }
        if self.configs.has_cfg(CoteKind::Help) {
            ret.extend(quote! {
                if self.inner_parser().find_val::<bool>(#HELP_OPTION_NAME).ok() == Some(&true) {
                    let running_ctx = self.rctx_mut()?;
                    if sub_parser {
                        running_ctx.set_display_sub_help(true);
                        running_ctx.set_exit_sub(true);
                    }
                    else {
                        running_ctx.set_display_help(true);
                        running_ctx.set_exit(true);
                    }
                }
            })
        }
        ret
    }

    pub fn gen_help_display_ctx(&self) -> TokenStream {
        let head = if let Some(head_cfg) = self.configs.find_cfg(CoteKind::Head) {
            let value = head_cfg.value();

            quote! {
                String::from(#value)
            }
        } else {
            quote! {
                String::from(env!("CARGO_PKG_DESCRIPTION"))
            }
        };
        let foot = if let Some(foot_cfg) = self.configs.find_cfg(CoteKind::Foot) {
            let value = foot_cfg.value();

            quote! {
                String::from(#value)
            }
        } else {
            quote! {
                format!("Create by {} v{}", env!("CARGO_PKG_AUTHORS"), env!("CARGO_PKG_VERSION"))
            }
        };
        let width = if let Some(head_cfg) = self.configs.find_cfg(CoteKind::HelpWidth) {
            let value = head_cfg.value();

            quote! {
                #value
            }
        } else {
            quote! { 40 }
        };
        let usage_width = if let Some(head_cfg) = self.configs.find_cfg(CoteKind::UsageWidth) {
            let value = head_cfg.value();

            quote! {
                #value
            }
        } else {
            quote! { 10 }
        };
        let name = &self.name;

        quote! {
            cote::HelpDisplayCtx::default()
                .with_name(#name)
                .with_head(#head)
                .with_foot(#foot)
                .with_width(#width)
                .with_usagew(#usage_width)
        }
    }

    pub fn gen_ret_default_policy_ty(&self) -> syn::Result<TokenStream> {
        let policy_ty = self.configs.find_cfg(CoteKind::Policy);

        Ok(if let Some(policy_ty) = policy_ty {
            let policy_name = policy_ty.value().to_token_stream().to_string();
            let policy = gen_ret_default_policy_ty(&policy_name, Some(policy_ty.value()));

            if let Some(policy) = policy {
                policy
            } else {
                policy_ty.value().to_token_stream()
            }
        } else if self.has_sub_command() {
            gen_ret_default_policy_ty(POLICY_PRE, None).unwrap()
        } else {
            gen_ret_default_policy_ty(POLICY_FWD, None).unwrap()
        })
    }

    pub fn gen_ret_policy_ty_generics(&self) -> syn::Result<TokenStream> {
        let policy_ty = self.configs.find_cfg(CoteKind::Policy);

        Ok(if let Some(policy_ty) = policy_ty {
            let policy_name = policy_ty.value().to_token_stream().to_string();
            let policy = gen_ret_policy_ty_generics(&policy_name, Some(policy_ty.value()));

            if let Some(policy) = policy {
                policy
            } else {
                policy_ty.value().to_token_stream()
            }
        } else if self.has_sub_command() {
            gen_ret_policy_ty_generics(POLICY_PRE, None).unwrap()
        } else {
            gen_ret_policy_ty_generics(POLICY_FWD, None).unwrap()
        })
    }

    pub fn gen_main_option_update(&self, idx: usize) -> syn::Result<Option<OptUpdate>> {
        let ident = self.ident;
        let then = self.configs.find_cfg(CoteKind::Then);
        let on = self.configs.find_cfg(CoteKind::On);
        let fallback = self.configs.find_cfg(CoteKind::Fallback);

        if on.is_some() || fallback.is_some() {
            let ident = gen_option_ident(idx, ident.span());
            let uid = gen_option_uid_ident(idx, ident.span());

            Ok(Some((
                Some(quote! {
                    let #ident = {
                        ctor.new_with({
                            let mut config = cote::SetCfg::<Set>::default();
                            config.set_name(format!("main_option_{}", #idx));
                            <cote::Main>::infer_fill_info(&mut config, true);
                            config
                        }).map_err(Into::into)?
                    };
                }),
                Some(quote! {
                    let #uid = set.insert(#ident);
                }),
                Some({
                    if let Some(on_config) = on {
                        let value = on_config.value();

                        if let Some(then_config) = then {
                            let then = then_config.value();

                            quote! {
                                parser.entry(#uid)?.on(#value).then(#then);
                            }
                        } else {
                            quote! {
                                parser.entry(#uid)?.on(#value);
                            }
                        }
                    } else if let Some(fallback_config) = fallback {
                        let value = fallback_config.value();

                        if let Some(then_config) = then {
                            let then = then_config.value();

                            quote! {
                                parser.entry(#uid)?.fallback(#value).then(#then);
                            }
                        } else {
                            quote! {
                                parser.entry(#uid)?.fallback(#value);
                            }
                        }
                    } else {
                        panic!("can not go here")
                    }
                }),
            )))
        } else if then.is_some() {
            error(
                ident.span(),
                "`then` must use with `on` or `fallback` together".to_owned(),
            )
        } else {
            Ok(None)
        }
    }

    pub fn gen_help_option_update(&self, idx: usize) -> Option<(Ident, OptUpdate)> {
        let ident = self.ident;
        self.configs.find_cfg(CoteKind::Help).map(|_| {
            let ident = gen_option_ident(idx, ident.span());
            let uid = gen_option_uid_ident(idx, ident.span());

            (
                uid.clone(),
                (
                    Some(quote! {
                        let #ident = {
                            ctor.new_with({
                                let mut config = cote::SetCfg::<Set>::default();
                                config.set_name(#HELP_OPTION_NAME);
                                config.add_alias(#HELP_OPTION_SHORT);
                                config.set_help(#HELP_OPTION_HELP);
                                <bool>::infer_fill_info(&mut config, true);
                                config
                            }).map_err(Into::into)?
                        };
                    }),
                    Some(quote! {
                        #[allow(unused)]
                        let #uid = set.insert(#ident);
                    }),
                    None,
                ),
            )
        })
    }
}
