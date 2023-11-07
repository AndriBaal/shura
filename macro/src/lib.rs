use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::sync::Mutex;
use std::{collections::HashSet, sync::OnceLock};
use syn::{
    parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Expr, Fields, LitStr, Type,
    TypePath,
};

const IDENT_NAME: &'static str = "shura";

fn field_attribute(data_struct: &DataStruct, attr_name: &str) -> Option<(Ident, TypePath)> {
    let mut result = None;
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident(attr_name) {
                                match &field.ty {
                                    Type::Path(type_name) => {
                                        let field_name = field.ident.as_ref().unwrap();
                                        assert!(
                                            result.is_none(),
                                            "{attr_name} is already defined!"
                                        );
                                        result = Some((field_name.clone(), type_name.clone()));
                                    }
                                    _ => panic!("Cannot extract the type of the component."),
                                };
                            }
                            Ok(())
                        })
                        .unwrap();
                    }
                }
            }
        }
        _ => (),
    };
    return result;
}

fn struct_attribute_name_value(ast: &DeriveInput, attr_name: &str) -> Option<Expr> {
    let mut result: Option<Expr> = None;
    for attr in &ast.attrs {
        if attr.path().is_ident(IDENT_NAME) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(attr_name) {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    if s.value() == attr_name {
                        assert!(result.is_none(), "{attr_name} is already defined!");
                        result = Some(syn::parse_str::<Expr>(&s.value()).unwrap())
                    }
                }
                Ok(())
            })
            .unwrap();
        }
    }
    return result;
}

static USED_COMPONENT_HASHES: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };

    let struct_name = ast.ident.clone();
    let struct_name_str = struct_name.to_string();
    let struct_identifier =
        struct_attribute_name_value(&ast, "name").unwrap_or(parse_quote!(#struct_name_str));
    let struct_identifier_str = struct_identifier.to_token_stream().to_string();
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

    let identifier = quote!(
        impl #impl_generics ::shura::ComponentIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::ComponentTypeId = ::shura::ComponentTypeId::new(#hash);

            fn component_type_id(&self) -> ::shura::ComponentTypeId {
                Self::IDENTIFIER
            }
        }
    );
    if let Some((instance_field_name, instance_type)) = field_attribute(data_struct, "instance") {
        return quote!(
            #identifier

            impl #impl_generics ::shura::Component for #struct_name #ty_generics #where_clause {
                type InstanceHandler = #instance_type;

                fn handler(&self) -> &Self::InstanceHandler {
                    &self.#instance_field_name
                }

                fn init(&mut self, handle: ::shura::ComponentHandle, world: &mut ::shura::World) {
                    self.#instance_field_name.init(handle, world)
                }

                fn finish(&mut self, world: &mut ::shura::World) {
                    self.#instance_field_name.finish(world)
                }
            }
        )
        .into();
    } else {
        return quote!(
            #identifier

            impl #impl_generics ::shura::Component for #struct_name #ty_generics #where_clause {
                type InstanceHandler = ::shura::EmptyInstance;

                fn handler(&self) -> &Self::InstanceHandler {
                    &::shura::EMPTY_INSTANCE
                }

                fn init(&mut self, handle: ::shura::ComponentHandle, world: &mut ::shura::World) {}
                fn finish(&mut self, world: &mut ::shura::World) {}
            }
        )
        .into();
    }
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
