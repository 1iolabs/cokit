// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

macro_rules! cfg_wasmer {
    ($($item:item)*) => {
        $(
            #[cfg(any(
                feature = "headless",
                feature = "llvm",
                feature = "cranelift",
                feature = "wasmi",
                feature = "wamr",
                all(feature = "js", target_arch = "wasm32"),
                all(feature = "jsc", target_vendor = "apple"),
            ))]
            $item
        )*
    };
}

pub(crate) use cfg_wasmer;
