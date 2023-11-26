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
const ALLOWED_ATTRIBUTES: &'static [&'static str] = &["component"];

fn components(data_struct: &DataStruct, attr_name: &str) -> Vec<(Ident, Option<LitStr>, TypePath)> {
    let mut components = Vec::new();
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            ALLOWED_ATTRIBUTES
                                .iter()
                                .find(|a| meta.path.is_ident(a))
                                .expect("Unexpected attribute!");
                            if meta.path.is_ident(attr_name) {
                                match &field.ty {
                                    Type::Path(type_name) => {
                                        let field_name = field
                                            .ident
                                            .as_ref()
                                            .expect("Entity fields must be named!");
                                        let component_name = if let Ok(value) = meta.value() {
                                            value.parse().ok()
                                        } else {
                                            None
                                        };
                                        components.push((
                                            field_name.clone(),
                                            component_name,
                                            type_name.clone(),
                                        ));
                                    }
                                    _ => panic!("Cannot extract the type of the entity."),
                                };
                            }
                            Ok(())
                        })
                        .expect(
                            "Define your component like the this: #[shura(component = \"name\")]",
                        );
                    }
                }
            }
        }
        _ => (),
    };
    return components;
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

#[proc_macro_derive(Entity, attributes(shura))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
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

    let components = components(&data_struct, "component");
    let names_init = components.iter().map(|&(ref first, ..)| first);
    let names_components = names_init.clone();
    let names_finish = names_init.clone();

    let buffer = components
        .iter()
        .map(|(field_name, component_name, component_type)| {
            if let Some(component_name) = component_name {
                quote! {
                    let buffer = buffers.get_mut::<<#component_type as shura::Component> ::Instance>(#component_name).unwrap();
                    buffer.par_push_from_entities(world, &entities, |e| &e.#field_name);
                }
            } else {
                quote!()
            }
        });

    return quote!(
        impl #impl_generics ::shura::EntityIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::EntityTypeId = ::shura::EntityTypeId::new(#hash);

            fn entity_type_id(&self) -> ::shura::EntityTypeId {
                Self::IDENTIFIER
            }
        }

        impl #impl_generics ::shura::Entity for #struct_name #ty_generics #where_clause {
            fn buffer<'a>(
                entities: ::shura::EntitySet<'a, Self>,
                buffers: &mut ::shura::ComponentBufferManager,
                world: &::shura::World,
            ) {
                #( #buffer )*
            }

            fn components(&self) -> Vec<&dyn std::any::Any> {
                vec![ #( &self.#names_components as &dyn std::any::Any, )* ]
            }

            fn init(&mut self, handle: ::shura::EntityHandle, world: &mut ::shura::World) {
                #( self.#names_init.init(handle, world); )*
            }

            fn finish(&mut self, world: &mut ::shura::World) {
                #( self.#names_finish.finish(world); )*
            }
        }
    )
    .into();
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
