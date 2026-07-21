#![cfg(feature = "proxy")]

mod app;
mod route;

pub use app::{
    create_app,
    MonoProxyAppState as AppState,
};
pub use route::handler as proxy_handler;
