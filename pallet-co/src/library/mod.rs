// macros
#[macro_export]
macro_rules! export {
    ( $x:ident ) => {
        mod $x; pub use $x::*;
    };
}

// exports
export!(input_read_to_end);
// export!(ipld_value);
export!(reference);
