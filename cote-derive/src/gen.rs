mod arg;
mod cote;
mod sub;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Attribute;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Field;
use syn::Fields;
use syn::GenericArgument;
use syn::GenericParam;
use syn::Index;
use syn::Lifetime;
use syn::Lit;
use syn::PathArguments;
use syn::Token;
use syn::Type;
use syn::WherePredicate;

const HELP_OPTION_SHORT: &str = "-h";
const HELP_OPTION_NAME: &str = "--help";
const HELP_OPTION_HELP: &str = "Display help message";
const POLICY_PRE: &str = "pre";
const POLICY_FWD: &str = "fwd";
const POLICY_DELAY: &str = "delay";
const CONFIG_ARG: &str = "arg";
const CONFIG_POS: &str = "pos";
const CONFIG_CMD: &str = "cmd";
const APP_POSTFIX: &str = "InternalApp";

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
                let mut cote_generator = CoteGenerator::new(input)?;
                let mut arg_generator = vec![];
                let mut sub_generator = vec![];
                let mut sub_app_idx = 0;
                let mut pos_arg_idx = 1;

                for field in fields.named.iter() {
                    if check_if_has_sub_cfg(field)? {
                        sub_generator.push(SubGenerator::new(field, sub_app_idx)?);
                        cote_generator.set_has_sub_command(true);
                        sub_app_idx += 1;
                    } else {
                        let arg = ArgGenerator::new(field, pos_arg_idx)?;

                        if arg.has_pos_id() {
                            pos_arg_idx += 1;
                        }
                        arg_generator.push(arg);
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

    pub fn gen_all(&self) -> syn::Result<TokenStream> {
        let ident = self.cote_generator.get_ident();
        let (params, where_predicate) = self.cote_generator.split_for_impl();
        let (impl_parser, type_parser, where_parser) =
            self.gen_impl_for_parser(params, where_predicate);
        let (impl_ip, type_ip, where_ip) = self.gen_impl_for_ip(params, where_predicate);
        let (impl_sd, type_sd, where_sd) = self.gen_impl_for_sd(params, where_predicate);
        let parser_update = self.gen_parser_update()?;
        let try_extract = self.gen_try_extract()?;
        let parser_interface = self.gen_parser_interface()?;
        let new_app_interface = self.gen_new_app_for_struct()?;

        Ok(quote! {
            #[doc=concat!("Automatic generated by cote-derive for [`", stringify!(#ident), "`].")]
            impl #impl_ip cote::IntoParserDerive<Set, Inv, Ser>
                for #ident #type_ip #where_ip {
                fn update(parser: &mut aopt::prelude::Parser<Set, Inv, Ser>) -> Result<(), aopt::Error> {
                    #parser_update
                }
            }

            #[doc=concat!("Automatic generated by cote-derive for [`", stringify!(#ident), "`].")]
            impl #impl_sd cote::ExtractFromSetDerive<'z, S>
                for #ident #type_sd #where_sd {
                fn try_extract(set: &'z mut S) -> Result<Self, aopt::Error> where Self: Sized {
                    #try_extract
                }
            }

            #[doc=concat!("Automatic generated by cote-derive for [`", stringify!(#ident), "`].")]
            impl #impl_parser #ident #type_parser #where_parser {
                #parser_interface
            }

            #new_app_interface
        })
    }

    pub fn gen_impl_for_sd(
        &self,
        params: &Punctuated<GenericParam, Token![,]>,
        where_predicate: Option<&Punctuated<WherePredicate, Token![,]>>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        (
            if params.is_empty() {
                quote! {
                    <'z, S>
                }
            } else {
                quote! {
                    <'z, #params, S>
                }
            },
            if params.is_empty() {
                quote! {}
            } else {
                quote! {
                    <#params>
                }
            },
            self.gen_where_for_set_derive(where_predicate),
        )
    }

    pub fn gen_impl_for_ip(
        &self,
        params: &Punctuated<GenericParam, Token![,]>,
        where_predicate: Option<&Punctuated<WherePredicate, Token![,]>>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        (
            if params.is_empty() {
                quote! {
                    <Set, Inv, Ser>
                }
            } else {
                quote! {
                    <#params, Set, Inv, Ser>
                }
            },
            if params.is_empty() {
                quote! {}
            } else {
                quote! {
                    <#params>
                }
            },
            self.gen_where_for_into_parser(where_predicate),
        )
    }

    pub fn gen_impl_for_parser(
        &self,
        params: &Punctuated<GenericParam, Token![,]>,
        where_predicate: Option<&Punctuated<WherePredicate, Token![,]>>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        (
            if params.is_empty() {
                quote! {}
            } else {
                quote! {
                    <#params>
                }
            },
            if params.is_empty() {
                quote! {}
            } else {
                quote! {
                    <#params>
                }
            },
            if let Some(where_predicate) = where_predicate {
                quote! { where #where_predicate }
            } else {
                quote! {}
            },
        )
    }

    pub fn gen_where_for_set_derive(
        &self,
        where_predicate: Option<&Punctuated<WherePredicate, Token![,]>>,
    ) -> TokenStream {
        let default_where = quote! {
            where S: aopt::prelude::SetValueFindExt,
        };
        if let Some(where_predicate) = where_predicate {
            quote! {
                #default_where
                #where_predicate
            }
        } else {
            default_where
        }
    }

    pub fn gen_where_for_into_parser(
        &self,
        where_predicate: Option<&Punctuated<WherePredicate, Token![,]>>,
    ) -> TokenStream {
        let default_where = quote! {
            where
            Ser: aopt::ser::ServicesValExt,
            Set: aopt::prelude::Set + aopt::prelude::ErasedTy + aopt::set::SetValueFindExt,
            Inv: for<'z> aopt::ctx::HandlerCollection<'z, Set, Ser>,
            aopt::prelude::SetCfg<Set>: aopt::prelude::Config + aopt::prelude::ConfigValue + Default,
        };
        if let Some(where_predicate) = where_predicate {
            quote! {
                #default_where
                #where_predicate
            }
        } else {
            default_where
        }
    }

    pub fn gen_try_extract(&self) -> syn::Result<TokenStream> {
        let mut mut_field = vec![];
        let mut ref_field = vec![];

        for field in self.arg_generator.iter() {
            let (is_refopt, ts) = field.gen_value_extract()?;

            if is_refopt {
                ref_field.push(ts);
            } else {
                mut_field.push(ts);
            }
        }
        for field in self.sub_generator.iter() {
            let (is_refopt, ts) = field.gen_field_extract()?;

            if is_refopt {
                ref_field.push(ts);
            } else {
                mut_field.push(ts);
            }
        }
        let mut ret = quote! {};

        ret.extend(mut_field.into_iter());
        ret.extend(ref_field.into_iter());
        Ok(quote! {
            Ok(Self {
                #ret
            })
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
        let sub_parser_tuple_ty = self.gen_sub_parser_tuple_ty(None)?;
        let is_process_help = self.cote_generator.is_process_help();
        let mut help_uid = None;

        let mut append = |(c, i, h): OptUpdate| {
            c.into_iter().for_each(|v| create.push(v));
            i.into_iter().for_each(|v| insert.push(v));
            h.into_iter().for_each(|v| handler.push(v));
        };

        if let Some(update) = self.cote_generator.gen_main_option_update(option_id) {
            append(update);
            option_id += 1;
        }
        if let Some((uid, update)) = self.cote_generator.gen_help_option_update(option_id) {
            help_uid = Some(uid);
            append(update);
            option_id += 1;
        }
        for field in self.arg_generator.iter() {
            append(field.gen_option_update(option_id)?);
            option_id += 1;
        }
        for field in self.sub_generator.iter() {
            append(field.gen_option_update(
                option_id,
                &sub_parser_tuple_ty,
                is_process_help,
                help_uid.as_ref(),
            )?);
            option_id += 1;
        }
        ret.extend(create.into_iter());
        ret.extend(insert.into_iter());
        ret.extend(handler.into_iter());
        ret.extend(quote! { Ok(()) });
        Ok(ret)
    }

    pub fn gen_sub_app_display_call(&self) -> syn::Result<TokenStream> {
        let sub_parser_tuple_ty = self.gen_sub_parser_tuple_ty(None)?;
        let mut sub_parser_tuple_mat = quote! {
            let sub_app_name = &subnames[idx + 1];
            let sub_app_name = app.find_opt(sub_app_name.as_str())?.name();
            let name = subnames.join(" ");
            let sub_parser_tuple = app.inner_parser()
                             .app_data::<#sub_parser_tuple_ty>()?;
        };

        for sub_generator in self.sub_generator.iter() {
            let sub_help_context_gen = sub_generator.gen_sub_help_context()?;
            let idx = sub_generator.get_sub_id();
            let idx = Index::from(idx);

            sub_parser_tuple_mat.extend(quote! {
                if sub_app_name == sub_parser_tuple.#idx.name() {
                    let sub_help_context = { #sub_help_context_gen };

                    return cote::simple_display_set_help(
                        sub_parser_tuple.#idx.inner_parser().optset(),
                        &name, sub_help_context.head(), sub_help_context.foot(),
                        sub_help_context.width(), sub_help_context.usagew()
                    ).map_err(|e| aopt::Error::raise_error(format!("Can not display help message: {:?}", e)))
                }
            });
        }

        Ok(sub_parser_tuple_mat)
    }

    pub fn gen_sub_app_help_call(&self) -> syn::Result<TokenStream> {
        let sub_parser_tuple_ty = self.gen_sub_parser_tuple_ty(None)?;
        let mut sub_parser_tuple_mat = quote! {
            let sub_app_name = &subnames[idx + 1];
            let sub_app_name = app.find_opt(sub_app_name.as_str())?.name();
            let name = subnames.join(" ");
            let sub_parser_tuple = app.inner_parser()
                             .app_data::<#sub_parser_tuple_ty>()?;
        };

        for sub_generator in self.sub_generator.iter() {
            let idx = sub_generator.get_sub_id();
            let idx = Index::from(idx);

            sub_parser_tuple_mat.extend(quote! {
                if sub_app_name == sub_parser_tuple.#idx.name() {
                    return sub_parser_tuple.#idx.display_sub_help_idx(subnames, idx + 1);
                }
            });
        }

        Ok(sub_parser_tuple_mat)
    }

    pub fn gen_sub_parser_tuple_ty(&self, lifetime: Option<Lifetime>) -> syn::Result<TokenStream> {
        let mut inner_app_ty = quote! {};

        for sub_generator in self.sub_generator.iter() {
            let sub_policy_ty = sub_generator.gen_policy_type()?;
            let app_type = sub_generator.gen_app_type(lifetime.clone(), &sub_policy_ty)?;

            inner_app_ty.extend(quote! {
                #app_type,
            });
        }

        Ok(quote! {
            (#inner_app_ty)
        })
    }

    pub fn gen_insert_sub_apps(&self) -> syn::Result<TokenStream> {
        let mut inner_app_ty = quote! {};

        for sub_generator in self.sub_generator.iter() {
            let without_option_ty = sub_generator.get_without_option_type();
            let sub_policy_ty = sub_generator.gen_policy_type()?;
            let sub_app_name = sub_generator.name();

            inner_app_ty.extend(quote! {
                {
                    let mut sub_app = <#without_option_ty>::into_app_policy::<'_, #sub_policy_ty>()?;
                    sub_app.set_name(#sub_app_name);
                    sub_app
                },
            });
        }

        if self.sub_generator.is_empty() {
            Ok(quote! {})
        } else {
            Ok(quote! {
                parser.set_app_data((#inner_app_ty))?;
            })
        }
    }

    pub fn gen_policy_settings(&self) -> TokenStream {
        let mut ret = quote! {};

        if let Some(style_settings) = self.cote_generator.gen_style_settings_for_policy() {
            ret.extend(style_settings);
        }
        for arg in self.arg_generator.iter() {
            ret.extend(arg.gen_nodelay_for_delay_parser().into_iter());
        }
        ret
    }

    pub fn where_clause_for_policy() -> TokenStream {
        quote! {
            where
            P::Ser: aopt::ser::ServicesValExt + 'z,
            P::Error: Into<aopt::Error>,
            P::Set: aopt::prelude::Set + aopt::prelude::ErasedTy + aopt::set::SetValueFindExt + 'z,
            P::Inv<'z>: aopt::ctx::HandlerCollection<'z, P::Set, P::Ser>,
            P: aopt::prelude::Policy + aopt::prelude::APolicyExt<P> + aopt::prelude::PolicySettings + Default + 'z,
            aopt::prelude::SetCfg<P::Set>: aopt::prelude::Config + aopt::prelude::ConfigValue + Default,
        }
    }

    pub fn where_clause_for_policy_debug() -> TokenStream {
        quote! {
            where
            P::Ret: std::fmt::Debug,
            P::Ser: aopt::ser::ServicesValExt + std::fmt::Debug + 'z,
            P::Error: Into<aopt::Error>,
            P::Set: aopt::prelude::Set + std::fmt::Debug + aopt::set::SetValueFindExt + 'z,
            P::Inv<'z>: aopt::ctx::HandlerCollection<'z, P::Set, P::Ser> + std::fmt::Debug,
            P: aopt::prelude::Policy + aopt::prelude::APolicyExt<P> + aopt::prelude::PolicySettings + Default + std::fmt::Debug + 'z,
            aopt::prelude::SetCfg<P::Set>: aopt::prelude::Config + aopt::prelude::ConfigValue + Default,
        }
    }

    pub fn where_clause_for_parser() -> TokenStream {
        quote! {
            where
            Ser: aopt::ser::ServicesValExt + std::fmt::Debug,
            Set: aopt::prelude::Set + std::fmt::Debug + aopt::set::SetValueFindExt,
            Inv: for<'z> aopt::ctx::HandlerCollection<'z, Set, Ser> + std::fmt::Debug,
            aopt::prelude::SetCfg<Set>: aopt::prelude::Config + aopt::prelude::ConfigValue + Default,
        }
    }

    pub fn gen_parser_interface(&self) -> syn::Result<TokenStream> {
        let struct_app_ty = self.cote_generator.gen_struct_app_type();
        let policy_ty = self.cote_generator.gen_policy_type()?;
        let insert_sub_apps = self.gen_insert_sub_apps()?;
        let policy_settings = self.gen_policy_settings();
        let app_raw_tweaks = self.cote_generator.gen_tweak_on_app();
        let parser_app_name = self.cote_generator.get_name();
        let where_clause = Self::where_clause_for_policy();
        let where_clause_parser = Self::where_clause_for_parser();

        Ok(quote! {
            pub fn gen_parser<Set, Inv, Ser>(set: Set, inv: Inv, ser: Ser,) ->
                Result<cote::CoteParser<Set, Inv, Ser>, aopt::Error> #where_clause_parser {
                let parser = <Self  as cote::IntoParserDerive<Set, Inv, Ser>>::into_parser(set, inv, ser)?;
                Ok(cote::CoteParser::new_with_parser(#parser_app_name, parser))
            }

            pub fn gen_parser_with<'a, P>(policy: &P) ->
                Result<cote::CoteParser<P::Set, P::Inv<'a>, P::Ser>, aopt::Error> #where_clause_parser {
                let parser = <Self  as cote::IntoParserDerive<P::Set, P::Inv<'a>, P::Ser>>::into_parser_with(policy)?;
                Ok(cote::CoteParser::new_with_parser(#parser_app_name, parser))
            }

            pub fn gen_policy<P>() -> P where P: aopt::prelude::PolicySettings + Default {
                let mut policy = P::default();
                #policy_settings
                policy
            }

            pub fn gen_default_policy() -> #policy_ty {
                Self::gen_policy::<#policy_ty>()
            }

            /// Parsing the given arguments and return the [`GetoptRes`](aopt::GetoptRes).
            pub fn parse_args_with<'a, P>(policy: &mut P, args: aopt::prelude::Args)
                -> Result<aopt::GetoptRes<<P as aopt::prelude::Policy>::Ret, cote::CoteParser<P::Set, P::Inv<'a>, P::Ser>, aopt::Error> {
                let mut parser = Self::gen_parser_with(policy)?;
                
                parser.service_mut().set_rctx(cote::RunningCtx::default());
                parser.service_mut().rctx_mut()?.add_name(#parser_app_name);
                // todo
                let parser = app.inner_parser_mut();

                parser.init()?;
                let ret = parser.parse(aopt::ARef::new(aopt::prelude::Args::from(args))).map_err(Into::into);

                app.sync_running_ctx(&ret, false)?;
                let running_ctx = app.get_running_ctx()?;

                if running_ctx.display_sub_help() {
                    app.display_sub_help(running_ctx.names())?;
                    if running_ctx.exit_sub() {
                        std::process::exit(0)
                    }
                }
                else if running_ctx.display_help() {
                    app.display_help()?;
                    if running_ctx.exit() {
                        std::process::exit(0)
                    }
                }
                Ok(aopt::GetoptRes{ ret: ret?, parser: app })
            }

            /// Parsing arguments returned from [`from_env`](aopt::prelude::Args::from_env) and return the [`GetoptRes`](aopt::GetoptRes).
            pub fn parse_env_args<'z>()
            -> Result<aopt::GetoptRes<<#policy_ty as aopt::prelude::Policy>::Ret, #struct_app_ty<'z, #policy_ty>>, aopt::Error> {
                Self::parse_args(aopt::prelude::Args::from_env())
            }

            /// Parsing the given arguments and generate a .
            pub fn parse(args: aopt::prelude::Args) -> Result<Self, aopt::Error> {
                let GetoptRes { mut ret, mut parser } = Self::parse_args(args)?;

                if ret.status() {
                    Self::try_extract(parser.inner_parser_mut().optset_mut())
                }
                else {
                    let mut rctx = parser.take_running_ctx()?;
                    let error = rctx.chain_error();
                    let mut finfo = rctx.take_failed_info();
                    let (command, ret) = finfo.first_mut().map(|v|(Some(v.0.as_str()), &mut v.1)).unwrap_or((None, &mut ret));
                    let e = {
                        let ctx = ret.take_ctx();
                        let args = ctx.orig_args()[1..]
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ");
                        let inner_ctx = ctx.inner_ctx().ok();
                        let failed_msg = if let Some(command) = command {
                            format!("Parsing command `{}`", command)
                        }
                        else {
                            format!("Parsing arguments `{}`", args)
                        };
                        let inner_ctx = if let Some(inner_ctx) = inner_ctx {
                            format!("{}", inner_ctx)
                        } else {
                            "None".to_owned()
                        };

                        if let Some(error) = error {
                            // return failure with more detail error message
                            aopt::raise_failure!("{} failed: {}", failed_msg, inner_ctx).cause_by(error)
                        }
                        else {
                            // return failure with more detail error message
                            aopt::raise_failure!("{} failed: {}", failed_msg, inner_ctx)
                        }
                    };

                    Err(e)
                }
            }

            pub fn parse_env() -> Result<Self, aopt::Error> {
                Self::parse(aopt::prelude::Args::from_env())
            }
        })
    }

    pub fn gen_new_app_for_struct(&self) -> syn::Result<TokenStream> {
        let new_app_type = self.cote_generator.gen_struct_app_type();
        let new_app_define = self.cote_generator.gen_new_app_define(&new_app_type);
        let help_context = self.cote_generator.gen_help_display_ctx();
        let sub_app_display_call = self.gen_sub_app_display_call()?;
        let sub_app_help_call = self.gen_sub_app_help_call()?;
        let sync_main_running_ctx = self.cote_generator.gen_sync_running_ctx();
        let where_clause = Self::where_clause_for_policy();
        let where_clause_debug = Self::where_clause_for_policy_debug();
        let static_lifetime = Lifetime::new("'static", new_app_type.span());
        let sub_apps_tuple_ty = self.gen_sub_parser_tuple_ty(Some(static_lifetime))?;
        let override_run = gen_run_override(&new_app_type);

        Ok(quote! {
            #new_app_define

            impl<'z, P: Policy> std::fmt::Debug for #new_app_type<'z, P> #where_clause_debug {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!(#new_app_type))
                        .field("app", &self.0)
                        .finish()
                }
            }

            impl<'z, P: Policy> #new_app_type<'z, P> #where_clause {
                pub fn new(app: cote::CoteApp<'z, P>) -> Self {
                    Self(app)
                }

                pub fn sub_apps(&self) -> Result<&#sub_apps_tuple_ty, aopt::Error> {
                    self.inner_parser().app_data::<#sub_apps_tuple_ty>()
                }

                pub fn sub_apps_mut(&mut self) -> Result<&mut #sub_apps_tuple_ty, aopt::Error> {
                    self.inner_parser_mut().app_data_mut::<#sub_apps_tuple_ty>()
                }

                pub fn sub_apps_of(ser: &P::Ser) -> Result<&#sub_apps_tuple_ty, aopt::Error> {
                    ser.sve_val::<#sub_apps_tuple_ty>()
                }

                pub fn sub_apps_mut_of(ser: &mut P::Ser) -> Result<&mut #sub_apps_tuple_ty, aopt::Error> {
                    ser.sve_val_mut::<#sub_apps_tuple_ty>()
                }

                pub fn with_default_running_ctx(mut self) -> Result<Self, aopt::Error> {
                    self.set_running_ctx(cote::AppRunningCtx::default())?;
                    Ok(self)
                }

                pub fn set_running_ctx(&mut self, ctx: cote::AppRunningCtx) -> Result<&mut Self, aopt::Error> {
                    self.inner_parser_mut().set_app_data(ctx)?;
                    Ok(self)
                }

                pub fn get_running_ctx(&self) -> Result<&cote::AppRunningCtx, aopt::Error> {
                    self.inner_parser().app_data::<cote::AppRunningCtx>()
                }

                pub fn get_running_ctx_mut(&mut self) -> Result<&mut cote::AppRunningCtx, aopt::Error> {
                    self.inner_parser_mut().app_data_mut::<cote::AppRunningCtx>()
                }

                pub fn take_running_ctx(&mut self) -> Result<cote::AppRunningCtx, aopt::Error> {
                    Ok(std::mem::take(self.inner_parser_mut().app_data_mut::<cote::AppRunningCtx>()?))
                }

                pub fn sync_running_ctx(&mut self, ret: &Result<aopt::prelude::ReturnVal, aopt::Error>, sub_parser: bool) -> Result<&mut Self, aopt::Error> {
                    #sync_main_running_ctx
                    Ok(self)
                }

                pub fn gen_help_display_ctx(&self) -> cote::HelpDisplayCtx {
                    #help_context
                }

                pub fn display_help(&self) -> Result<(), aopt::Error> {
                    self.display_help_with(self.gen_help_display_ctx())
                }

                pub fn display_help_with(&self, context: cote::HelpDisplayCtx) -> Result<(), aopt::Error> {
                    let name = context.generate_name();

                    cote::simple_display_set_help(
                        self.inner_parser().optset(),
                        &name, context.head(), context.foot(),
                        context.width(), context.usagew()
                    ).map_err(|e| aopt::Error::raise_error(format!("Can not display help message: {:?}", e)))
                }

                pub fn display_sub_help(&self, subnames: &[String]) -> Result<(), aopt::Error> {
                    self.display_sub_help_idx(subnames, 0)
                }

                pub fn display_sub_help_idx(&self, subnames: &[String], idx: usize) -> Result<(), aopt::Error> {
                    let len = subnames.len();
                    let app = &self.0;

                    if len >= 1 {
                        let name_matched = &subnames[idx] == app.name();

                        if idx == len - 1 && len == 1 {
                            return self.display_help()
                        }
                        else if idx == len - 2 && name_matched {
                            #sub_app_display_call
                        }
                        else if idx < len && name_matched {
                            #sub_app_help_call
                        }
                    }
                    Err(aopt::Error::raise_error(format!("Can not display help message of subnames: {:?}", subnames)))
                }

                #override_run
            }
        })
    }
}

