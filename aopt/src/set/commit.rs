use std::fmt::Debug;
use std::marker::PhantomData;

use crate::opt::ConfigValue;
use crate::prelude::ErasedTy;
use crate::set::Ctor;
use crate::set::Set;
use crate::set::SetCfg;
use crate::set::SetExt;
use crate::value::Infer;
use crate::value::RawValParser;
use crate::value::ValInitializer;
use crate::value::ValStorer;
use crate::value::ValValidator;
use crate::Error;
use crate::Uid;

use super::Commit;
use super::SetCommitInfered;

/// Create option using given configurations.
pub struct SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    info: Option<SetCfg<S>>,
    set: Option<&'a mut S>,
    commited: Option<Uid>,
    pub(crate) drop_commit: bool,
}

impl<'a, S> Debug for SetCommit<'a, S>
where
    S: Set + Debug,
    SetCfg<S>: ConfigValue + Default + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commit")
            .field("info", &self.info)
            .field("set", &self.set)
            .field("commited", &self.commited)
            .field("drop_commit", &self.drop_commit)
            .finish()
    }
}

impl<'a, S> SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    pub fn new(set: &'a mut S, info: SetCfg<S>) -> Self {
        Self {
            set: Some(set),
            info: Some(info),
            commited: None,
            drop_commit: true,
        }
    }

    pub(crate) fn run_and_commit_the_change(&mut self) -> Result<Uid, Error> {
        if let Some(commited) = self.commited {
            Ok(commited)
        } else {
            self.drop_commit = false;

            let info = std::mem::take(&mut self.info);
            let mut info = info.unwrap();
            let set = self.set.as_mut().unwrap();

            // Note !!
            // here we don't have value type here, set the ValAccessor with fake type
            // fix it in option creator handler if `Config` set `fix_infer`
            info.set_fix_infer(true);

            let _name = info.name().cloned();
            let ctor = info.ctor().ok_or_else(|| {
                Error::raise_error("Invalid configuration: missing creator name!")
            })?;
            let opt = set.ctor_mut(ctor)?.new_with(info).map_err(|e| e.into())?;
            let uid = set.insert(opt);

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

impl<'a, S> Drop for SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    fn drop(&mut self) {
        if self.drop_commit && self.commited.is_none() {
            let error = "Error when commit the option in Commit::Drop, call `run` get the Result";

            self.run_and_commit_the_change().expect(error);
        }
    }
}

impl<'a, S> Commit<S> for SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    fn cfg(&self) -> &SetCfg<S> {
        self.info.as_ref().unwrap()
    }

    fn cfg_mut(&mut self) -> &mut SetCfg<S> {
        self.info.as_mut().unwrap()
    }
}

/// Convert [`Commit`] to [`CommitInfered`].
impl<'a, S> SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the type of option.
    pub fn set_type<U: Infer>(mut self) -> SetCommitInfered<'a, S, U>
    where
        U::Val: RawValParser,
    {
        self.drop_commit = false;

        let set = self.set.take();
        let info = self.info.take();

        SetCommitInfered::new(set.unwrap(), info.unwrap())
    }

    /// Set the option value validator.
    pub fn set_validator<U: Infer>(
        self,
        validator: ValValidator<U::Val>,
    ) -> SetCommitInfered<'a, S, U>
    where
        U::Val: RawValParser,
    {
        self.set_type::<U>().set_validator(validator)
    }

    /// Set the option default value.
    pub fn set_value<U: Infer>(self, value: U::Val) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Copy + RawValParser,
    {
        self.set_type::<U>().set_value(value)
    }

    /// Set the option default value.
    pub fn set_value_clone<U: Infer>(self, value: U::Val) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Clone + RawValParser,
    {
        self.set_type::<U>().set_value_clone(value)
    }

    /// Set the option default value.
    pub fn set_values<U: Infer>(self, value: Vec<U::Val>) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Clone + RawValParser,
    {
        self.set_type::<U>().set_values(value)
    }
}

/// Convert [`Commit`] to [`CommitWithValue`].
impl<'a, S> SetCommit<'a, S>
where
    S: Set,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the type of option.
    fn set_value_type<T: ErasedTy>(mut self) -> SetCommitWithValue<'a, S, T> {
        self.drop_commit = false;

        let set = self.set.take();
        let info = self.info.take();

        SetCommitWithValue::new(set.unwrap(), info.unwrap())
    }

    /// Set the option value validator.
    pub fn set_validator_t<T: ErasedTy + RawValParser>(
        self,
        validator: ValValidator<T>,
    ) -> SetCommitWithValue<'a, S, T> {
        self.set_value_type::<T>().set_validator_t(validator)
    }

    /// Set the option default value.
    pub fn set_value_t<T: ErasedTy + Copy>(self, value: T) -> SetCommitWithValue<'a, S, T> {
        self.set_value_type::<T>().set_value_t(value)
    }

    /// Set the option default value.
    pub fn set_value_clone_t<T: ErasedTy + Clone>(self, value: T) -> SetCommitWithValue<'a, S, T> {
        self.set_value_type::<T>()
            .set_initializer(ValInitializer::with_clone(value))
    }

    /// Set the option default value.
    pub fn set_values_t<T: ErasedTy + Clone>(self, value: Vec<T>) -> SetCommitWithValue<'a, S, T> {
        self.set_value_type::<T>()
            .set_initializer(ValInitializer::with_vec(value))
    }
}

