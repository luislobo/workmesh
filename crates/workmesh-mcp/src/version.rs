pub const FULL: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+git.",
    env!("WORKMESH_GIT_COUNT"),
    ".",
    env!("WORKMESH_GIT_SHA"),
    env!("WORKMESH_GIT_DIRTY")
);
