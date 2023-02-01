use std::fmt::Debug;
use std::marker::PhantomData;

use crate::map::ErasedTy;
use crate::opt::Action;
use crate::opt::ConfigValue;
use crate::opt::Index;
use crate::set::Ctor;
use crate::set::Set;
use crate::set::SetCfg;
use crate::set::SetExt;
use crate::value::Infer;
use crate::value::RawValParser;
use crate::value::ValAccessor;
use crate::value::ValInitializer;
use crate::value::ValStorer;
use crate::value::ValValidator;
use crate::Error;
use crate::Str;
use crate::Uid;

/// Create option using given configurations.
pub struct UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    info: SetCfg<S>,
    set: &'a mut S,
    commited: Option<Uid>,
    pub(crate) drop_commit: bool,
    pub(crate) storer: Option<ValStorer>,
    pub(crate) initializer: Option<ValInitializer>,
    marker: PhantomData<U>,
}

impl<'a, S, U> Debug for UCommit<'a, S, U>
where
    U: Infer,
    S: Set + Debug,
    U::Val: RawValParser,
    SetCfg<S>: ConfigValue + Default + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commit")
            .field("info", &self.info)
            .field("set", &self.set)
            .field("commited", &self.commited)
            .field("drop_commit", &self.drop_commit)
            .field("storer", &self.storer)
            .field("initializer", &self.initializer)
            .finish()
    }
}

impl<'a, S, U> UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    pub fn new(set: &'a mut S, info: SetCfg<S>) -> Self {
        let initializer = U::infer_initializer();
        let storer = if let Some(validator) = U::infer_validator() {
            Some(ValStorer::from(validator))
        } else {
            None
        };
        let info = Self::fill_infer_data(info);

        Self {
            set,
            info,
            commited: None,
            drop_commit: true,
            storer,
            initializer,
            marker: PhantomData::default(),
        }
    }

    pub(crate) fn fill_infer_data(mut info: SetCfg<S>) -> SetCfg<S> {
        let act = U::infer_act();
        let style = U::infer_style();
        let index = U::infer_index();
        let ignore_name = U::infer_ignore_name();
        let support_alias = U::infer_support_alias();
        let positional = U::infer_positional();
        let force = U::infer_force();
        let ctor = U::infer_ctor();

        (!info.has_ctor()).then(|| info.set_ctor(ctor));
        (!info.has_idx()).then(|| index.map(|idx| info.set_idx(idx)));
        (!info.has_type()).then(|| info.set_type::<U::Val>());
        (!info.has_action()).then(|| info.set_action(act));
        (!info.has_style()).then(|| info.set_style(style));
        (!info.has_force()).then(|| info.set_force(force));
        (!info.has_action()).then(|| info.set_action(act));
        info.set_ignore_name(ignore_name);
        info.set_support_alias(support_alias);
        info.set_postional(positional);
        info
    }

    pub fn set_cfg(&self) -> &SetCfg<S> {
        &self.info
    }

    pub fn set_cfg_mut(&mut self) -> &mut SetCfg<S> {
        &mut self.info
    }

    /// Set the option index of commit configuration.
    pub fn set_idx(mut self, index: Index) -> Self {
        self.info.set_idx(index);
        self
    }

    /// Set the option value action.
    pub fn set_action(mut self, action: Action) -> Self {
        self.info.set_action(action);
        self
    }

    /// Set the option name of commit configuration.
    pub fn set_name<T: Into<Str>>(mut self, name: T) -> Self {
        self.info.set_name(name);
        self
    }

    /// Set the option creator of commit configuration.
    pub fn set_ctor<T: Into<Str>>(mut self, ctor: T) -> Self {
        self.cfg_mut().set_ctor(ctor);
        self
    }

    /// Clear all the alias of commit configuration.
    pub fn clr_alias(mut self) -> Self {
        self.info.clr_alias();
        self
    }

    /// Remove the given alias of commit configuration.
    pub fn rem_alias<T: Into<Str>>(mut self, alias: T) -> Self {
        self.info.rem_alias(alias);
        self
    }

    /// Add given alias into the commit configuration.
    pub fn add_alias<T: Into<Str>>(mut self, alias: T) -> Self {
        self.info.add_alias(alias);
        self
    }

    /// Set the option optional of commit configuration.
    pub fn set_force(mut self, force: bool) -> Self {
        self.info.set_force(force);
        self
    }

    /// Set the option hint message of commit configuration.
    pub fn set_hint<T: Into<Str>>(mut self, hint: T) -> Self {
        self.info.set_hint(hint);
        self
    }

    /// Set the option help message of commit configuration.
    pub fn set_help<T: Into<Str>>(mut self, help: T) -> Self {
        self.info.set_help(help);
        self
    }

    /// Set the option value initiator.
    pub fn set_initializer(mut self, initializer: ValInitializer) -> Self {
        self.initializer = Some(initializer);
        self
    }

    pub(crate) fn run_and_commit_the_change(&mut self) -> Result<Uid, Error> {
        if let Some(commited) = self.commited {
            Ok(commited)
        } else {
            self.drop_commit = false;
            self.info.set_storer(ValAccessor::from_storer::<U::Val>(
                self.initializer.take(),
                self.storer.take(),
            ));
            let default_ctor = crate::set::ctor_default_name();
            let info = std::mem::take(&mut self.info);
            let _name = info.name().cloned();
            let ctor = info.ctor().unwrap_or(&default_ctor);
            let opt = self
                .set
                .ctor_mut(ctor)?
                .new_with(info)
                .map_err(|e| e.into())?;
            let uid = self.set.insert(opt);

            crate::trace_log!("Register a opt {:?} --> {}", _name, uid);
            self.commited = Some(uid);
            Ok(uid)
        }
    }

    /// Run the commit.
    ///
    /// It create an option using given type [`Ctor`].
    /// And add it to referenced [`Set`](Set), return the new option [`Uid`].
    pub fn run(mut self) -> Result<Uid, Error> {
        self.drop_commit = false;
        self.run_and_commit_the_change()
    }
}

