use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parse;
use syn::{
    Block, Expr, ExprBlock, ItemFn, LitStr, Macro, Stmt, StmtMacro, parse_quote, token::Brace,
    token::Semi,
};

#[proc_macro_attribute]
pub fn shfix(args: TokenStream, input: TokenStream) -> TokenStream {
    shfix_impl(args.into(), input.into()).into()
}

fn shfix_impl(_args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let input_fn: ItemFn = syn::parse2(input).expect("expected a function");

    let mut output = TokenStream2::new();

    match rewrite_block(&input_fn.block) {
        Rewritten::Unchanged(_) => {
            output.extend(quote! { #input_fn });
        }
        Rewritten::Unfolded(unfoldings) => {
            for (path, block) in unfoldings {
                let mut unfolded_fn = input_fn.clone();
                let name = format!("{}_{}", unfolded_fn.sig.ident, path.join("_"));
                unfolded_fn.sig.ident = syn::Ident::new(&name, input_fn.sig.ident.span());
                unfolded_fn.block = block;
                output.extend(quote! { #unfolded_fn });
            }
        }
    }

    output
}

type SectionPath = Vec<String>;

enum Rewritten<Q> {
    Unchanged(Q),
    Unfolded(Vec<(SectionPath, Q)>),
}

// rewrites each individual statement
fn rewrite_stmts(stmts: &Vec<Stmt>) -> Vec<Rewritten<Stmt>> {
    let mut rewritten = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::Local(_) | Stmt::Item(_) | Stmt::Expr(_, _) => {
                rewritten.push(Rewritten::Unchanged(stmt.clone()))
            }
            Stmt::Macro(stmt_macro) => {
                if let Some((id, description, section_block)) = parse_section_macro(stmt_macro) {
                    match rewrite_block2(&description, &section_block) {
                        Rewritten::Unchanged(_) => {
                            rewritten
                                .push(Rewritten::Unfolded(vec![(vec![id.clone()], stmt.clone())]));
                        }
                        Rewritten::Unfolded(rewritten_section_blocks) => {
                            let mut stmt_rewrites = vec![];
                            for (rewritten_path, rewritten_block) in rewritten_section_blocks {
                                let mut path = vec![id.clone()];
                                path.extend(rewritten_path);
                                stmt_rewrites.push((
                                    path,
                                    Stmt::Expr(
                                        Expr::Block(ExprBlock {
                                            attrs: stmt_macro.attrs.clone(),
                                            label: None,
                                            block: rewritten_block,
                                        }),
                                        stmt_macro.semi_token.clone(),
                                    ),
                                ));
                            }
                            rewritten.push(Rewritten::Unfolded(stmt_rewrites));
                        }
                    }
                } else {
                    rewritten.push(Rewritten::Unchanged(stmt.clone()))
                }
            }
        }
    }

    rewritten
}

fn rewrite_block2(description: &String, block: &Block) -> Rewritten<Block> {
    let rewritten_stmts = rewrite_stmts(&block.stmts);
    let unfolded_stmts = unfold_stmts(&rewritten_stmts);
    let header_stmt = parse_quote! {
        println!(#description);
    };

    let mut unfoldings = Vec::new();
    if unfolded_stmts.is_empty() {
        unfoldings.push((
            vec![],
            Block {
                brace_token: block.brace_token.clone(),
                stmts: vec![header_stmt],
            },
        ));
    } else {
        for (path, stmts) in unfold_stmts(&rewritten_stmts) {
            let mut new_stmts = vec![header_stmt.clone()];
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
    Rewritten::Unfolded(unfoldings)
}

fn unfold_stmts(stmts: &[Rewritten<Stmt>]) -> Vec<(SectionPath, Vec<Stmt>)> {
    if stmts.is_empty() {
        vec![]
    } else {
        match &stmts.last().unwrap() {
            Rewritten::Unchanged(x) => unfold_stmts(&stmts[..stmts.len() - 1])
                .into_iter()
                .map(|(p, mut vs)| {
                    vs.push(x.clone());
                    (p, vs)
                })
                .collect(),
            Rewritten::Unfolded(last_unfoldings) => {
                let mut unfolded = vec![];

                let unchanged_prefix = &grab_unchanged(&stmts[..stmts.len() - 1]);

                for (last_path, last_stmt) in last_unfoldings {
                    let mut merged: Vec<Stmt> = unchanged_prefix.iter().cloned().cloned().collect(); // XXX: cloned^2 ???
                    merged.push(last_stmt.clone());
                    unfolded.push((last_path.clone(), merged));
                }

                unfolded
            }
        }
    }
}

fn grab_unchanged(stmts: &[Rewritten<Stmt>]) -> Vec<&Stmt> {
    stmts
        .iter()
        .filter_map(|rewritten| {
            if let Rewritten::Unchanged(stmt) = rewritten {
                Some(stmt)
            } else {
                None
            }
        })
        .collect()
}

fn rewrite_block(block: &Box<Block>) -> Rewritten<Box<Block>> {
    let mut unfoldings = Vec::new();

    let mut prefix_stmts = Vec::new();
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(local) => prefix_stmts.push(syn::Stmt::Local(local.clone())),
            syn::Stmt::Item(item) => prefix_stmts.push(syn::Stmt::Item(item.clone())),
            syn::Stmt::Expr(expr, opt_token) => {
                prefix_stmts.push(syn::Stmt::Expr(expr.clone(), opt_token.clone()))
            }
            syn::Stmt::Macro(stmt_macro) => {
                if let Some((id, description, section_block)) = parse_section_macro(stmt_macro) {
                    // recursively, compute all unfoldings of the macro block
                    match rewrite_block(&Box::new(section_block)) {
                        Rewritten::Unfolded(section_unfoldings) =>
                        // for each unfolded block `{ macro_stmts };` create `{ prefix_stmts; { macro_stmts }; }`
                        {
                            for (leaf_path, unfolded_section_block) in section_unfoldings {
                                let stitched_block =
                                    stitch(&description, &prefix_stmts, unfolded_section_block);
                                let mut path = vec![id.clone()];
                                path.extend(leaf_path);
                                unfoldings.push((path, stitched_block));
                            }
                        }
                        Rewritten::Unchanged(section_block) => {
                            let stitched_block = stitch(&description, &prefix_stmts, section_block);
                            unfoldings.push((vec![id.clone()], stitched_block));
                        }
                    }
                } else {
                    prefix_stmts.push(syn::Stmt::Macro(stmt_macro.clone()))
                }
            }
        }
    }

    if unfoldings.is_empty() {
        // leaf block
        Rewritten::Unchanged(block.clone())
    } else {
        Rewritten::Unfolded(unfoldings)
    }
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

fn stitch(description: &String, stmts: &Vec<Stmt>, last: Box<Block>) -> Box<Block> {
    let mut new_stmts = Vec::new();

    new_stmts.push(parse_quote! {
        println!(#description);
    });

    new_stmts.extend(stmts.clone());

    let block_expr = Expr::Block(ExprBlock {
        attrs: Vec::new(),
        label: None,
        block: *last,
    });

    // add the block as a statement with semicolon
    new_stmts.push(Stmt::Expr(block_expr, Some(Semi::default())));

    Box::new(Block {
        brace_token: Brace::default(),
        stmts: new_stmts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, parse_quote};

    #[test]
    fn test_impl() {
        let attr_args = TokenStream2::new();
        let input_fn: ItemFn = parse_quote! {
            #[test]
            fn root() {
                let x = 1;
                section!(a, "section a", {
                    println!(x);
                });
                let y = 2;
                section!(b, "section b", { println!(y); });
                let z = 2;
            }
        };
        let result = shfix_impl(attr_args, quote! { #input_fn });
        let expected = quote! {
            fn renamed() {
            }
        };

        let result_file: syn::File = syn::parse2(quote! { #result }).unwrap();
        let expected_file: syn::File = syn::parse2(quote! { #expected }).unwrap();

        let result_pretty = prettyplease::unparse(&result_file);
        let expected_pretty = prettyplease::unparse(&expected_file);

        println!("Result:\n{}", result_pretty);
        println!("Expected:\n{}", expected_pretty);

        // assert_eq!(result.to_string(), expected.to_string());
    }
}
