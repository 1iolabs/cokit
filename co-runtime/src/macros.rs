/// Any wasmer backend enabled?
macro_rules! cfg_wasmer {
    ($($item:item)*) => {
        $(
            #[cfg(any(
                feature = "headless",
                feature = "llvm",
                feature = "cranelift",
                feature = "wasmi",
                feature = "wamr",
                feature = "js",
                all(feature = "jsc", target_vendor = "apple"),
            ))]
            $item
        )*
    };
}

pub(crate) use cfg_wasmer;
