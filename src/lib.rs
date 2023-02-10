#![feature(trait_alias, type_alias_impl_trait)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod _cookie;
mod future_util;
mod layer;
mod redis_store;
mod session;
mod store;
mod util;

pub use _cookie::*;
pub use future_util::*;
pub use layer::*;
pub use redis_store::*;
pub use session::*;
pub use store::*;
pub use util::*;

cfg_if! {
    if #[cfg(feature = "account")] {
        mod account_store;
        pub use account_store::*;
    }
}