impl<'a, S, U> UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option value validator.
    pub fn set_validator(mut self, validator: ValValidator<U::Val>) -> Self {
        self.storer = Some(ValStorer::from(validator));
        self
    }

    /// Set the option value validator.
    pub fn set_validator_t<T: ErasedTy + RawValParser>(
        mut self,
        validator: ValValidator<T>,
    ) -> Self {
        self.storer = Some(ValStorer::from(validator));
        self
    }
}

impl<'a, S, U> UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: Copy + RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option default value.
    pub fn set_value(self, value: U::Val) -> Self {
        self.set_initializer(ValInitializer::with(value))
    }

    /// Set the option default value.
    pub fn set_value_t<T: ErasedTy + Copy>(self, value: T) -> Self {
        self.set_initializer(ValInitializer::with(value))
    }
}
impl<'a, S, U> UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: Clone + RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option default value.
    pub fn set_value_clone(self, value: U::Val) -> Self {
        self.set_initializer(ValInitializer::with_clone(value))
    }

    /// Set the option default value.
    pub fn set_values(self, value: Vec<U::Val>) -> Self {
        self.set_initializer(ValInitializer::with_vec(value))
    }

    /// Set the option default value.
    pub fn set_value_clone_t<T: ErasedTy + Clone>(self, value: T) -> Self {
        self.set_initializer(ValInitializer::with_clone(value))
    }

    /// Set the option default value.
    pub fn set_values_t<T: ErasedTy + Clone>(self, value: Vec<T>) -> Self {
        self.set_initializer(ValInitializer::with_vec(value))
    }
}

impl<'a, S, U> Drop for UCommit<'a, S, U>
where
    S: Set,
    U: Infer,
    U::Val: RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    fn drop(&mut self) {
        if self.drop_commit && self.commited.is_none() {
            let error = "Error when commit the option in Commit::Drop, call `run` get the Result";

            self.run_and_commit_the_change().expect(error);
        }
    }
}