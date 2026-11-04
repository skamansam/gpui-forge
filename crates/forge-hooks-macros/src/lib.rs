//! `#[forge_hook]` — marks a function as a registered hook.
//!
//! The hook's standard signature is
//! `fn(&mut ForgeView, &HookCtx, &[String], &mut Context<ForgeView>)` where the
//! crate declares `type HookCtx = ...` once; `forge_hooks::dispatch::<Ctx>`
//! routes events to the generated registry entries.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn forge_hook(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let f = parse_macro_input!(item as ItemFn);
    let name = f.sig.ident.to_string();
    let ident = &f.sig.ident;
    let vis = &f.vis;
    quote! {
        #f
        #[allow(non_upper_case_globals)]
        #vis const _: () = {
            ::forge_hooks::inventory::submit! {
                ::forge_hooks::HookDef::new(
                    #name,
                    |view, ctx, args, cx| #ident(
                        view,
                        ctx.downcast_ref::<crate::HookCtx>().expect("hook ctx"),
                        args,
                        cx,
                    ),
                )
            }
        };
    }
    .into()
}
