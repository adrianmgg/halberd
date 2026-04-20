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

    let allows = "allow(unused, non_camel_case_types, non_upper_case_globals, clippy::upper_case_acronyms, clippy::enum_variant_names, clippy::doc_markdown, clippy::wildcard_imports)";
    let prelude = "use crate::{spv::{self, operand_kind as ok}, iil::{self, block}, types};";
    mods.iil().vis("pub").attr(allows);
    mods.iil_flat().vis("pub").scope().raw(prelude);
    mods.iil_f_instructions().vis("pub").scope().raw(prelude);
    mods.spv().vis("pub").attr(allows);
    mods.spv_operandkind().vis("pub");
    mods.spv_instruction().vis("pub").scope().raw(prelude);

    // pull in the full namespace so we can define things manually there and have the
    // codegen'd structs still see them
    mods.spv_operandkind()
        .scope()
        .raw("use crate::spv::operand_kind::*;");
    for operand_kind in &grammar.operand_kinds {
        codegen_operand_kind(&mut mods, operand_kind);
    }

    codegen_instructions(&grammar, &mut mods, grammar.instructions.as_slice());

    std::fs::write(&out_file, mods.root().to_string())
        .wrap_err_with(|| eyre!("failed to write generated code to {out_file:?}"))?;

    Ok(())
}

// quick wrapper so i dont have to write these manually and maybe typo stuff
// is this the best way to do this? probably not. does it work? absolutely yes
struct Modules(codegen::Scope);
impl Modules {
    fn root(&mut self) -> &mut codegen::Scope { &mut self.0 }

    fn iil(&mut self) -> &mut codegen::Module { self.root().get_or_new_module("iil") }

    fn iil_flat(&mut self) -> &mut codegen::Module { self.iil().get_or_new_module("flat") }

    fn iil_f_instructions(&mut self) -> &mut codegen::Module {
        self.iil_flat().get_or_new_module("instruction")
    }

    fn spv(&mut self) -> &mut codegen::Module { self.root().get_or_new_module("spv") }

    fn spv_operandkind(&mut self) -> &mut codegen::Module {
        self.spv().get_or_new_module("operand_kind")
    }

    fn spv_instruction(&mut self) -> &mut codegen::Module {
        self.spv().get_or_new_module("instruction")
    }
}

struct CodegennedInstructionInfo<'a> {
    name: String,
    is_iil: bool,
    operands: CodegenOperands<'a>,
}
#[derive(Copy, Clone, PartialEq, Eq)]
enum InstructionRetKind {
    RetUntyped,
    RetTyped,
    Void,
}
struct CodegenOperands<'a> {
    ret_kind: InstructionRetKind,
    other_operands: Vec<CodegenOperand<'a>>,
}
struct CodegenOperand<'a> {
    raw: &'a spv_grammar::Operand,
    name: String,
    is_expr: bool,
    is_bitenum: bool,
}

