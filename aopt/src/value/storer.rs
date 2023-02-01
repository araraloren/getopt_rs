use std::fmt::Debug;

use crate::ctx::Ctx;
use crate::map::ErasedTy;
use crate::opt::Action;
use crate::trace_log;
use crate::Error;
use crate::RawVal;

use super::AnyValue;
use super::RawValParser;
use super::ValValidator;

#[cfg(feature = "sync")]
pub type StoreHandler<T> =
    Box<dyn FnMut(Option<&RawVal>, &Ctx, &Action, &mut T) -> Result<(), Error> + Send + Sync>;

#[cfg(not(feature = "sync"))]
pub type StoreHandler<T> =
    Box<dyn FnMut(Option<&RawVal>, &Ctx, &Action, &mut T) -> Result<(), Error>>;

pub struct ValStorer(StoreHandler<AnyValue>);

impl Debug for ValStorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WriterHandler").field(&"{...}").finish()
    }
}

impl ValStorer {
    pub fn new<U: ErasedTy + RawValParser>() -> Self {
        Self(Self::fallback::<U>())
    }

    pub fn new_validator<U: ErasedTy + RawValParser>(validator: ValValidator<U>) -> Self {
        Self(Self::validator(validator))
    }

    pub fn invoke(
        &mut self,
        raw: Option<&RawVal>,
        ctx: &Ctx,
        act: &Action,
        arg: &mut AnyValue,
    ) -> Result<(), Error> {
        crate::trace_log!("Saving raw value({:?}) for {}", raw, ctx.uid()?);
        (self.0)(raw, ctx, act, arg)
    }

    pub fn validator<U: ErasedTy + RawValParser>(
        validator: ValValidator<U>,
    ) -> StoreHandler<AnyValue> {
        Box::new(
            move |raw: Option<&RawVal>, ctx: &Ctx, act: &Action, handler: &mut AnyValue| {
                let val = U::parse(raw, ctx).map_err(Into::into)?;

                trace_log!("Validator value storer, parsing {:?} -> {:?}", raw, val);
                if validator.invoke(&val) {
                    Err(Error::raise_failure(format!(
                        "Value check failed for option {:?}",
                        ctx.uid()
                    )))
                } else {
                    act.store1(Some(val), handler);
                    Ok(())
                }
            },
        )
    }

    pub fn fallback<U: ErasedTy + RawValParser>() -> StoreHandler<AnyValue> {
        Box::new(
            |raw: Option<&RawVal>, ctx: &Ctx, act: &Action, handler: &mut AnyValue| {
                let val = U::parse(raw, ctx).map_err(Into::into)?;

                trace_log!("Fallback value storer, parsing {:?} -> {:?}", raw, val);
                act.store1(Some(val), handler);
                Ok(())
            },
        )
    }
}

impl<U: ErasedTy + RawValParser> From<ValValidator<U>> for ValStorer {
    fn from(validator: ValValidator<U>) -> Self {
        Self::new_validator(validator)
    }
}

impl<U: ErasedTy + RawValParser> From<Option<ValValidator<U>>> for ValStorer {
    fn from(validator: Option<ValValidator<U>>) -> Self {
        if let Some(validator) = validator {
            Self::new_validator(validator)
        } else {
            Self::new::<U>()
        }
    }
}