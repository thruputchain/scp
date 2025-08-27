use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parse;
use syn::{
    Block, Expr, ExprBlock, ExprClosure, ItemFn, LitStr, Local, Macro, Pat, PatIdent, Stmt,
    StmtMacro, parse_quote,
};

#[proc_macro_attribute]
pub fn shfix(args: TokenStream, input: TokenStream) -> TokenStream {
    shfix_impl(args.into(), input.into()).into()
}

fn shfix_impl(_args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let input_fn: ItemFn = syn::parse2(input).expect("expected a function");

    let mut output = TokenStream2::new();

    match rewrite_block(None, &input_fn.block) {
        Rewritten::Fix(_) => {
            output.extend(quote! { #input_fn });
        }
        Rewritten::Exclusive(unfoldings) => {
            for (path, block) in unfoldings {
                let mut unfolded_fn = input_fn.clone();
                let name = format!("{}_{}", unfolded_fn.sig.ident, path.join("_"));
                unfolded_fn.sig.ident = syn::Ident::new(&name, input_fn.sig.ident.span());
                unfolded_fn.block = Box::new(block);
                output.extend(quote! { #unfolded_fn });
            }
        }
        Rewritten::ExclusiveOrFix(..) => todo!("XXX"),
    }

    output
}

type SectionPath = Vec<String>;

enum Rewritten<Q> {
    Fix(Q),
    Exclusive(Vec<(SectionPath, Q)>), // can use one Q, and no other exclusive rewrites can be activated
    ExclusiveOrFix(Vec<(SectionPath, Q)>, Q),
}

// rewrites each individual statement
fn rewrite_stmts(stmts: &Vec<Stmt>) -> Vec<Rewritten<Stmt>> {
    let mut rewritten = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Local(local) => match rewrite_local(local) {
                Rewritten::Fix(_) => rewritten.push(Rewritten::Fix(stmt.clone())),
                Rewritten::Exclusive(ls) => {
                    let unfolded_stmts = ls.into_iter().map(|(p, l)| (p, Stmt::Local(l))).collect();
                    rewritten.push(Rewritten::Exclusive(unfolded_stmts));
                }
                Rewritten::ExclusiveOrFix(exc, fix) => {
                    let exc_stmts = exc.into_iter().map(|(p, l)| (p, Stmt::Local(l))).collect();
                    rewritten.push(Rewritten::ExclusiveOrFix(exc_stmts, Stmt::Local(fix)));
                }
            },
            Stmt::Item(_) | Stmt::Expr(_, _) => rewritten.push(Rewritten::Fix(stmt.clone())),
            Stmt::Macro(stmt_macro) => {
                if let Some((id, description, section_block)) = parse_section_macro(stmt_macro) {
                    match rewrite_block(Some(&description), &section_block) {
                        Rewritten::Fix(_) => {
                            rewritten
                                .push(Rewritten::Exclusive(vec![(vec![id.clone()], stmt.clone())]));
                        }
                        Rewritten::Exclusive(rewritten_blocks) => {
                            rewritten.push(Rewritten::Exclusive(lift_blocks_to_stmts(
                                &id,
                                stmt_macro,
                                rewritten_blocks,
                            )));
                        }
                        Rewritten::ExclusiveOrFix(exc_blocks, fix_block) => {
                            rewritten.push(Rewritten::ExclusiveOrFix(
                                lift_blocks_to_stmts(&id, stmt_macro, exc_blocks),
                                lift_block_to_stmt(stmt_macro, fix_block),
                            ));
                        }
                    }
                } else {
                    rewritten.push(Rewritten::Fix(stmt.clone()))
                }
            }
        }
    }

    rewritten
}

fn lift_blocks_to_stmts(
    id: &String,
    stmt_macro: &StmtMacro,
    rewritten_blocks: Vec<(Vec<String>, Block)>,
) -> Vec<(Vec<String>, Stmt)> {
    let mut stmt_rewrites = vec![];
    for (rewritten_path, rewritten_block) in rewritten_blocks {
        let mut path = vec![id.clone()];
        path.extend(rewritten_path);
        stmt_rewrites.push((path, lift_block_to_stmt(stmt_macro, rewritten_block)));
    }
    stmt_rewrites
}

fn lift_block_to_stmt(stmt_macro: &StmtMacro, block: Block) -> Stmt {
    Stmt::Expr(
        Expr::Block(ExprBlock {
            attrs: stmt_macro.attrs.clone(),
            label: None,
            block: block,
        }),
        stmt_macro.semi_token.clone(),
    )
}

fn rewrite_local(local: &Local) -> Rewritten<Local> {
    if let Some((expr_closure, expr_block, block)) = parse_local_closure(local) {
        match rewrite_block(None, block) {
            Rewritten::Fix(_) => Rewritten::Fix(local.clone()),
            Rewritten::Exclusive(bs) => Rewritten::Exclusive(
                bs.into_iter()
                    .map(|(p, b)| {
                        let mut new_local = local.clone();
                        let new_expr_closure =
                            if let Expr::Closure(x) = &mut *new_local.init.as_mut().unwrap().expr {
                                x
                            } else {
                                panic!("bug")
                            };
                        let expr_block = Expr::Block(ExprBlock {
                            attrs: expr_block.attrs.clone(),
                            label: expr_block.label.clone(),
                            block: b,
                        });
                        new_expr_closure.body = Box::new(expr_block);
                        (p, new_local)
                    })
                    .collect(),
            ),
        }
    } else {
        Rewritten::Fix(local.clone())
    }
}

fn parse_local_closure(local: &Local) -> Option<(&ExprClosure, &ExprBlock, &Block)> {
    if let Some(local_init) = &local.init {
        if let Expr::Closure(expr_closure) = &*local_init.expr {
            if let Expr::Block(expr_block) = &*expr_closure.body {
                return Some((expr_closure, expr_block, &expr_block.block));
            }
        }
    }
    None
}

fn rewrite_block(description: Option<&String>, block: &Block) -> Rewritten<Block> {
    let rewritten_stmts = rewrite_stmts(&block.stmts);
    let unfolded_stmts = unfold_stmts(&rewritten_stmts);

    let header_stmt = description.map(|d| {
        let s: Stmt = parse_quote! {
            println!(#d);
        };
        s
    });

    let mut unfoldings = Vec::new();
    if unfolded_stmts.is_empty() {
        unfoldings.push((
            vec![],
            Block {
                brace_token: block.brace_token.clone(),
                stmts: if let Some(h) = header_stmt {
                    vec![h]
                } else {
                    vec![]
                },
            },
        ));
    } else {
        for (path, stmts) in unfold_stmts(&rewritten_stmts) {
            let mut new_stmts = if let Some(h) = &header_stmt {
                vec![h.clone()]
            } else {
                vec![]
            };
            new_stmts.extend(stmts);
            unfoldings.push((
                path.clone(),
                Block {
                    brace_token: block.brace_token.clone(),
                    stmts: new_stmts,
                },
            ));
        }
    }
    Rewritten::Exclusive(unfoldings)
}

fn _rewritten_stmts_to_str(stmts: &[Rewritten<Stmt>]) -> String {
    let mut r = String::new();
    for rw in stmts {
        match rw {
            Rewritten::Fix(s) => {
                r.push_str(&format!("fix({}) ", quote! { #s }));
            }
            Rewritten::Exclusive(u) => {
                r.push_str(&format!("exclusive("));
                for (path, stmt) in u {
                    r.push_str(&format!("{}: {} ¶", path.join("_"), quote! { #stmt }));
                }
                r.push_str(") ");
            }
            Rewritten::ExclusiveOrFix(u, x) => {
                r.push_str(&format!("exclusive_or_fix("));
                for (path, stmt) in u {
                    r.push_str(&format!("{}: {} ¶ ", path.join("_"), quote! { #stmt }));
                }
                r.push_str(&format!("fix: {}", quote! { #x }));
                r.push_str(") ");
            }
        }
    }
    r
}

fn _unfolded_stmts_to_str(u: &[(SectionPath, Vec<Stmt>)]) -> String {
    let mut r = String::new();
    for (path, ss) in u {
        r.push_str(&format!(
            "{}({:?}) ",
            path.join("_"),
            ss.iter()
                .map(|s| { format!("{}", quote! { #s }) })
                .collect::<Vec<String>>()
                .join(" ¶ ")
        ));
    }
    r
}

fn unfold_stmts(stmts: &[Rewritten<Stmt>]) -> Vec<(SectionPath, Vec<Stmt>)> {
    if stmts.is_empty() {
        vec![(SectionPath::new(), Vec::new())]
    } else {
        match &stmts.last().unwrap() {
            Rewritten::Fix(last_stmt) => unfold_stmts(&stmts[..stmts.len() - 1])
                .into_iter()
                .map(|(p, mut vs)| {
                    vs.push(last_stmt.clone());
                    (p, vs)
                })
                .collect(),
            Rewritten::Exclusive(last_unfoldings) => {
                let mut unfolded = vec![];

                // unfold the last stmt
                let unchanged_prefix = &grab_fixed(&stmts[..stmts.len() - 1]);
                for (last_path, last_stmt) in last_unfoldings {
                    let mut merged: Vec<Stmt> = unchanged_prefix.iter().cloned().cloned().collect(); // XXX: cloned^2 ???
                    merged.push(last_stmt.clone());
                    unfolded.push((last_path.clone(), merged));
                }

                // skip the last statement
                unfolded.extend(unfold_stmts(&stmts[..stmts.len() - 1]));

                unfolded
            }
            Rewritten::ExclusiveOrFix(last_exc, last_fix) => {
                let mut unfolded = vec![];

                // unfold the last stmt
                let fixed_prefix = &grab_fixed(&stmts[..stmts.len() - 1]);
                for (last_path, last_stmt) in last_exc {
                    let mut merged: Vec<Stmt> = fixed_prefix.iter().cloned().cloned().collect(); // XXX: cloned^2 ???
                    merged.push(last_stmt.clone());
                    unfolded.push((last_path.clone(), merged));
                }

                // unfold the fixed variant of the last stmt
                let unfolded_prefix = unfold_stmts(&stmts[..stmts.len() - 1]);
                for (p, mut stmts) in unfolded_prefix {
                    stmts.push(last_fix.clone());
                    unfolded.push((p, stmts));
                }

                unfolded
            }
        }
    }
}

fn grab_fixed(stmts: &[Rewritten<Stmt>]) -> Vec<&Stmt> {
    stmts
        .iter()
        .filter_map(|rewritten| {
            if let Rewritten::Fix(stmt) = rewritten {
                Some(stmt)
            } else {
                None
            }
        })
        .collect()
}

/// parses macros of the form:
///     section!(identifier, "description", { ... });
fn parse_section_macro(stmt_macro: &syn::StmtMacro) -> Option<(String, String, Block)> {
    if !stmt_macro.mac.path.is_ident("section") {
        return None;
    }

    let tokens = stmt_macro.mac.tokens.clone();

    struct SectionArgs {
        ident: syn::Ident,
        description: LitStr,
        block: Block,
    }

    impl Parse for SectionArgs {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            // parse everything in parentheses: identifier, "description", { ... }
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![,]>()?;
            let description: LitStr = input.parse()?;
            input.parse::<syn::Token![,]>()?;
            let block: Block = input.parse()?;

            Ok(SectionArgs {
                ident,
                description,
                block: block,
            })
        }
    }

    match syn::parse2::<SectionArgs>(tokens) {
        Ok(args) => Some((args.ident.to_string(), args.description.value(), args.block)),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, parse_quote};

    use std::sync::LazyLock;

    struct TestCase {
        input: ItemFn,
        expected: TokenStream2,
    }

    fn get_test_cases() -> Vec<TestCase> {
        vec![
            TestCase {
                input: parse_quote! {
                    #[test]
                    fn root() {
                        let x = 1;
                        section!(a, "section a", {
                            println!(x);
                        });
                        let y = 2;
                        section!(b, "section b", {
                            println!(y);
                        });
                        let z = 2;
                    }
                },
                expected: quote! {
                    #[test]
                    fn root_b() {
                        let x = 1;
                        let y = 2;
                        {
                            println!("section b");
                            println!(y);
                        };
                        let z = 2;
                    }
                    #[test]
                    fn root_a() {
                        let x = 1;
                        {
                            println!("section a");
                            println!(x);
                        };
                        let y = 2;
                        let z = 2;
                    }
                    #[test]
                    fn root_() {
                        let x = 1;
                        let y = 2;
                        let z = 2;
                    }
                },
            },
            TestCase {
                input: parse_quote! {
                    #[test]
                    fn root() {
                        let x = 1;
                        section!(a, "section a", {
                            println!(x);
                        });
                        let y = || {
                            section!(b, "section b", {
                                println!(y);
                            });
                        };
                        let z = 2;
                    }
                },
                expected: quote! {
                    #[test]
                    fn root_b() {
                        let x = 1;
                        let y = 2;
                        {
                            println!("section b");
                            println!(y);
                        };
                        let z = 2;
                    }
                    #[test]
                    fn root_a() {
                        let x = 1;
                        {
                            println!("section a");
                            println!(x);
                        };
                        let y = 2;
                        let z = 2;
                    }
                    #[test]
                    fn root_() {
                        let x = 1;
                        let y = 2;
                        let z = 2;
                    }
                },
            },
        ]
    }

    #[test]
    fn test_impl() {
        for TestCase { input, expected } in get_test_cases() {
            let attr_args = TokenStream2::new();
            let result = shfix_impl(attr_args, quote! { #input });

            let result_file: syn::File = syn::parse2(quote! { #result }).unwrap();
            let expected_file: syn::File = syn::parse2(quote! { #expected }).unwrap();

            let result_pretty = prettyplease::unparse(&result_file);
            let expected_pretty = prettyplease::unparse(&expected_file);

            println!("Result:\n{}", result_pretty);
            println!("Expected:\n{}", expected_pretty);

            assert_eq!(result.to_string(), expected.to_string());
        }
    }
}
