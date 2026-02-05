use proc_macro::TokenStream;

pub fn max_hardcoded(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();

    quote::quote! {

        #[derive(Clone)]
        pub struct SdkGrpcService {
            pub app: std::sync::Arc<#input>,
        }

        impl SdkGrpcService {
            pub fn new(app: std::sync::Arc<#input>) -> Self {
                Self { app }
            }
        }

    }
    .into()
}
