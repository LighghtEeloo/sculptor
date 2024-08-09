#[macro_export]
macro_rules! submod {
    ( $( $m:ident ),* ) => {
        $(
            pub mod $m;
            pub use $m::*;
        )*
    };
}

#[cfg(feature = "project_info")]
submod!(project_info);
#[cfg(feature = "file_io")]
submod!(file_io);
#[cfg(feature = "sha_snap")]
submod!(sha_snap);

// diff
// watch