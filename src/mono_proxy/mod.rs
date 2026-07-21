#![cfg(feature = "proxy")]

mod app;
mod route;

pub use app::{MonoProxyAppState as AppState, create_app};
pub use route::handler as proxy_handler;
