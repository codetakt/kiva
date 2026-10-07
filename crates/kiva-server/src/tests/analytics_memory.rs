#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn analytics_surface_is_governed_in_memory() -> Result<(), String> {
    assert_governed_analytics_boundary(&super::AppState::default()).await
}
