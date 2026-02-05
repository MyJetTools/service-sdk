use proc_macro::TokenStream;
use syn::parse_macro_input;

use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, Path, Result, Token, Type,
};

struct GenerateGrpcServiceArgs {
    service_ident: Ident,
    app_ty: Type,
    server_path: Path,
}

impl Parse for GenerateGrpcServiceArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let service_ident: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let app_ty: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let server_path: Path = input.parse()?;

        Ok(Self {
            service_ident,
            app_ty,
            server_path,
        })
    }
}

pub fn with_params(input: TokenStream) -> TokenStream {
    let GenerateGrpcServiceArgs {
        service_ident,
        app_ty,
        server_path,
    } = parse_macro_input!(input as GenerateGrpcServiceArgs);

    let expanded = quote! {
        #[derive(Clone)]
        pub struct #service_ident {
            pub app_context: ::std::sync::Arc<#app_ty>,
        }

        impl #service_ident {
            pub fn new(app_context: ::std::sync::Arc<#app_ty>) -> Self {
                Self { app_context }
            }
        }

        impl service_sdk::IntoGrpcServer for #service_ident {
            type GrpcServer = #server_path<Self>;

            fn into_grpc_server(self) -> Self::GrpcServer {
                #server_path::new(self)
            }
        }
    };

    expanded.into()
}
