use proc_macro::TokenStream;
use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, parse_str, Data, DataStruct, DeriveInput, Expr, LitStr, Type,
};

type AllComponents = HashMap<Ident, Type>;

const IDENT_NAME: &str = "shura";

fn component_data(data_struct: &DataStruct) -> AllComponents {
    let mut components = AllComponents::default();
    for field in data_struct.fields.iter() {
        for attr in &field.attrs {
            if attr.path().is_ident(IDENT_NAME) {
                attr.parse_nested_meta(|meta| {
                    let field_name = field.ident.as_ref().unwrap();
                    if meta.path.is_ident("component") {
                        components.insert(field_name.clone(), field.ty.clone());
                    }
                    Ok(())
                })
                .expect("Define your components like the this: '#[shura(component)]'");
            }
        }
    }
    components
}

fn identifier_name(ast: &DeriveInput) -> Option<Expr> {
    const ATTR_NAME: &str = "name";
    let mut result: Option<Expr> = None;
    for attr in &ast.attrs {
        if attr.path().is_ident(IDENT_NAME) {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(ATTR_NAME) {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    assert!(result.is_none(), "{ATTR_NAME} is already defined!");
                    if let Ok(value) = syn::parse_str::<Expr>(&s.value()) {
                        result = Some(value)
                    }
                }
                Ok(())
            });
        }
    }
    result
}

fn component(ast: &DeriveInput, trait_type: Type, trait_identifier_type: Type) -> TokenStream2 {
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };

    let struct_name = ast.ident.clone();
    let struct_name_str = struct_name.to_string();
    let struct_identifier = identifier_name(ast).unwrap_or(parse_quote!(#struct_name_str));
    let components = component_data(data_struct);

    let component = components
        .iter()
        .enumerate()
        .map(|(idx, (field_name, _field_type))| {
            let idx = idx as u32;
            quote! {
                #idx => Some( &self.#field_name as _)
            }
        });
    let component_mut = components
        .iter()
        .enumerate()
        .map(|(idx, (field_name, _field_type))| {
            let idx = idx as u32;
            quote! {
                #idx => Some( &mut self.#field_name as _)
            }
        });

    
    let component_identifiers = components
        .iter()
        .enumerate()
        .map(|(idx, (_field_name, field_type))| {
            let idx = idx as u32;
            quote! {
                (#field_type::IDENTIFIER, #idx)
            }
        });

    let component_identifiers_recursive = components
        .iter()
        .enumerate()
        .map(|(idx, (_field_name, field_type))| {
            let idx = idx as u32;
            quote! {
                result.push((#field_type::IDENTIFIER, vec![#idx]));
                for (identifier, mut indexes) in #field_type::component_identifiers_recursive() {
                    indexes.insert(0, #idx);
                    result.push((identifier, indexes));
                }
            }
        });

    let components = components.keys().collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote!(
        impl #impl_generics ::shura::entity::ConstIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
        }
        impl #impl_generics #trait_identifier_type for #struct_name #ty_generics #where_clause {}
        impl #impl_generics #trait_type for #struct_name #ty_generics #where_clause {
            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.init(handle, world); )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.finish(world); )*
            }

            fn component(&self, idx: u32) -> Option<&dyn ::shura::component::Component> {
                match idx {
                    #( #component, )*
                    _ => None
                }
            }

            fn component_mut(&mut self, idx: u32) -> Option<&mut dyn ::shura::component::Component> {
                match idx {
                    #( #component_mut, )*
                    _ => None
                }
            }

            fn component_identifiers() -> &'static [(::shura::entity::ConstTypeId, u32)]
            where
                Self: Sized,
            {
                &[
                    #( #component_identifiers, )*
                ]
            }

            fn component_identifiers_recursive() -> Vec<(::shura::entity::ConstTypeId, Vec<u32>)>
            where
                Self: Sized,
            {
                let mut result = vec![];
                #( #component_identifiers_recursive )*
                return result;
            }
        }
    )
}

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let component = component(
        &ast,
        parse_str("::shura::component::Component").unwrap(),
        parse_str("::shura::component::ComponentIdentifier").unwrap(),
    );

    component.into()
}

#[proc_macro_derive(Entity, attributes(shura))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let entity = component(
        &ast,
        parse_str("::shura::entity::Entity").unwrap(),
        parse_str("::shura::entity::EntityIdentifier").unwrap(),
    );
    entity.into()
}

#[proc_macro_attribute]
/// This macro helps setup a cross plattform main method
pub fn app(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item: TokenStream2 = item.into();
    quote!(
        #item

        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(app: ::shura::winit::platform::android::activity::AndroidApp) {
            app(::shura::app::AppConfig::new(app));
        }

        #[cfg(not(target_os = "android"))]
        #[allow(dead_code)]
        fn main() {
            app(::shura::app::AppConfig::new());
        }
    )
    .into()
}