fn codegen_instructions(
    grammar: &spv_grammar::Grammar,
    mods: &mut Modules,
    instructions: &[spv_grammar::Instruction],
) {
    let instruction_infos: Vec<_> = (instructions.iter())
        .map(|i| codegen_instruction(grammar, mods, i))
        .collect();

    // spv instruction enums
    for (spv_enum_name, f_enum_name, f_op_trait, irk) in [
        (
            "OpVoid",
            "OpVoid",
            "iil::f::IilOpVoid",
            InstructionRetKind::Void,
        ),
        (
            "OpRetUntyped",
            "OpExprUntyped",
            "iil::f::IilOpExprUntyped",
            InstructionRetKind::RetUntyped,
        ),
        (
            "OpRetTyped",
            "OpExpr",
            "iil::f::IilOpExpr",
            InstructionRetKind::RetTyped,
        ),
    ] {
        let spv_enum = mods.spv().new_enum(spv_enum_name).vis("pub");
        for inst in &instruction_infos {
            if inst.operands.ret_kind != irk {
                continue;
            }
            spv_enum
                .new_variant(&inst.name)
                .tuple(format!("instruction::{}", &inst.name));
        }

        fn gen_from_impl(
            module: &mut codegen::Module,
            enum_name: &str,
            instruction_ns: &str,
            inst: &CodegennedInstructionInfo<'_>,
        ) {
            let from_impl = module
                .new_impl(enum_name)
                .impl_trait(format!("From<{instruction_ns}::{}>", &inst.name));
            from_impl
                .new_fn("from")
                .arg("x", format!("{instruction_ns}::{}", &inst.name))
                .ret(enum_name)
                .line(format!("{enum_name}::{variant}(x)", variant = &inst.name));
        }
        for inst in &instruction_infos {
            if inst.operands.ret_kind != irk {
                continue;
            }
            gen_from_impl(mods.spv(), spv_enum_name, "instruction", inst);
            if inst.is_iil {
                gen_from_impl(mods.iil_flat(), f_enum_name, "instruction", inst);
            }
        }

        let f_op_enum = (mods.iil_flat())
            .new_enum(f_enum_name)
            .vis("pub")
            .derive("Debug");
        instruction_infos
            .iter()
            .filter(|ii| ii.is_iil && ii.operands.ret_kind == irk)
            .for_each(|ii| {
                f_op_enum
                    .new_variant(&ii.name)
                    .tuple(format!("instruction::{}", ii.name));
            });

        let f_op_enum_renumberable = (mods.iil_flat())
            .new_impl(f_enum_name)
            .impl_trait("block::Renumberable");
        let f_op_enum_renumber = f_op_enum_renumberable
            .new_fn("renumber")
            .arg_mut_self()
            .arg("from", "block::BlockLocalRef")
            .arg("to", "block::BlockLocalRef");
        f_op_enum_renumber.line("match self {");
        instruction_infos
            .iter()
            .filter(|ii| ii.is_iil && ii.operands.ret_kind == irk)
            .for_each(|ii| {
                f_op_enum_renumber.line(format!(
                    "Self::{name}(o) => o.renumber(from, to),",
                    name = ii.name
                ));
            });
        f_op_enum_renumber.line("}");

        let f_op_enum_impl = (mods.iil_flat())
            .new_impl(f_enum_name)
            .impl_trait(f_op_trait);
        let f_op_enum_impl_into = f_op_enum_impl
            .new_fn(match irk {
                InstructionRetKind::RetTyped => "into_spv_expr",
                InstructionRetKind::RetUntyped => "into_spv_retuntyped",
                InstructionRetKind::Void => "into_spv_void",
            })
            .generic("MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef");
        if matches!(irk, InstructionRetKind::RetTyped) {
            f_op_enum_impl_into.generic("MapTypes: Fn(types::Type) -> ok::IdResultType");
        }
        f_op_enum_impl_into.arg_self().arg("map_refs", "MapRefs");
        if matches!(irk, InstructionRetKind::RetTyped) {
            f_op_enum_impl_into.arg("map_types", "MapTypes");
        }
        f_op_enum_impl_into.ret(format!("spv::{spv_enum_name}"));
        let argstr = match irk {
            InstructionRetKind::RetTyped => "map_refs, map_types",
            InstructionRetKind::RetUntyped => "map_refs",
            InstructionRetKind::Void => "map_refs",
        };
        let intospvfn = match irk {
            InstructionRetKind::RetUntyped => "into_spv_retuntyped",
            InstructionRetKind::RetTyped => "into_spv_expr",
            InstructionRetKind::Void => "into_spv_void",
        };
        f_op_enum_impl_into.line("match self {");
        instruction_infos
            .iter()
            .filter(|ii| ii.is_iil && ii.operands.ret_kind == irk)
            .for_each(|ii| {
                f_op_enum_impl_into.line(format!(
                    "Self::{name}(x) => x.{intospvfn}({argstr}),",
                    name = ii.name
                ));
            });
        f_op_enum_impl_into.line("}");

        if matches!(irk, InstructionRetKind::RetTyped) {
            let f_op_enum_impl_rt = f_op_enum_impl
                .new_fn("ret_type")
                .arg_ref_self()
                .ret("&types::Type");
            f_op_enum_impl_rt.line("match self {");
            instruction_infos
                .iter()
                .filter(|ii| ii.is_iil && ii.operands.ret_kind == irk)
                .for_each(|ii| {
                    f_op_enum_impl_rt
                        .line(format!("Self::{name}(x) => x.ret_type(),", name = ii.name));
                });
            f_op_enum_impl_rt.line("}");
        }
    }
}

