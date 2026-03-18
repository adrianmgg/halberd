use std::{borrow::Cow, collections::HashMap, path::Path};

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

    let mut mods = Modules(codegen::Scope::new());

    mods.iil()
        .vis("pub")
        .attr("allow(non_camel_case_types,non_upper_case_globals,unused)");
    mods.spv()
        .vis("pub")
        .attr("allow(non_camel_case_types,non_upper_case_globals,unused)");
    mods.spv_operandkind().vis("pub");
    mods.spv_instruction()
        .vis("pub")
        .scope()
        .raw("use crate::spv::operand_kind as ok;");

    // pull in the full namespace so we can define things manually there and have the
    // codegen'd structs still see them
    mods.spv_operandkind()
        .scope()
        .raw("use crate::spv::operand_kind::*;");
    for operand_kind in grammar.operand_kinds {
        codegen_operand_kind(&mut mods, operand_kind);
    }

    for instruction in grammar.instructions {
        codegen_instruction(&mut mods, &instruction);
    }

    std::fs::write(&out_file, mods.root().to_string())
        .wrap_err_with(|| eyre!("failed to write generated code to {out_file:?}"))?;

    Ok(())
}

// quick wrapper so i dont have to write these manually and maybe typo stuff
// is this the best way to do this? probably not. does it work? absolutely yes
struct Modules(codegen::Scope);
impl Modules {
    fn root(&mut self) -> &mut codegen::Scope {
        &mut self.0
    }
    fn iil(&mut self) -> &mut codegen::Module {
        self.root().get_or_new_module("iil")
    }
    fn ill_flat(&mut self) -> &mut codegen::Module {
        self.iil().get_or_new_module("f")
    }
    fn ill_hierarchical(&mut self) -> &mut codegen::Module {
        self.iil().get_or_new_module("h")
    }
    fn spv(&mut self) -> &mut codegen::Module {
        self.root().get_or_new_module("spv")
    }
    fn spv_operandkind(&mut self) -> &mut codegen::Module {
        self.spv().get_or_new_module("operand_kind")
    }
    fn spv_instruction(&mut self) -> &mut codegen::Module {
        self.spv().get_or_new_module("instruction")
    }
}

fn codegen_instruction(mods: &mut Modules, instruction: &spv_grammar::Instruction) {
    let r#mod = Modules::spv_instruction;
    let name = ensure_valid_ident(&instruction.opname);
    let inst_struct = r#mod(mods).new_struct(&name).vis("pub").derive("Debug");
    // FIXME extensions
    // FIXME version
    if let Some(operands) = &instruction.operands {
        for operand in operands {
            let mut op_type = format!("ok::{}", ensure_valid_ident(&operand.kind));
            match operand.quantifier {
                Some(spv_grammar::Quantifier::ZeroOrOne) => op_type = format!("Option<{op_type}>"),
                Some(spv_grammar::Quantifier::ZeroOrMore) => op_type = format!("Vec<{op_type}>"),
                _ => {}
            }
            let op_doc = operand
                .name
                .as_ref()
                .map(|name| format!("#[doc = {name:?}]"))
                .unwrap_or_default();
            inst_struct.tuple_field(format!("{op_doc} {op_type}"));
        }
    }
    // FIXME does this need to include our operands' capabilities too?
    codegen_hascapabilities(r#mod(mods), &name, |function| {
        function.line(codegen_capability_set(
            instruction
                .capabilities
                .as_ref()
                .map(|v| &v[..])
                .unwrap_or_default(),
        ));
    });

    let impl_instruction = r#mod(mods)
        .new_impl(&name)
        .impl_trait("crate::spv::Instruction");
    impl_instruction
        .new_fn("opcode")
        .arg_ref_self()
        .ret("u32")
        .line(instruction.opcode);
}