/// Create option using given configurations.
pub struct SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy,
    SetCfg<S>: ConfigValue + Default,
{
    info: Option<SetCfg<S>>,
    set: Option<&'a mut S>,
    commited: Option<Uid>,
    pub(crate) drop_commit: bool,
    marker: PhantomData<T>,
}

impl<'a, S, T> SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy,
    SetCfg<S>: ConfigValue + Default,
{
    pub fn new(set: &'a mut S, info: SetCfg<S>) -> Self {
        Self {
            set: Some(set),
            info: Some(info),
            commited: None,
            drop_commit: true,
            marker: PhantomData::default(),
        }
    }

    pub(crate) fn run_and_commit_the_change(&mut self) -> Result<Uid, Error> {
        if let Some(commited) = self.commited {
            Ok(commited)
        } else {
            self.drop_commit = false;

            let info = std::mem::take(&mut self.info);
            let mut info = info.unwrap();
            let set = self.set.as_mut().unwrap();

            // Note !!
            // here we don't have value type here, set the ValAccessor with fake type
            // fix it in option creator handler if `Config` set `fix_infer`
            info.set_fix_infer(true);

            let _name = info.name().cloned();
            let ctor = info.ctor().ok_or_else(|| {
                Error::raise_error("Invalid configuration: missing creator name!")
            })?;
            let opt = set.ctor_mut(ctor)?.new_with(info).map_err(|e| e.into())?;
            let uid = set.insert(opt);

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

impl<'a, S, T> SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy + RawValParser,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option value validator.
    pub fn set_validator_t(mut self, validator: ValValidator<T>) -> Self {
        self.cfg_mut()
            .set_storer(ValStorer::new_validator(validator));
        self
    }
}

impl<'a, S, T> SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy + Copy,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option default value.
    pub fn set_value_t(self, value: T) -> Self {
        self.set_initializer(ValInitializer::with(value))
    }
}
impl<'a, S, T> SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy + Clone,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the option default value.
    pub fn set_value_clone_t(self, value: T) -> Self {
        self.set_initializer(ValInitializer::with_clone(value))
    }

    /// Set the option default value.
    pub fn set_values_t(self, value: Vec<T>) -> Self {
        self.set_initializer(ValInitializer::with_vec(value))
    }
}

impl<'a, S, T> SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy,
    SetCfg<S>: ConfigValue + Default,
{
    /// Set the type of option.
    pub fn set_type<U: Infer>(mut self) -> SetCommitInfered<'a, S, U>
    where
        U::Val: RawValParser,
    {
        self.drop_commit = false;

        let set = self.set.take();
        let info = self.info.take();

        SetCommitInfered::new(set.unwrap(), info.unwrap())
    }

    /// Set the option value validator.
    pub fn set_validator<U: Infer>(
        self,
        validator: ValValidator<U::Val>,
    ) -> SetCommitInfered<'a, S, U>
    where
        U::Val: RawValParser,
    {
        self.set_type::<U>().set_validator(validator)
    }

    /// Set the option default value.
    pub fn set_value<U: Infer>(self, value: U::Val) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Copy + RawValParser,
    {
        self.set_type::<U>().set_value(value)
    }

    /// Set the option default value.
    pub fn set_value_clone<U: Infer>(self, value: U::Val) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Clone + RawValParser,
    {
        self.set_type::<U>().set_value_clone(value)
    }

    /// Set the option default value.
    pub fn set_values<U: Infer>(self, value: Vec<U::Val>) -> SetCommitInfered<'a, S, U>
    where
        U::Val: Clone + RawValParser,
    {
        self.set_type::<U>().set_values(value)
    }
}

impl<'a, S, T> Commit<S> for SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy,
    SetCfg<S>: ConfigValue + Default,
{
    fn cfg(&self) -> &SetCfg<S> {
        self.info.as_ref().unwrap()
    }

    fn cfg_mut(&mut self) -> &mut SetCfg<S> {
        self.info.as_mut().unwrap()
    }
}

impl<'a, S, T> Drop for SetCommitWithValue<'a, S, T>
where
    S: Set,
    T: ErasedTy,
    SetCfg<S>: ConfigValue + Default,
{
    fn drop(&mut self) {
        if self.drop_commit && self.commited.is_none() {
            let error = "Error when commit the option in Commit::Drop, call `run` get the Result";

            self.run_and_commit_the_change().expect(error);
        }
    }
}
