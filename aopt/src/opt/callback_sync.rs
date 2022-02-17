use std::fmt::Debug;
use std::marker::PhantomData;

use super::OptValue;
use crate::err::Result;
use crate::set::Set;
use crate::uid::Uid;

/// The callback type of option.
///
/// Since rust has a lot of restrict on reference.
/// So we can't store block code into option itself of [`Set`](crate::set::Set).
/// Instead we put the callback code into [`Parser`](crate::parser::Parser).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallbackType {
    Opt,

    OptMut,

    Pos,

    PosMut,

    Main,

    MainMut,

    Null,
}

impl Default for CallbackType {
    fn default() -> Self {
        CallbackType::Null
    }
}

impl CallbackType {
    pub fn is_opt(&self) -> bool {
        match self {
            Self::Opt => true,
            _ => false,
        }
    }

    pub fn is_opt_mut(&self) -> bool {
        match self {
            Self::OptMut => true,
            _ => false,
        }
    }

    pub fn is_pos(&self) -> bool {
        match self {
            Self::Pos => true,
            _ => false,
        }
    }

    pub fn is_pos_mut(&self) -> bool {
        match self {
            Self::PosMut => true,
            _ => false,
        }
    }

    pub fn is_main(&self) -> bool {
        match self {
            Self::Main => true,
            _ => false,
        }
    }

    pub fn is_main_mut(&self) -> bool {
        match self {
            Self::MainMut => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }
}

impl From<Callback> for CallbackType {
    fn from(cb: Callback) -> Self {
        (&cb).into()
    }
}

impl<'a> From<&'a Callback> for CallbackType {
    fn from(cb: &'a Callback) -> Self {
        match cb {
            Callback::Opt(_) => CallbackType::Opt,
            Callback::OptMut(_) => CallbackType::OptMut,
            Callback::Pos(_) => CallbackType::Pos,
            Callback::PosMut(_) => CallbackType::PosMut,
            Callback::Main(_) => CallbackType::Main,
            Callback::MainMut(_) => CallbackType::MainMut,
            Callback::Null => CallbackType::Null,
        }
    }
}

pub trait OptCallback: Debug + Send + Sync {
    fn call(&mut self, uid: Uid, set: &dyn Set, value: OptValue) -> Result<Option<OptValue>>;
}

pub trait OptMutCallback: Debug + Send + Sync {
    fn call(&mut self, uid: Uid, set: &mut dyn Set, value: OptValue) -> Result<Option<OptValue>>;
}

pub trait PosCallback: Debug + Send + Sync {
    fn call(
        &mut self,
        uid: Uid,
        set: &dyn Set,
        arg: &str,
        noa_index: u64,
        value: OptValue,
    ) -> Result<Option<OptValue>>;
}

pub trait PosMutCallback: Debug + Send + Sync {
    fn call(
        &mut self,
        uid: Uid,
        set: &mut dyn Set,
        arg: &str,
        noa_index: u64,
        value: OptValue,
    ) -> Result<Option<OptValue>>;
}

pub trait MainCallback: Debug + Send + Sync {
    fn call(
        &mut self,
        uid: Uid,
        set: &dyn Set,
        args: &[&str],
        value: OptValue,
    ) -> Result<Option<OptValue>>;
}

pub trait MainMutCallback: Debug + Send + Sync {
    fn call(
        &mut self,
        uid: Uid,
        set: &mut dyn Set,
        args: &[&str],
        value: OptValue,
    ) -> Result<Option<OptValue>>;
}

#[derive(Debug)]
pub enum Callback {
    Opt(Box<dyn OptCallback>),

    OptMut(Box<dyn OptMutCallback>),

    Pos(Box<dyn PosCallback>),

    PosMut(Box<dyn PosMutCallback>),

    Main(Box<dyn MainCallback>),

    MainMut(Box<dyn MainMutCallback>),

    Null,
}

unsafe impl Send for Callback {}
unsafe impl Sync for Callback {}

impl Default for Callback {
    fn default() -> Self {
        Self::Null
    }
}

impl Callback {
    pub fn is_mut(&self) -> bool {
        match self {
            Callback::Opt(_) | Callback::Pos(_) | Callback::Main(_) => false,
            Callback::OptMut(_) | Callback::PosMut(_) | Callback::MainMut(_) => true,
            Callback::Null => false,
        }
    }

    pub fn to_callback_type(&self) -> CallbackType {
        self.into()
    }