fn codegen_operand_kind(mods: &mut Modules, operand_kind: spv_grammar::OperandKind) {
    let r#mod = Modules::spv_operandkind;
    match operand_kind {
        // https://registry.khronos.org/SPIR-V/specs/unified1/MachineReadableGrammar.html#bitenum-operand-kind
        spv_grammar::OperandKind::BitEnum { kind, enumerants } => {
            let name = ensure_valid_ident(&kind);
            let e = r#mod(mods)
                .new_enum(&name)
                .vis("pub")
                .repr("u32")
                .derive("Debug")
                .derive("::enumset::EnumSetType")
                .r#macro(r##"#[enumset(repr = "u32", map = "mask")]"##);
            let mut zero = None;
            for enumerant in &enumerants {
                if enumerant.value == 0 {
                    zero = Some(enumerant);
                } else {
                    e.new_variant(ensure_valid_ident(&enumerant.enumerant))
                        .discriminant(enumerant.value);
                }
            }
            if let Some(zero) = zero {
                r#mod(mods).new_impl(&name).associate_const(
                    &zero.enumerant,
                    "::enumset::EnumSet<Self>",
                    "::enumset::EnumSet::empty()",
                    "pub",
                );
            }
            // FIXME needs capabilities
        }
        spv_grammar::OperandKind::ValueEnum { kind, enumerants } => {
            let name = ensure_valid_ident(&kind);
            let e = r#mod(mods).new_enum(&name).vis("pub");
            match kind.as_ref() {
                "Capability" => {
                    e.repr("u32")
                        .derive("Debug")
                        .derive("::enumset::EnumSetType")
                        .r#macro(r##"#[enumset(map = "compact")]"##);
                }
                _ => {
                    e.repr("u32").derive("Debug").derive("Copy").derive("Clone");
                }
            }
            for enumerant in &enumerants {
                // FIXME need to use version,capabilities
                // TODO should use aliases
                e.new_variant(ensure_valid_ident(&enumerant.enumerant))
                    .discriminant(enumerant.value);
            }

            codegen_hascapabilities(r#mod(mods), &name, |function| {
                // group the variants which share capabilities together so we can write them in a
                // single `a | b | c | d => foo` match case
                let mut cap2cases = HashMap::new();
                for enumerant in enumerants {
                    let name = ensure_valid_ident(&enumerant.enumerant);
                    let caps: Vec<_> = enumerant.capabilities.iter().flatten().collect();
                    let expr = codegen_capability_set(&caps);
                    cap2cases
                        .entry(expr)
                        .or_insert_with(Vec::new)
                        .push(format!("Self::{name}"));
                }
                if cap2cases.len() == 1 {
                    // if homogeneous, don't bother writing a match, just put the expr directly
                    function.line(cap2cases.iter().next().unwrap().0);
                } else {
                    function.line("match self {");
                    for (expr, patterns) in cap2cases.iter() {
                        function.line(format!("    {} => {expr},", patterns.join(" | ")));
                    }
                    function.line("}");
                }
            });
        }
        spv_grammar::OperandKind::Id { kind, doc } => {
            // "An <id> always consumes one word."
            // - SPIR-V Specification v1.6r7, 2.2.1
            // "For an operand kind belonging to this category, its value is an <id> definition or reference."
            // - SPIR-V Machine-readable Grammar, 3.2
            r#mod(mods)
                .new_struct(ensure_valid_ident(&kind))
                .vis("pub")
                .doc(clean_doc(&doc))
                .derive("Debug")
                .tuple_field("pub u32");
        }
        spv_grammar::OperandKind::Literal { kind: _, doc: _ } => {
            // (these are each defined manually instead)
            // TODO maybe write something here that will complain if the manually-defined struct is missing?
        }
        spv_grammar::OperandKind::Composite { kind, bases } => {
            // TODO should maybe just be a type alias to a tuple?
            let t = r#mod(mods)
                .new_struct(ensure_valid_ident(&kind))
                .vis("pub")
                .derive("Debug");
            for base in &bases {
                t.tuple_field(format!("pub {}", ensure_valid_ident(base)));
            }
        }
    }
}

fn codegen_hascapabilities<F: FnOnce(&mut codegen::Function)>(
    module: &mut codegen::Module,
    target: &str,
    f: F,
) {
    let has_cap_impl = module
        .new_impl(target)
        .impl_trait("crate::spv::HasCapabilities");
    let has_cap_fn = has_cap_impl
        .new_fn("capabilities")
        .arg_ref_self()
        .ret("::enumset::EnumSet<crate::spv::operand_kind::Capability>");
    f(has_cap_fn);
}

fn codegen_capability_set<S: AsRef<str>>(capabilities: &[S]) -> String {
    const PREFIX: &str = "crate::spv::operand_kind::Capability::";
    match capabilities {
        [] => "::enumset::EnumSet::empty()".into(),
        [one] => format!("{PREFIX}{}.into()", ensure_valid_ident(one.as_ref())),
        multiple => multiple
            .iter()
            .map(|s| format!("{PREFIX}{}", ensure_valid_ident(s.as_ref())))
            .collect::<Vec<_>>()
            .join(" | "),
    }
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

fn clean_doc(s: &'_ str) -> Cow<'_, str> {
    if s.contains("<id>") {
        Cow::Owned(s.replace("<id>", "`<id>`"))
    } else {
        Cow::Borrowed(s)
    }
}

#[allow(unused)]
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
        pub opname: String,
        // TODO double check this is a u32
        pub opcode: u32,
        pub operands: Option<Vec<Operand>>,
        pub capabilities: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    pub struct Operand {
        pub kind: String,
        pub quantifier: Option<Quantifier>,
        /// "A short descriptive name for this operand."
        pub name: Option<String>,
    }

    #[derive(Deserialize)]
    pub enum Quantifier {
        #[serde(rename = "?")]
        ZeroOrOne,
        #[serde(rename = "*")]
        ZeroOrMore,
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

    fn hex_literal<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: de::Deserializer<'de>,
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
        u32::from_str_radix(&s[third_char..], 16).map_err(Error::custom)
    }
}
