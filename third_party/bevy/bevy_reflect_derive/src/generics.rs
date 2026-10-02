use crate::derive_data::ReflectMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{GenericParam, Token};

/// Creates a `TokenStream` for generating an expression that creates a `Generics` instance.
///
/// Returns `None` if `Generics` cannot or should not be generated.
pub(crate) fn generate_generics(meta: &ReflectMeta) -> Option<TokenStream> {
    let bevy_reflect_path = meta.bevy_reflect_path();

    let generics = meta
        .type_path()
        .generics()
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(ty_param) => {
                let ident = &ty_param.ident;
                let name = ident.to_string();
                Some(quote! {
                    #bevy_reflect_path::GenericInfo::Type(
                        #bevy_reflect_path::TypeParamInfo::new::<#ident>(
                            #bevy_reflect_path::__macro_exports::alloc_utils::Cow::Borrowed(#name),
                        )
                    )
                })
            }
            GenericParam::Const(const_param) => {
                let ty = &const_param.ty;
                let name = const_param.ident.to_string();
                Some(quote! {
                    #bevy_reflect_path::GenericInfo::Const(
                        #bevy_reflect_path::ConstParamInfo::new::<#ty>(
                            #bevy_reflect_path::__macro_exports::alloc_utils::Cow::Borrowed(#name),
                        )
                    )
                })
            }
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Punctuated<_, Token![,]>>();

    if generics.is_empty() {
        // No generics to generate
        return None;
    }

    Some(quote!(#bevy_reflect_path::Generics::from_iter([ #generics ])))
}
