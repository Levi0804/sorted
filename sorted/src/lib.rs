use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, Item};
use quote::{quote, ToTokens};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    assert!(args.is_empty());

    if let Item::Enum(ref e) = item {
        if let Err(err) = ordered(e.clone()) {
            return err.to_compile_error().into();
        }
    } else {
        return syn::Error::new(proc_macro2::Span::call_site(), "expected enum or match expression")
            .to_compile_error()
            .into(); 
    }

    quote! { #item }.into()
}

fn ordered(item: syn::ItemEnum) -> syn::Result<()> {
    let mut names = Vec::new();
    for variant in &item.variants {
        let name = variant.ident.to_string();
        if names.last().map(|last| &name < last).unwrap_or(false)  {
            let next_lex_i = names.binary_search(&name).unwrap_err();
            return Err(syn::Error::new(variant.span(), format!("{} should sort before {}", name, names[next_lex_i])));
        }
        names.push(name);  
    }
    Ok(())
}

#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {

    let item_fn = parse_macro_input!(input as syn::ItemFn);
    assert!(args.is_empty());

    if let Some(v) = get_arms(&item_fn) {
        if let Err(err) = ordered_match(v) {
            let mut out = TokenStream::from(remove_attr(item_fn).to_token_stream());
            out.extend(TokenStream::from(err.to_compile_error()));
            return out;
        }
    } 

    let item_fn = remove_attr(item_fn);
    quote! { #item_fn }.into()
}

fn remove_attr(mut item_fn: syn::ItemFn) -> syn::ItemFn {
    for stmt in &mut item_fn.block.stmts {
        if let syn::Stmt::Expr(syn::Expr::Match(expr), ..) = stmt {
            if !expr.attrs.is_empty() {
                expr.attrs.retain(|attr| {
                    if let syn::Meta::Path(path) = &attr.meta {
                        let ident = &path.segments[0].ident;
                        ident != "sorted" 
                    } else {
                        true
                    }
                });
            }
        }
    }
    item_fn
}

fn get_arms(item_fn: &syn::ItemFn) -> Option<Vec<syn::Arm>> {
    for stmt in &item_fn.block.stmts {
        if let syn::Stmt::Expr(syn::Expr::Match(expr), ..) = stmt {
            if !expr.attrs.is_empty() {
                for attr in &expr.attrs {
                    if let syn::Meta::Path(path) = &attr.meta {
                        let ident = &path.segments[0].ident;
                        if ident == "sorted" {
                            return Some(expr.arms.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

fn ordered_match(arms: Vec<syn::Arm>) -> syn::Result<()> {
    let mut names = Vec::new();
    let mut ident: syn::Ident;
    for arm in &arms {
        if let syn::Pat::TupleStruct(syn::PatTupleStruct { path, .. }) = &arm.pat {
            ident = path.segments[0].ident.clone();
            let pairs = path.segments.pairs();
            for pair in &pairs.collect::<Vec<_>>() {
                if let syn::punctuated::Pair::Punctuated(_,_) = pair {
                    ident = path.segments[path.segments.len() - 1].ident.clone();
                    let error = &path.segments[0].ident.to_string();
                    if names.last().map(|last| &ident < last).unwrap_or(false)  {
                        let next_lex_i = names.binary_search(&ident).unwrap_err();
                        return Err(syn::Error::new_spanned(path, format!("{error}::{} should sort before {error}::{}", ident, names[next_lex_i])));
                    }
                    names.push(ident.clone());
                }
            }
        } else if let syn::Pat::Ident(pat) = &arm.pat {
            ident = pat.ident.clone();
        } else if let syn::Pat::Wild(syn::PatWild {underscore_token,..}) = &arm.pat {
            ident = syn::Ident::new("_", underscore_token.span());
        } else {
            return Err(syn::Error::new_spanned(arm.clone().pat, "unsupported by #[sorted]"));
        }
        if names.last().map(|last| &ident < last).unwrap_or(false)  {
            let next_lex_i = names.binary_search(&ident).unwrap_err();
            return Err(syn::Error::new_spanned(ident.clone(), format!("{} should sort before {}", ident, names[next_lex_i])));
        }
        names.push(ident);
}
    Ok(())
}