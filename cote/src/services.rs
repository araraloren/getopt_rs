use crate::ctx::RunningCtx;
use aopt::prelude::AppServices;
use aopt::prelude::ErasedTy;
use aopt::prelude::ServicesValExt;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct CoteService {
    service: AppServices,
}

impl CoteService {
    pub fn with_rctx(mut self, ctx: RunningCtx) -> Self {
        self.set_rctx(ctx);
        self
    }

    pub fn set_rctx(&mut self, ctx: RunningCtx) -> &mut Self {
        self.service.sve_insert(ctx);
        self
    }

    pub fn take_rctx(&mut self) -> Result<RunningCtx, aopt::Error> {
        Ok(std::mem::take(self.rctx_mut()?))
    }

    pub fn rctx(&self) -> Result<&RunningCtx, aopt::Error> {
        self.service.sve_val()
    }

    pub fn rctx_mut(&mut self) -> Result<&mut RunningCtx, aopt::Error> {
        self.service.sve_val_mut()
    }

    fn inner_parsers<Sub: ErasedTy>(&self) -> Result<&HashMap<String, Sub>, aopt::Error> {
        self.service.sve_val::<HashMap<String, Sub>>()
    }

    fn inner_parsers_mut<Sub: ErasedTy>(
        &mut self,
    ) -> Result<&mut HashMap<String, Sub>, aopt::Error> {
        self.service.sve_val_mut::<HashMap<String, Sub>>()
    }

    pub fn sub_parsers_iter<Sub: ErasedTy>(
        &self,
    ) -> Result<std::collections::hash_map::Values<'_, String, Sub>, aopt::Error> {
        self.inner_parsers().map(|parsers| parsers.values())
    }

    pub fn sub_parser<Sub: ErasedTy>(&self, name: &str) -> Result<&Sub, aopt::Error> {
        let parsers = self.inner_parsers()?;
        parsers
            .get(name)
            .ok_or_else(|| aopt::raise_error!("Can not find parser by name: {}", name))
    }

    pub fn sub_parser_mut<Sub: ErasedTy>(&mut self, name: &str) -> Result<&mut Sub, aopt::Error> {
        let parsers = self.inner_parsers_mut()?;
        parsers
            .get_mut(name)
            .ok_or_else(|| aopt::raise_error!("Can not find parser by name: {}", name))
    }
}