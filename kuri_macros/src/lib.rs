use proc_macro::TokenStream;

fn is_injected_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(ty) => {
            let path = &ty.path;
            if let Some(segment) = path.segments.last() {
                segment.ident == "Inject"
            } else {
                false
            }
        }
        _ => false,
    }
}

mod prompt;
mod tool;

#[proc_macro_attribute]
pub fn prompt(args: TokenStream, input: TokenStream) -> TokenStream {
    prompt::prompt(args, input)
}

#[proc_macro_attribute]
pub fn tool(args: TokenStream, input: TokenStream) -> TokenStream {
    tool::tool(args, input)
}
