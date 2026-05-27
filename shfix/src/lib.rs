use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parse;
use syn::{
    Block, Expr, ExprBlock, ItemFn, LitStr, Local, Stmt, StmtMacro, parse_quote,
};

#[proc_macro_attribute]
pub fn shfix(args: TokenStream, input: TokenStream) -> TokenStream {
    shfix_impl(args.into(), input.into()).into()
}

fn shfix_impl(_args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let input_fn: ItemFn = syn::parse2(input).expect("expected a function");

    let mut output = TokenStream2::new();
    let rw_block = rewrite_block(None, &input_fn.block);
    if rw_block.exclusive.is_empty() {
        output.extend(quote! { #input_fn });
    } else {
        for (path, block) in rw_block.exclusive {
            let mut unfolded_fn = input_fn.clone();
            let name = format!("{}_{}", unfolded_fn.sig.ident, path.join("_"));
            unfolded_fn.sig.ident =
                syn::Ident::new(&name, input_fn.sig.ident.span());
            *unfolded_fn.block = block;
            output.extend(quote! { #unfolded_fn });
        }
    }

    output
}

type SectionPath = Vec<String>;

struct Rewritten<Q> {
    exclusive: Vec<(SectionPath, Q)>,
    fixed: Q,
}

// rewrites each individual statement
fn rewrite_stmts(stmts: &[Stmt]) -> Vec<Rewritten<Stmt>> {
    let mut rewritten = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Item(_) | Stmt::Expr(_, _) => rewritten.push(Rewritten {
                exclusive: vec![],
                fixed: stmt.clone(),
            }),
            Stmt::Local(local) => {
                let rw_local = rewrite_local(local);
                let unfolded_stmts = rw_local
                    .exclusive
                    .into_iter()
                    .map(|(p, l)| (p, Stmt::Local(l)))
                    .collect();
                rewritten.push(Rewritten {
                    exclusive: unfolded_stmts,
                    fixed: Stmt::Local(rw_local.fixed),
                });
            }
            Stmt::Macro(stmt_macro) => {
                if let Some((id, description, section_block)) =
                    parse_section_macro(stmt_macro)
                {
                    let rw_section_block =
                        rewrite_block(Some(&description), &section_block);
                    if rw_section_block.exclusive.is_empty() {
                        rewritten.push(Rewritten {
                            exclusive: vec![(
                                vec![id.clone()],
                                lift_block_to_stmt(
                                    stmt_macro,
                                    rw_section_block.fixed,
                                ),
                            )],
                            fixed: lift_block_to_stmt(
                                stmt_macro,
                                Block {
                                    // empty block
                                    brace_token: section_block.brace_token,
                                    stmts: vec![],
                                },
                            ),
                        });
                    } else {
                        rewritten.push(Rewritten {
                            exclusive: lift_blocks_to_stmts(
                                &id,
                                stmt_macro,
                                rw_section_block,
                            ),
                            fixed: lift_block_to_stmt(
                                stmt_macro,
                                Block {
                                    // empty block
                                    brace_token: section_block.brace_token,
                                    stmts: vec![],
                                },
                            ),
                        });
                    }
                } else {
                    rewritten.push(Rewritten {
                        exclusive: vec![],
                        fixed: stmt.clone(),
                    })
                }
            }
        }
    }

    rewritten
}

fn lift_blocks_to_stmts(
    id: &str,
    stmt_macro: &StmtMacro,
    rewritten_block: Rewritten<Block>,
) -> Vec<(Vec<String>, Stmt)> {
    let mut rw_stmts = vec![];
    for (rw_path, rw_block) in rewritten_block.exclusive {
        let mut path = vec![id.to_owned()];
        path.extend(rw_path);
        rw_stmts.push((path, lift_block_to_stmt(stmt_macro, rw_block)));
    }
    rw_stmts
}

fn lift_block_to_stmt(stmt_macro: &StmtMacro, block: Block) -> Stmt {
    Stmt::Expr(
        Expr::Block(ExprBlock {
            attrs: stmt_macro.attrs.clone(),
            label: None,
            block,
        }),
        stmt_macro.semi_token,
    )
}

fn rewrite_local(local: &Local) -> Rewritten<Local> {
    if let Some((expr_block, block)) = parse_local_closure(local) {
        let rw_block = rewrite_block(None, block);

        // exclusive
        let exclusive = rw_block
            .exclusive
            .into_iter()
            .map(|(p, b)| {
                let mut new_local = local.clone();
                let new_expr_closure = if let Expr::Closure(x) =
                    &mut *new_local.init.as_mut().unwrap().expr
                {
                    x
                } else {
                    panic!("bug")
                };
                let expr_block = Expr::Block(ExprBlock {
                    attrs: expr_block.attrs.clone(),
                    label: expr_block.label.clone(),
                    block: b,
                });
                *new_expr_closure.body = expr_block;
                (p, new_local)
            })
            .collect();

        // fixed
        let mut new_fixed = local.clone();
        let new_expr_closure = if let Expr::Closure(x) =
            &mut *new_fixed.init.as_mut().unwrap().expr
        {
            x
        } else {
            panic!("bug")
        };
        let expr_block = Expr::Block(ExprBlock {
            attrs: expr_block.attrs.clone(),
            label: expr_block.label.clone(),
            block: rw_block.fixed,
        });
        *new_expr_closure.body = expr_block;

        Rewritten {
            exclusive,
            fixed: new_fixed,
        }
    } else {
        Rewritten {
            exclusive: vec![],
            fixed: local.clone(),
        }
    }
}

fn rewrite_block(description: Option<&str>, block: &Block) -> Rewritten<Block> {
    let rewritten_stmts = rewrite_stmts(&block.stmts);

    // println!(
    //     "rewrite_block/rw_stmts: {}",
    //     _rewritten_stmts_to_str(&rewritten_stmts)
    // );

    let (unfolded_stmts, fixed_stmts) = unfold_stmts(&rewritten_stmts);

    // println!(
    //     "rewrite_block/unfolded_stmts: {} /fixed_stmts: {}",
    //     _unfolded_stmts_to_str(&unfolded_stmts),
    //     _stmts_to_str(&fixed_stmts),
    // );

    let header_stmt = description.map(|d| {
        let s: Stmt = parse_quote! {
            println!(#d);
        };
        s
    });

    // fixed
    let mut new_fixed_stmts = if let Some(h) = header_stmt.clone() {
        vec![h]
    } else {
        vec![]
    };
    new_fixed_stmts.extend(fixed_stmts);
    let block_fixed = Block {
        brace_token: block.brace_token,
        stmts: new_fixed_stmts,
    };

    // exclusive
    let mut block_unfoldings = Vec::new();
    for (path, stmts) in unfolded_stmts {
        let mut new_stmts = if let Some(h) = &header_stmt {
            vec![h.clone()]
        } else {
            vec![]
        };
        new_stmts.extend(stmts);
        block_unfoldings.push((
            path.clone(),
            Block {
                brace_token: block.brace_token,
                stmts: new_stmts,
            },
        ));
    }

    Rewritten {
        exclusive: block_unfoldings,
        fixed: block_fixed,
    }
}

// unfoldings with one exclusive activation, unfolding with no exclusive activations
fn unfold_stmts(
    stmts: &[Rewritten<Stmt>],
) -> (Vec<(SectionPath, Vec<Stmt>)>, Vec<Stmt>) {
    if stmts.is_empty() {
        (vec![], vec![])
    } else {
        let last = stmts.last().unwrap();

        let fixed_prefix = grab_fixed(stmts);

        // unfoldings with last stmt activated
        let mut unfolded = vec![];
        for (last_path, last_stmt) in &last.exclusive {
            let mut merged: Vec<Stmt> = fixed_prefix[..fixed_prefix.len() - 1]
                .iter()
                .cloned()
                .cloned()
                .collect(); // XXX: cloned^2 ???
            merged.push(last_stmt.clone());
            unfolded.push((last_path.clone(), merged));
        }

        // unfoldings with activations in the prefix
        let (prefix_unfoldings, _) = unfold_stmts(&stmts[..stmts.len() - 1]);
        for mut unfolding in prefix_unfoldings {
            unfolding.1.push(last.fixed.clone());
            unfolded.push(unfolding);
        }

        (unfolded, fixed_prefix.iter().cloned().cloned().collect())
    }
}

fn grab_fixed(stmts: &[Rewritten<Stmt>]) -> Vec<&Stmt> {
    stmts.iter().map(|rewritten| &rewritten.fixed).collect()
}

/// parses macros of the form:
///     section!(identifier, "description", { ... });
fn parse_section_macro(
    stmt_macro: &syn::StmtMacro,
) -> Option<(String, String, Block)> {
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
                block,
            })
        }
    }

    match syn::parse2::<SectionArgs>(tokens) {
        Ok(args) => {
            Some((args.ident.to_string(), args.description.value(), args.block))
        }
        Err(_) => None,
    }
}

