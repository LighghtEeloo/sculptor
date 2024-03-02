#[macro_export]
macro_rules! submod {
    ( $( $m:ident ),* ) => {
        $(
            pub mod $m;
            pub use $m::*;
        )*
    };
}

submod!(project_info);
submod!(file_io);