fn codegen_instruction<'a>(
    grammar: &spv_grammar::Grammar,
    mods: &mut Modules,
    instruction: &'a spv_grammar::Instruction,
) -> CodegennedInstructionInfo<'a> {
    let cg_operands = instruction
        .operands
        .as_ref()
        .map(|operands| {
            // strip off the return value related operands
            let (ret_kind, skip_by) = if operands.first().is_some_and(|o| o.kind == "IdResultType")
                && operands.get(1).is_some_and(|o| o.kind == "IdResult")
            {
                (InstructionRetKind::RetTyped, 2)
            } else if operands.first().is_some_and(|o| o.kind == "IdResult") {
                (InstructionRetKind::RetUntyped, 1)
            } else {
                (InstructionRetKind::Void, 0)
            };
            // ensure the return-related operands look how we expect
            operands.iter().take(skip_by).for_each(|o| {
                assert!(matches!(o.kind.as_ref(), "IdResult" | "IdResultType"));
                assert!(o.quantifier.is_none());
            });
            // ensure none of the remaining operands are return-related
            if (operands.iter().skip(skip_by))
                .any(|operand| matches!(operand.kind.as_ref(), "IdResult" | "IdResultType"))
            {
                panic!("unexpected placement of result-related operands in {instruction:?}");
            }
            let other_operands = (operands.iter().skip(skip_by).enumerate())
                .map(|(idx, o)| CodegenOperand {
                    raw: o,
                    name: format!("op{idx}"),
                    is_expr: matches!(o.kind.as_ref(), "IdRef"),
                    is_bitenum: grammar.operand_kinds.iter().any(|ok| matches!(ok, spv_grammar::OperandKind::BitEnum { kind, .. } if kind == &o.kind)),
                })
                .collect();
            CodegenOperands { ret_kind, other_operands }
        })
        .unwrap_or_else(|| CodegenOperands {
            ret_kind: InstructionRetKind::Void,
            other_operands: Vec::new(),
        });

    let r#mod = Modules::spv_instruction;
    let name = ensure_valid_ident(&instruction.opname);
    let inst_struct = r#mod(mods).new_struct(&name).vis("pub").derive("Debug");
    // FIXME extensions
    // FIXME version
    match cg_operands.ret_kind {
        InstructionRetKind::Void => {}
        InstructionRetKind::RetUntyped => {
            // inst_struct.new_field("ret_id", "ok::IdResult").vis("pub");
        }
        InstructionRetKind::RetTyped => {
            // inst_struct.new_field("ret_id", "ok::IdResult").vis("pub");
            inst_struct
                .new_field("ret_type", "ok::IdResultType")
                .vis("pub");
        }
    }
    for operand in &cg_operands.other_operands {
        let mut op_type = format!("ok::{}", ensure_valid_ident(&operand.raw.kind));
        if operand.is_bitenum {
            op_type = format!("::enumset::EnumSet<{op_type}>");
        }
        match operand.raw.quantifier {
            Some(spv_grammar::Quantifier::ZeroOrOne) => op_type = format!("Option<{op_type}>"),
            Some(spv_grammar::Quantifier::ZeroOrMore) => op_type = format!("Vec<{op_type}>"),
            _ => {}
        }
        let field = inst_struct.new_field(&operand.name, op_type).vis("pub");
        if let Some(doc) = operand.raw.name.as_ref() {
            field.doc(doc);
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

    // we handle types and constants separately & insert them in the final f-iil -> il phase,
    // so no need to output those instructions at all
    let should_generate_iil = !(matches!(instruction.class.as_ref(), "Constant-Creation")
        || matches!(instruction.opname.as_ref(), "OpFunction" | "OpFunctionEnd"));

    if should_generate_iil {
        let fiil_struct = mods
            .iil_f_instructions()
            .new_struct(&name)
            .vis("pub")
            .derive("Debug");
        match cg_operands.ret_kind {
            InstructionRetKind::Void => {}
            InstructionRetKind::RetUntyped => {}
            InstructionRetKind::RetTyped => {
                fiil_struct.new_field("ret_type", "types::Type").vis("pub");
            }
        }
        for operand in &cg_operands.other_operands {
            // FIXME should probably just put the string of the type into our operand struct
            //       instead of doing the same work in 3 different places
            let mut op_type = match operand.is_expr {
                true => "iil::block::BlockLocalRef".into(),
                false => format!("ok::{}", operand.raw.kind),
            };
            if operand.is_bitenum {
                op_type = format!("::enumset::EnumSet<{op_type}>");
            }
            match operand.raw.quantifier {
                Some(spv_grammar::Quantifier::ZeroOrOne) => op_type = format!("Option<{op_type}>"),
                Some(spv_grammar::Quantifier::ZeroOrMore) => op_type = format!("Vec<{op_type}>"),
                _ => {}
            }
            fiil_struct.new_field(&operand.name, op_type).vis("pub");
        }

        // impl block::Renumberable for this f-iil instruction
        let renumberable = mods
            .iil_f_instructions()
            .new_impl(&name)
            .impl_trait("block::Renumberable");
        let renumber = renumberable
            .new_fn("renumber")
            .arg_mut_self()
            .arg("from", "block::BlockLocalRef")
            .arg("to", "block::BlockLocalRef");
        for operand in &cg_operands.other_operands {
            if operand.is_expr {
                renumber.line(format!("self.{}.renumber(from, to);", &operand.name));
            }
        }

        // impl iilop
        let impl_iil_op_x = mods.iil_f_instructions().new_impl(&name);
        impl_iil_op_x.impl_trait(match cg_operands.ret_kind {
            InstructionRetKind::RetUntyped => "iil::flat::IilOpExprUntyped",
            InstructionRetKind::RetTyped => "iil::flat::IilOpExpr",
            InstructionRetKind::Void => "iil::flat::IilOpVoid",
        });

        if matches!(cg_operands.ret_kind, InstructionRetKind::RetTyped) {
            impl_iil_op_x
                .new_fn("ret_type")
                .arg_ref_self()
                .ret("&types::Type")
                .line("&self.ret_type");
        }

        let into_spv_fn = impl_iil_op_x.new_fn(match cg_operands.ret_kind {
            InstructionRetKind::RetUntyped => "into_spv_retuntyped",
            InstructionRetKind::RetTyped => "into_spv_expr",
            InstructionRetKind::Void => "into_spv_void",
        });
        into_spv_fn.generic("MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef");
        if matches!(cg_operands.ret_kind, InstructionRetKind::RetTyped) {
            into_spv_fn.generic("MapTypes: Fn(types::Type) -> ok::IdResultType");
        }
        into_spv_fn.arg_self();
        into_spv_fn.arg("map_refs", "MapRefs");
        if matches!(cg_operands.ret_kind, InstructionRetKind::RetTyped) {
            into_spv_fn.arg("map_types", "MapTypes");
        }
        into_spv_fn.ret(match cg_operands.ret_kind {
            InstructionRetKind::RetUntyped => "spv::OpRetUntyped",
            InstructionRetKind::RetTyped => "spv::OpRetTyped",
            InstructionRetKind::Void => "spv::OpVoid",
        });
        into_spv_fn.line(format!("spv::instruction::{name} {{"));
        if matches!(cg_operands.ret_kind, InstructionRetKind::RetTyped) {
            into_spv_fn.line("    ret_type: map_types(self.ret_type),");
        }
        for operand in &cg_operands.other_operands {
            if operand.is_expr {
                match operand.raw.quantifier {
                    Some(spv_grammar::Quantifier::ZeroOrOne) => {
                        into_spv_fn.line(format!(
                            "    {name}: self.{name}.map(&map_refs),",
                            name = &operand.name
                        ));
                    }
                    Some(spv_grammar::Quantifier::ZeroOrMore) => {
                        into_spv_fn.line(format!(
                            "    {name}: self.{name}.into_iter().map(map_refs).collect(),",
                            name = &operand.name
                        ));
                    }
                    None => {
                        into_spv_fn.line(format!(
                            "    {name}: map_refs(self.{name}),",
                            name = &operand.name
                        ));
                    }
                }
            } else {
                into_spv_fn.line(format!("    {name}: self.{name},", name = &operand.name));
            }
        }
        into_spv_fn.line("}.into()");
    }

    CodegennedInstructionInfo {
        name: name.into(),
        is_iil: should_generate_iil,
        operands: cg_operands,
    }
}

fn codegen_operand_kind(mods: &mut Modules, operand_kind: &spv_grammar::OperandKind) {
    let r#mod = Modules::spv_operandkind;
    match operand_kind {
        // https://registry.khronos.org/SPIR-V/specs/unified1/MachineReadableGrammar.html#bitenum-operand-kind
        spv_grammar::OperandKind::BitEnum { kind, enumerants } => {
            let name = ensure_valid_ident(kind);
            let e = r#mod(mods)
                .new_enum(&name)
                .vis("pub")
                .repr("u32")
                .derive("Debug")
                .derive("::enumset::EnumSetType")
                .r#macro(r##"#[enumset(repr = "u32", map = "mask")]"##);
            let mut zero = None;
            for enumerant in enumerants {
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
            let name = ensure_valid_ident(kind);
            let e = r#mod(mods).new_enum(&name).vis("pub");
            match kind.as_ref() {
                "Capability" => {
                    e.repr("u32")
                        .derive("Debug")
                        .derive("::enumset::EnumSetType")
                        .r#macro(r##"#[enumset(map = "compact")]"##);
                }
                _ => {
                    e.repr("u32").derive("Debug,Copy,Clone,PartialEq,Eq,Hash");
                }
            }
            for enumerant in enumerants {
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
                .new_struct(ensure_valid_ident(kind))
                .vis("pub")
                .doc(clean_doc(doc))
                .derive("Debug,Copy,Clone,Hash,PartialEq,Eq")
                .tuple_field("pub u32");
        }
        spv_grammar::OperandKind::Literal { kind: _, doc: _ } => {
            // (these are each defined manually instead)
            // TODO maybe write something here that will complain if the manually-defined struct is missing?
        }
        spv_grammar::OperandKind::Composite { kind, bases } => {
            // TODO should maybe just be a type alias to a tuple?
            let t = r#mod(mods)
                .new_struct(ensure_valid_ident(kind))
                .vis("pub")
                .derive("Debug");
            for base in bases {
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

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Deserialize)]
    pub struct InstructionPrintingClass {
        // TODO
    }

    #[derive(Debug, Deserialize)]
    pub struct Instruction {
        pub opname: String,
        pub opcode: u32,
        pub operands: Option<Vec<Operand>>,
        pub capabilities: Option<Vec<String>>,
        pub class: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Operand {
        pub kind: String,
        pub quantifier: Option<Quantifier>,
        /// "A short descriptive name for this operand."
        pub name: Option<String>,
    }

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub enum Quantifier {
        #[serde(rename = "?")]
        ZeroOrOne,
        #[serde(rename = "*")]
        ZeroOrMore,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "category")]
    pub enum OperandKind {
        BitEnum { kind: String, enumerants: Vec<BitEnumerant> },
        ValueEnum { kind: String, enumerants: Vec<ValueEnumerant> },
        Id { kind: String, doc: String },
        Literal { kind: String, doc: String },
        Composite { kind: String, bases: Vec<String> },
    }

    #[derive(Debug, Deserialize)]
    pub struct BitEnumerant {
        pub enumerant: String,
        pub aliases: Option<Vec<String>>,
        #[serde(deserialize_with = "hex_literal")]
        pub value: u32,
        pub version: Option<String>,
        pub capabilities: Option<Vec<String>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ValueEnumerant {
        pub enumerant: String,
        pub aliases: Option<Vec<String>>,
        pub value: u32,
        pub version: Option<String>,
        pub capabilities: Option<Vec<String>>,
    }

    fn hex_literal<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where D: de::Deserializer<'de> {
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