fn parse_local_closure(local: &Local) -> Option<(&ExprBlock, &Block)> {
    if let Some(local_init) = &local.init
        && let Expr::Closure(expr_closure) = &*local_init.expr
        && let Expr::Block(expr_block) = &*expr_closure.body
    {
        return Some((expr_block, &expr_block.block));
    }
    None
}

fn _rewritten_stmts_to_str(stmts: &[Rewritten<Stmt>]) -> String {
    let mut r = String::new();
    for rw in stmts {
        let fixed = &rw.fixed;
        r.push_str(&format!("fixed({}) ", quote! { #fixed }));
        for (path, stmt) in &rw.exclusive {
            r.push_str("exclusive(");
            r.push_str(&format!("{}: {}", path.join("_"), quote! { #stmt }));
            r.push_str(") ");
        }
        r.push_str(" ^ ");
    }
    r
}

fn _unfolded_stmts_to_str(u: &[(SectionPath, Vec<Stmt>)]) -> String {
    let mut r = String::new();
    for (path, ss) in u {
        r.push_str(&format!("{}({:?}) ", path.join("_"), _stmts_to_str(ss),));
    }
    r
}

fn _stmts_to_str(stmts: &[Stmt]) -> String {
    stmts
        .iter()
        .map(|s| format!("{}", quote! { #s }))
        .collect::<Vec<String>>()
        .join(" • ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, parse_quote};

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
                        {};
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
                        {};
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
                            section!(b, "section b", {
                                println!(y);
                            });
                            section!(c, "section c", {
                                println!(y);
                            });
                        });
                        let y = 2;
                    }
                },
                expected: quote! {
                    #[test]
                    fn root_a_c() {
                        let x = 1;
                        {
                            println!("section a");
                            println!(x);
                            {};
                            {
                                println!("section c");
                                println!(y);
                            };
                        };
                        let y = 2;
                    }
                    #[test]
                    fn root_a_b() {
                        let x = 1;
                        {
                            println!("section a");
                            println!(x);
                            {
                                println!("section b");
                                println!(y);
                            };
                            {};
                        };
                        let y = 2;
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
                        let y = move |x| {
                            let u = 0;
                            section!(b, "section b", {
                                println!(y);
                            });
                            let w = 0;
                        };
                        let z = 2;
                    }
                },
                expected: quote! {
                    #[test]
                    fn root_b() {
                        let x = 1;
                        {};
                        let y = move |x| {
                            let u = 0;
                            {
                                println!("section b");
                                println!(y);
                            };
                            let w = 0;
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
                        let y = move |x| {
                            let u = 0;
                            {};
                            let w = 0;
                        };
                        let z = 2;
                    }
                },
            },
        ]
    }

    #[test]
    fn test_impl() {
        for (i, TestCase { input, expected }) in
            get_test_cases().iter().enumerate()
        {
            let attr_args = TokenStream2::new();
            let result = shfix_impl(attr_args, quote! { #input });

            println!("test #{}", i);

            let result_file: syn::File =
                syn::parse2(quote! { #result }).unwrap();
            let expected_file: syn::File =
                syn::parse2(quote! { #expected }).unwrap();

            let result_pretty = prettyplease::unparse(&result_file);
            let expected_pretty = prettyplease::unparse(&expected_file);

            println!("result:\n{}", result_pretty);
            println!("expected:\n{}", expected_pretty);

            assert_eq!(result.to_string(), expected.to_string(), "test {}", i);
        }
    }
}