    pub fn match_callback(&self, callback_type: CallbackType) -> bool {
        match self {
            Callback::Opt(_) => callback_type == CallbackType::Opt,
            Callback::OptMut(_) => callback_type == CallbackType::OptMut,
            Callback::Pos(_) => callback_type == CallbackType::Pos,
            Callback::PosMut(_) => callback_type == CallbackType::PosMut,
            Callback::Main(_) => callback_type == CallbackType::Main,
            Callback::MainMut(_) => callback_type == CallbackType::MainMut,
            Callback::Null => false,
        }
    }
}

impl From<Box<dyn OptCallback>> for Callback {
    fn from(cb: Box<dyn OptCallback>) -> Self {
        Callback::Opt(cb)
    }
}

impl From<Box<dyn OptMutCallback>> for Callback {
    fn from(cb: Box<dyn OptMutCallback>) -> Self {
        Callback::OptMut(cb)
    }
}

impl From<Box<dyn PosCallback>> for Callback {
    fn from(cb: Box<dyn PosCallback>) -> Self {
        Callback::Pos(cb)
    }
}

impl From<Box<dyn PosMutCallback>> for Callback {
    fn from(cb: Box<dyn PosMutCallback>) -> Self {
        Callback::PosMut(cb)
    }
}

impl From<Box<dyn MainCallback>> for Callback {
    fn from(cb: Box<dyn MainCallback>) -> Self {
        Callback::Main(cb)
    }
}

impl From<Box<dyn MainMutCallback>> for Callback {
    fn from(cb: Box<dyn MainMutCallback>) -> Self {
        Callback::MainMut(cb)
    }
}

pub struct SimpleOptCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + Fn(Uid, &dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimpleOptCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimpleOptCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleOptCallback")
            .field("Fn", &String::from("..."))
            .finish()
    }
}

impl<'a, T> OptCallback for SimpleOptCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(&mut self, uid: Uid, set: &dyn Set, value: OptValue) -> Result<Option<OptValue>> {
        self.0(uid, set, value)
    }
}

pub struct SimpleOptMutCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + FnMut(Uid, &mut dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimpleOptMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimpleOptMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleOptMutCallback")
            .field("FnMut", &String::from("..."))
            .finish()
    }
}

impl<'a, T> OptMutCallback for SimpleOptMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(&mut self, uid: Uid, set: &mut dyn Set, value: OptValue) -> Result<Option<OptValue>> {
        self.0(uid, set, value)
    }
}

pub struct SimplePosCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + Fn(Uid, &dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimplePosCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimplePosCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimplePosCallback")
            .field("Fn", &String::from("..."))
            .finish()
    }
}

impl<'a, T> PosCallback for SimplePosCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(
        &mut self,
        uid: Uid,
        set: &dyn Set,
        arg: &str,
        noa_index: u64,
        value: OptValue,
    ) -> Result<Option<OptValue>> {
        self.0(uid, set, arg, noa_index, value)
    }
}

pub struct SimplePosMutCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + FnMut(Uid, &mut dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimplePosMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimplePosMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimplePosMutCallback")
            .field("FnMut", &String::from("..."))
            .finish()
    }
}

impl<'a, T> PosMutCallback for SimplePosMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &str, u64, OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(
        &mut self,
        uid: Uid,
        set: &mut dyn Set,
        arg: &str,
        noa_index: u64,
        value: OptValue,
    ) -> Result<Option<OptValue>> {
        self.0(uid, set, arg, noa_index, value)
    }
}

pub struct SimpleMainCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + Fn(Uid, &dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimpleMainCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimpleMainCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleMainCallback")
            .field("Fn", &String::from("..."))
            .finish()
    }
}

impl<'a, T> MainCallback for SimpleMainCallback<'a, T>
where
    T: 'a + Fn(Uid, &dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(
        &mut self,
        uid: Uid,
        set: &dyn Set,
        args: &[&str],
        value: OptValue,
    ) -> Result<Option<OptValue>> {
        self.0(uid, set, args, value)
    }
}

pub struct SimpleMainMutCallback<'a, T>(T, PhantomData<&'a T>)
where
    T: 'a + FnMut(Uid, &mut dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync;

impl<'a, T> SimpleMainMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    pub fn new(cb: T) -> Self {
        Self(cb, PhantomData::default())
    }
}

impl<'a, T> Debug for SimpleMainMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleMainMutCallback")
            .field("FnMut", &String::from("..."))
            .finish()
    }
}

impl<'a, T> MainMutCallback for SimpleMainMutCallback<'a, T>
where
    T: 'a + FnMut(Uid, &mut dyn Set, &[&str], OptValue) -> Result<Option<OptValue>> + Send + Sync,
{
    fn call(
        &mut self,
        uid: Uid,
        set: &mut dyn Set,
        args: &[&str],
        value: OptValue,
    ) -> Result<Option<OptValue>> {
        self.0(uid, set, args, value)
    }
}