use std::{env, fs, path::Path};

use eyre::ContextCompat;

fn main() -> eyre::Result<()> {
    let out_dir = env::var_os("OUT_DIR").wrap_err("OUT_DIR environment variable was missing???")?;
    let dest_path = Path::new(&out_dir).join("generated.rs");
    fs::write(&dest_path, "")?;
    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}

fn generate() -> eyre::Result<String> {
    let mut scope = codegen::Scope::new();

    Ok(scope.to_string())
}

mod spirv_grammar {
    use serde::{Deserialize, Deserializer};

    #[derive(Debug, Deserialize)]
    pub struct Grammar {
        pub copyright: Vec<String>,
        #[serde(deserialize_with = "hex_literal")]
        pub magic_number: u32,
        pub major_version: u16,
        pub minor_version: u16,
        pub revision: u16,
    }

    fn hex_literal<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        use serde::de::Error;
        let s: &str = Deserialize::deserialize(deserializer)?;
        if !s.starts_with("0x") {
            return Err(Error::custom("hex literal must start with '0x'"));
        }
        let third_char = s
            .char_indices()
            .nth(2)
            .ok_or(Error::custom(
                "hex literal must have characters after its '0x' prefix",
            ))?
            .0;
        (s[third_char..]).parse().map_err(Error::custom)
    }
}
