pub mod auth;
pub mod credentials;
pub mod dashboard;
pub mod logs;
pub mod networks;
pub mod nodes;
pub mod system;
pub mod users;

use axum::Router;

use crate::state::SharedState;

pub fn api_router() -> Router<SharedState> {
    Router::new()
        .merge(auth::router())
        .merge(users::router())
        .merge(networks::router())
        .merge(nodes::router())
        .merge(credentials::router())
        .merge(logs::router())
        .merge(dashboard::router())
        .merge(system::router())
}
