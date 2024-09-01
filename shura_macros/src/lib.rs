use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashMap;
use syn::{parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Expr, LitStr, Type};

type AllComponents = HashMap<Ident, (String, Type)>;

const IDENT_NAME: &str = "shura";

fn component_data(data_struct: &DataStruct) -> AllComponents {
    let mut data: AllComponents = Default::default();
    for (i, field) in data_struct.fields.iter().enumerate() {
        for attr in &field.attrs {
            if attr.path().is_ident(IDENT_NAME) {
                attr.parse_nested_meta(|meta| {
                    let field_name = field.ident.as_ref().unwrap();
                    if meta.path.is_ident("component") {
                        let number = i.to_string();
                        data.insert(field_name.clone(), (number, field.ty.clone()));
                    }

                    return Ok(());
                })
                .expect("Define your components like the this: '#[shura(component)]'");
            }
        }
    }
    return data;
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

fn component(ast: &DeriveInput) -> TokenStream2 {
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };
    let struct_name = ast.ident.clone();
    let components = component_data(data_struct);

    let component = components
        .iter()
        .map(|(field_name, (name, field_type))| {
            quote! {
                #name | <#field_type>::NAME => Some( &self.#field_name as _)
            }
        });
    let component_mut = components
        .iter()
        .map(|(field_name, (name, field_type))| {
            quote! {
                #name | <#field_type>::NAME => Some( &mut self.#field_name as _)
            }
        });

    let tags = components.iter().map(|(_field_name, (name, field_type))| {
        quote! {
            #name,
            <#field_type>::NAME
        }
    });

    let tags_recursive = components.iter().map(|(_field_name, (name, field_type))| {
        quote! {
            result.push((<#field_type>::NAME, vec![#name]));
            for (identifier, mut indexes) in <#field_type>::tags_recursive() {
                indexes.insert(0, #name);
                result.push((identifier, indexes));
            }
        }
    });

    let components = components.keys().collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote!(
        impl #impl_generics ::shura::component::Component for #struct_name #ty_generics #where_clause {
            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.init(handle, world); )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.finish(world); )*
            }

            fn remove_from_world(&self, world: &mut ::shura::physics::World) {
                #( self.#components.remove_from_world(world); )*
            }

            fn component_dyn(&self, name: &str) -> Option<&dyn ::shura::component::Component> {
                match name {
                    #( #component, )*
                    _ => None
                }
            }

            fn component_mut_dyn(&mut self, name: &str) -> Option<&mut dyn ::shura::component::Component> {
                match name {
                    #( #component_mut, )*
                    _ => None
                }
            }

            fn tags() -> &'static [&'static str] where Self: Sized {
                return &[ #( #tags, )* ];
            }

            fn tags_recursive() -> Vec<(&'static str, Vec<&'static str>)>
            where
                Self: Sized,
            {
                let mut result = vec![];
                #( #tags_recursive )*
                return result;
            }

            fn as_component(&self) -> &dyn ::shura::component::Component {
                self as _
            }

            fn as_component_mut(&mut self) -> &mut dyn ::shura::component::Component {
                self as _
            }
        }
    )
}

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);    
    let struct_name = ast.ident.clone();
    let struct_name_str = struct_name.to_string();
    let struct_identifier = identifier_name(&ast).unwrap_or(parse_quote!(
        concat!(module_path!(), "::", #struct_name_str)
    ));
    let component = component(&ast);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote!(
        impl #impl_generics ::shura::component::ComponentIdentifier for #struct_name #ty_generics #where_clause {
            const NAME: &'static str = #struct_identifier;
        }
        #component
    )
    .into()
}

#[proc_macro_derive(Entity, attributes(shura))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = ast.ident.clone();
    let struct_name_str = struct_name.to_string();
    let struct_identifier = identifier_name(&ast).unwrap_or(parse_quote!(
        concat!(module_path!(), "::", #struct_name_str)
    ));
    let component = component(&ast);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote!(
        #component

        impl #impl_generics ::shura::entity::Entity for #struct_name #ty_generics #where_clause {}
        impl #impl_generics ::shura::entity::EntityIdentifier for #struct_name #ty_generics #where_clause {
            const NAME: &'static str = #struct_identifier;
        }
    )
    .into()
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