pub fn gen_option_ident(idx: usize, span: Span) -> Ident {
    Ident::new(&format!("option_{}", idx), span)
}

pub fn gen_option_uid_ident(idx: usize, span: Span) -> Ident {
    Ident::new(&format!("option_uid_{}", idx), span)
}

pub fn check_if_has_sub_cfg(field: &Field) -> syn::Result<bool> {
    let attrs = &field.attrs;
    let has_sub_cfg = attrs.iter().any(|v| v.path.is_ident("sub"));
    let has_arg_cfg = attrs.iter().any(|v| v.path.is_ident(CONFIG_ARG));
    let has_cmd_cfg = attrs.iter().any(|v| v.path.is_ident(CONFIG_CMD));
    let has_pos_cfg = attrs.iter().any(|v| v.path.is_ident(CONFIG_POS));

    if (has_arg_cfg || has_cmd_cfg || has_pos_cfg) && has_sub_cfg {
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

pub fn check_in_path(ty: &Type, name: &str) -> syn::Result<bool> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident = segment.ident.to_string();

            if ident == name {
                return Ok(true);
            } else if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                for arg in ab.args.iter() {
                    if let GenericArgument::Type(next_ty) = arg {
                        return check_in_path(next_ty, name);
                    }
                }
            }
        }
        Ok(false)
    } else {
        abort! {
            ty, "Cote not support reference type"
        }
    }
}

