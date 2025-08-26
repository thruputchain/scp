use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parse;
use syn::{Block, ItemFn, LitStr, parse_quote};

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

enum Rewritten<T> {
    Unchanged(T),
    Unfolded(Vec<(SectionPath, T)>),
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

fn stitch(description: &String, stmts: &Vec<syn::Stmt>, last: Box<Block>) -> Box<Block> {
    let mut new_stmts = Vec::new();

    let println_stmt: syn::Stmt = parse_quote! {
        println!(#description);
    };
    new_stmts.push(println_stmt);

    new_stmts.extend(stmts.clone());

    let block_expr = syn::Expr::Block(syn::ExprBlock {
        attrs: Vec::new(),
        label: None,
        block: *last,
    });

    // add the block as a statement with semicolon
    new_stmts.push(syn::Stmt::Expr(
        block_expr,
        Some(syn::token::Semi::default()),
    ));

    Box::new(Block {
        brace_token: syn::token::Brace::default(),
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
