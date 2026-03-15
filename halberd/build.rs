use std::{borrow::Cow, path::Path};

use check_keyword::CheckKeyword;
use eyre::{Context, eyre};

const GRAMMAR_FILE_VAR: &str = "SPIRV_GRAMMAR_JSON";
const OUT_FILE: &str = "generated.rs";

fn main() -> eyre::Result<()> {
    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo::rerun-if-env-changed=OUT_DIR");
    let out_dir = std::env::var("OUT_DIR").wrap_err("env var OUT_DIR not set")?;
    let out_file = Path::new(&out_dir).join(OUT_FILE);

    println!("cargo::rerun-if-env-changed={GRAMMAR_FILE_VAR}");
    let grammar_path = std::env::var(GRAMMAR_FILE_VAR)
        .wrap_err_with(|| eyre!("env var {GRAMMAR_FILE_VAR} not set"))?;
    println!("cargo::rerun-if-changed={grammar_path}");

    let grammar_file = std::fs::File::open(grammar_path)?;
    let grammar: spv_grammar::Grammar =
        serde_json::from_reader(std::io::BufReader::new(grammar_file))?;

    let mut scope = codegen::Scope::new();

    let m_spv = scope.new_module("spv").vis("pub");
    let m_operand_kinds = m_spv.new_module("operand_kind").vis("pub");
    for operand_kind in grammar.operand_kinds {
        match operand_kind {
            spv_grammar::OperandKind::BitEnum { kind, enumerants } => {
                // TODO
            }
            spv_grammar::OperandKind::ValueEnum { kind, enumerants } => {
                let name = ensure_valid_ident(&kind);
                let e = m_operand_kinds
                    .new_enum(&name)
                    .vis("pub")
                    .repr("u32")
                    .derive("Debug")
                    .derive("Copy")
                    .derive("Clone");
                for enumerant in &enumerants {
                    // FIXME need to use version,capabilities
                    // TODO should use aliases
                    e.new_variant(ensure_valid_ident(&enumerant.enumerant))
                        .discriminant(enumerant.value);
                }
                let has_cap_impl = m_operand_kinds
                    .new_impl(&name)
                    .impl_trait("crate::spv::HasCapabilities");
                let has_cap_fn = has_cap_impl
                    .new_fn("capabilities")
                    .arg_ref_self()
                    .ret("impl Iterator<Item = Capability>")
                    .line("match self {");
                for enumerant in enumerants {
                    let name = ensure_valid_ident(&enumerant.enumerant);
                    let caps: Vec<_> = enumerant
                        .capabilities
                        .iter()
                        .flatten()
                        .map(|cap| format!("Capability::{}", ensure_valid_ident(cap)))
                        .collect();
                    has_cap_fn.line(format!(
                        "Self::{name} => [{}].iter().copied(),",
                        caps.join(",")
                    ));
                }
                has_cap_fn.line("}");
            }
            spv_grammar::OperandKind::Id { kind, doc } => {
                // TODO
            }
            spv_grammar::OperandKind::Literal { kind, doc } => {
                // TODO
            }
            spv_grammar::OperandKind::Composite { kind, bases } => {
                // TODO
            }
        }
    }

    std::fs::write(&out_file, scope.to_string())
        .wrap_err_with(|| eyre!("failed to write generated code to {out_file:?}"))?;

    Ok(())
}

fn ensure_valid_ident(s: &'_ str) -> Cow<'_, str> {
    let mut s = if s.is_keyword() {
        Cow::Owned(s.into_safe())
    } else {
        Cow::Borrowed(s)
    };
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s = Cow::Owned(format!("_{s}"));
    }
    s
}

mod spv_grammar {
    use serde::{Deserialize, de};

    #[derive(Deserialize)]
    pub struct Grammar {
        pub copyright: Vec<String>,
        #[serde(deserialize_with = "hex_literal")]
        pub magic_number: u32,
        // TODO: figure out if there's a spec-defined int type we should be using for
        //       major/minor/rev
        pub major_version: u32,
        pub minor_version: u32,
        pub revision: u32,
        pub instruction_printing_class: Vec<InstructionPrintingClass>,
        pub instructions: Vec<Instruction>,
        pub operand_kinds: Vec<OperandKind>,
    }

    #[derive(Deserialize)]
    pub struct InstructionPrintingClass {
        // TODO
    }

    #[derive(Deserialize)]
    pub struct Instruction {
        // TODO
    }

    #[derive(Deserialize)]
    #[serde(tag = "category")]
    pub enum OperandKind {
        BitEnum {
            kind: String,
            enumerants: Vec<BitEnumerant>,
        },
        ValueEnum {
            kind: String,
            enumerants: Vec<ValueEnumerant>,
        },
        Id {
            kind: String,
            doc: String,
        },
        Literal {
            kind: String,
            doc: String,
        },
        Composite {
            kind: String,
            bases: Vec<String>,
        },
    }

    #[derive(Deserialize)]
    pub struct BitEnumerant {
        pub enumerant: String,
        pub aliases: Option<Vec<String>>,
        #[serde(deserialize_with = "hex_literal")]
        pub value: u32,
        pub version: Option<String>,
        pub capabilities: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    pub struct ValueEnumerant {
        pub enumerant: String,
        pub aliases: Option<Vec<String>>,
        pub value: u32,
        pub version: Option<String>,
        pub capabilities: Option<Vec<String>>,
    }

    fn hex_literal<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: de::Deserializer<'de>,
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        use serde::de::Error;
        let s: String = Deserialize::deserialize(deserializer)?;
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