pub fn gen_ty_without_option(ty: &Type) -> syn::Result<Type> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident_str = segment.ident.to_string();

            if ident_str == "Option" {
                if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                    if let Some(GenericArgument::Type(next_ty)) = ab.args.first().as_ref() {
                        return Ok(next_ty.clone());
                    }
                }
            }
        }
    }
    abort! {
        ty,
        "`sub` configuration only support `Option<T>`"
    }
}

// pub fn is_option_ty(ty: &Type) -> bool {
//     if let Type::Path(path) = ty {
//         if let Some(segment) = path.path.segments.last() {
//             let ident_str = segment.ident.to_string();

//             if ident_str == "Option" {
//                 if let PathArguments::AngleBracketed(_) = &segment.arguments {
//                     return true;
//                 }
//             }
//         }
//     }
//     false
// }

pub fn gen_subapp_without_option(ty: &Type) -> syn::Result<&Ident> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            return Ok(&segment.ident);
        }
    }
    abort! {
        ty,
        "can not generate sub app type"
    }
}

pub fn gen_run_override(app: &Ident) -> TokenStream {
    quote! {
        pub fn run_mut_with<'c, 'b, I, R, F>(
            &'c mut self,
            iter: impl Iterator<Item = I>,
            mut r: F,
        ) -> Result<R, aopt::Error>
        where
            'c: 'b,
            I: Into<aopt::prelude::RawVal>,
            F: FnMut(P::Ret, &'b mut #app<'z, P>) -> Result<R, aopt::Error>,
        {
            let args = iter.map(|v| v.into());
            let parser = self.inner_parser_mut();

            // initialize the option value
            parser.init()?;

            let ret = parser
                .parse(aopt::ARef::new(aopt::prelude::Args::from(args)))
                .map_err(Into::into)?;

            r(ret, self)
        }

        pub fn run_mut<'c, 'b, R, F>(&'c mut self, r: F) -> Result<R, aopt::Error>
        where
            'c: 'b,
            F: FnMut(P::Ret, &'b mut #app<'z, P>) -> Result<R, aopt::Error>,
        {
            let args = aopt::prelude::Args::from_env().into_inner();
            self.run_mut_with(args.into_iter(), r)
        }

        pub async fn run_async_mut_with<'c, 'b, I, R, FUT, F>(
            &'c mut self,
            iter: impl Iterator<Item = I>,
            mut r: F,
        ) -> Result<R, aopt::Error>
        where
            'c: 'b,
            I: Into<aopt::prelude::RawVal>,
            FUT: std::future::Future<Output = Result<R, aopt::Error>>,
            F: FnMut(P::Ret, &'b mut #app<'z, P>) -> FUT,
        {
            let args = iter.map(|v| v.into());
            let parser = self.inner_parser_mut();
            let async_ret;

            // initialize the option value
            parser.init()?;
            match parser.parse(aopt::ARef::new(aopt::prelude::Args::from(args))) {
                Ok(ret) => {
                    let ret = r(ret, self).await;

                    async_ret = ret;
                }
                Err(e) => {
                    async_ret = Err(e.into());
                }
            }
            async_ret
        }

        pub async fn run_async_mut<'c, 'b, R, FUT, F>(&'c mut self, r: F) -> Result<R, aopt::Error>
        where
            'c: 'b,
            FUT: std::future::Future<Output = Result<R, aopt::Error>>,
            F: FnMut(P::Ret, &'b mut #app<'z, P>) -> FUT,
        {
            let args = aopt::prelude::Args::from_env().into_inner();
            self.run_async_mut_with(args.into_iter(), r).await
        }

        pub fn run_with<'c, 'b, I, R, F>(
            &'c mut self,
            iter: impl Iterator<Item = I>,
            mut r: F,
        ) -> Result<R, aopt::Error>
        where
            'c: 'b,
            I: Into<aopt::prelude::RawVal>,
            F: FnMut(P::Ret, &'b #app<'z, P>) -> Result<R, aopt::Error>,
        {
            let args = iter.map(|v| v.into());
            let parser = self.inner_parser_mut();

            // initialize the option value
            parser.init()?;

            let ret = parser
                .parse(aopt::ARef::new(aopt::prelude::Args::from(args)))
                .map_err(Into::into)?;

            r(ret, self)
        }

        pub fn run<'c, 'b, R, F>(&'c mut self, r: F) -> Result<R, aopt::Error>
        where
            'c: 'b,
            F: FnMut(P::Ret, &'b #app<'z, P>) -> Result<R, aopt::Error>,
        {
            let args = aopt::prelude::Args::from_env().into_inner();
            self.run_with(args.into_iter(), r)
        }

        pub async fn run_async_with<'c, 'b, I, R, FUT, F>(
            &'c mut self,
            iter: impl Iterator<Item = I>,
            mut r: F,
        ) -> Result<R, aopt::Error>
        where
            'c: 'b,
            I: Into<aopt::prelude::RawVal>,
            FUT: std::future::Future<Output = Result<R, aopt::Error>>,
            F: FnMut(P::Ret, &'b #app<'z, P>) -> FUT,
        {
            let args = iter.map(|v| v.into());
            let parser = self.inner_parser_mut();
            let async_ret;

            // initialize the option value
            parser.init()?;
            match parser.parse(aopt::ARef::new(aopt::prelude::Args::from(args))) {
                Ok(ret) => {
                    let ret = r(ret, self).await;

                    async_ret = ret;
                }
                Err(e) => {
                    async_ret = Err(e.into());
                }
            }
            async_ret
        }

        pub async fn run_async<'c, 'b, R, FUT, F>(&'c mut self, r: F) -> Result<R, aopt::Error>
        where
            'c: 'b,
            FUT: std::future::Future<Output = Result<R, aopt::Error>>,
            F: FnMut(P::Ret, &'b #app<'z, P>) -> FUT,
        {
            let args = aopt::prelude::Args::from_env().into_inner();
            self.run_async_with(args.into_iter(), r).await
        }
    }
}
