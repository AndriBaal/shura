use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens, format_ident};
use std::sync::Mutex;
use std::{collections::HashSet, sync::OnceLock};
use syn::{parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Fields, Type, TypePath};

fn position_field(data_struct: &DataStruct, attr_name: &str) -> Option<Ident> {
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    let name = attr.path();
                    if name.is_ident(attr_name) {
                        match &field.ty {
                            Type::Path(_type_name) => {
                                let field_name = field.ident.as_ref().unwrap();
                                return Some(field_name.clone());
                            }
                            _ => panic!("Cannot extract the type of the component."),
                        };
                    }
                }
            }
        }
        _ => (),
    }
    None
}

fn name_value_field(ast: &DeriveInput, attr_name: &str) -> Option<syn::Expr> {
    for attr in &ast.attrs {
        let name = match &attr.meta {
            syn::Meta::NameValue(p) => p,
            _ => continue,
        };

        if name.path.is_ident(attr_name) {
            return Some(name.value.clone());
        }
    }
    return None;
}

fn is_path_set(ast: &DeriveInput, attr_name: &str) -> bool {
    for attr in &ast.attrs {
        let name = match &attr.meta {
            syn::Meta::Path(p) => p,
            _ => continue,
        };

        if name.is_ident(attr_name) {
            return true;
        }
    }
    return false;
}

fn fields_with_tag(data_struct: &DataStruct, attr_name: &str) -> (Vec<Ident>, Vec<TypePath>) {
    let mut fields = vec![];
    let mut tys = vec![];
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    let name = attr.path();
                    if name.is_ident(attr_name) {
                        match &field.ty {
                            Type::Path(type_name) => {
                                let field_name = field.ident.as_ref().unwrap();
                                fields.push(field_name.clone());
                                tys.push(type_name.clone());
                            }
                            _ => panic!(),
                        };
                    }
                }
            }
        }
        _ => (),
    }
    return (fields, tys);
}

static USED_COMPONENT_HASHES: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

#[proc_macro_derive(Component, attributes(position, parallel_buffer))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    return component(&ast).into();
}

fn component(ast: &DeriveInput) -> TokenStream2 {
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };

    let struct_name = ast.ident.clone();
    let struct_name_str = struct_name.to_string();
    let struct_identifier =
        name_value_field(&ast, "name").unwrap_or(parse_quote!(#struct_name_str));
    let struct_identifier_str = struct_identifier.to_token_stream().to_string();
    let (has_position, position_field_name) = position_field(data_struct, "position")
        .map(|f| (true, quote!(#f)))
        .unwrap_or((false, quote!(EMPTY_DEFAULT_COMPONENT)));
    let var_position_field_name = if has_position {
        quote!(self.#position_field_name)
    } else {
        quote!(::shura::#position_field_name)
    };

    let mut hashes = USED_COMPONENT_HASHES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    let mut hash = const_fnv1a_hash::fnv1a_hash_str_32(&struct_identifier_str);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    while hashes.contains(&hash) {
        hash += 1;
    }
    hashes.insert(hash);
    drop(hashes); // Free the mutex lock

    let (buffer_fields, buffer_types) = fields_with_tag(data_struct, "buffer");
    let mut struct_fields = Vec::with_capacity(buffer_fields.len());
    for (field, ty) in buffer_fields.iter().zip(buffer_types.iter()) {
        struct_fields.push(quote!(#field: #ty));
    }

    let parallel_buffer = is_path_set(&ast, "parallel_buffer");
    let buffer_method = if cfg!(feature = "rayon") && parallel_buffer {
        "par_buffer"
    } else {
        "buffer"
    };
    let buffer_method_with = format_ident!("{}_with", buffer_method);
    let buffer_method = format_ident!("{}", buffer_method);

    let init_finish = if has_position {
        quote!(
            fn init(&mut self, handle: ::shura::ComponentHandle, world: &mut ::shura::World) {
                #var_position_field_name.init(handle, world)
            }

            fn finish(&mut self, world: &mut ::shura::World) {
                #var_position_field_name.finish(world)
            }
        )
    } else {
        quote!(
            fn init(&mut self, handle: ::shura::ComponentHandle, world: &mut ::shura::World) {}
            fn finish(&mut self, world: &mut ::shura::World) {}
        )
    };

    return quote!(
        impl #impl_generics ::shura::ComponentIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::ComponentTypeId = ::shura::ComponentTypeId::new(#hash);
        }

        impl #impl_generics ::shura::Component for #struct_name #ty_generics #where_clause {
            #init_finish

            fn position(&self) -> &dyn ::shura::Position {
                &#var_position_field_name
            }

            fn buffer_with(
                mut helper: BufferHelper<Self>,
                each: impl Fn(&mut Self) + Send + Sync,
            ) {
                helper.#buffer_method_with(each)
            }
            fn buffer(mut helper: BufferHelper<Self>) {
                helper.#buffer_method()
            }

            fn component_type_id(&self) -> ::shura::ComponentTypeId {
                Self::IDENTIFIER
            }
        }
    );
}

#[proc_macro_attribute]
/// This macro helps setup a cross plattform main method
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item: TokenStream2 = item.into();
    quote!(
        #item

        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(app: ::shura::AndroidApp) {
            shura_main(::shura::AppConfig::default(app));
        }

        #[cfg(not(target_os = "android"))]
        #[allow(dead_code)]
        fn main() {
            shura_main(::shura::AppConfig::default());
        }
    )
    .into()
}
