use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use std::sync::Mutex;
use std::{collections::HashSet, sync::OnceLock};
use syn::{
    parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Expr, Fields, LitStr, Type,
    TypePath,
};

const IDENT_NAME: &'static str = "shura";

fn inner_component(data_struct: &DataStruct) -> Option<(Ident, TypePath)> {
    let mut inner = None;
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("inner") {
                                match &field.ty {
                                    Type::Path(type_name) => {
                                        let field_name = field
                                            .ident
                                            .as_ref()
                                            .expect("Inner field must be named!");

                                        if inner.is_some() {
                                            panic!("Inner already defined!");
                                        }
                                        inner = Some((field_name.clone(), type_name.clone()));
                                    }
                                    _ => panic!("Cannot extract the inner component!"),
                                };
                            }
                            Ok(())
                        })
                        .expect("Cannot extract the inner component!");
                    }
                }
            }
        }
        _ => (),
    };
    return inner;
}

type RenderGroups = HashMap<String, (LitStr, TypePath, Vec<Ident>)>;
type AllComponents = HashSet<Ident>;
type NamedComponents = HashMap<String, (LitStr, Vec<Ident>)>;

fn component_data(data_struct: &DataStruct) -> (RenderGroups, AllComponents, NamedComponents) {
    let mut all_components: AllComponents = Default::default();
    let mut named_components: NamedComponents = Default::default();
    let mut render_groups: RenderGroups = Default::default();
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            let field_name =
                                field.ident.as_ref().expect("Struct fields must be named!");
                            match &field.ty {
                                Type::Path(type_name) => {
                                    if meta.path.is_ident("component") {
                                        all_components.insert(field_name.clone());
                                        if let Ok(value) = meta.value() {
                                            if let Some(name) = value.parse::<LitStr>().ok() {
                                                render_groups
                                                    .entry(name.value())
                                                    .or_insert_with(|| {
                                                        (name, type_name.clone(), Vec::default())
                                                    })
                                                    .2
                                                    .push(field_name.clone());
                                            }
                                        }
                                    } else if meta.path.is_ident("tag") {
                                        if let Ok(value) = meta.value() {
                                            if let Some(name) = value.parse::<LitStr>().ok() {
                                                named_components
                                                    .entry(name.value())
                                                    .or_insert_with(|| (name, Vec::default()))
                                                    .1
                                                    .push(field_name.clone());
                                            }
                                        }
                                    }
                                }
                                _ => panic!("Cannot extract the type of the entity!"),
                            };
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
    return (render_groups, all_components, named_components);
}

fn entity_name(ast: &DeriveInput, attr_name: &str) -> Option<Expr> {
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
    let struct_identifier = entity_name(&ast, "name").unwrap_or(parse_quote!(#struct_name_str));
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

    let (render_groups, components, named_components) = component_data(&data_struct);
    let names = named_components.iter().map(|(_, (name, _))| name);

    let component_collection = named_components.iter().map(|(_, (name_lit, field_name))| {
        return quote! {
            #name_lit => Some( Box::new([#( &self.#field_name as _), *].into_iter()) )
        };
    });

    let component_collection_mut = named_components.iter().map(|(_, (name_lit, field_name))| {
        return quote! {
            #name_lit => Some( Box::new([#( &mut self.#field_name as _), *].into_iter()) )
        };
    });

    let buffer = render_groups
        .iter()
        .map(|(_, (group_name, type_name, field_names))| {
            quote! {
                let buffer = buffers.get_mut::<<<#type_name as shura::component::ComponentCollection>::Component as shura::component::Component>::Instance>(#group_name).expect("Cannot find RenderGroup #group_name!");
                for e in entities.clone() {
                    #( e.#field_names.buffer_all(world, buffer); ) *
                }
            }
                
        });

    let components_iter = components.iter();
    let components_iter2 = components.iter();

    return quote!(
        impl #impl_generics ::shura::entity::EntityIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::entity::EntityTypeId = ::shura::entity::EntityTypeId::new(#hash);

            fn entity_type_id(&self) -> ::shura::entity::EntityTypeId {
                Self::IDENTIFIER
            }
        }

        impl #impl_generics ::shura::entity::Entity for #struct_name #ty_generics #where_clause {
            fn buffer<'a>(
                entities: impl ::shura::entity::RenderEntityIterator<'a, Self>,
                buffers: &mut ::shura::graphics::RenderGroupManager,
                world: &::shura::physics::World,
            ) {
                use shura::component::ComponentCollection;
                #( #buffer )*
            }

            fn named_components() -> &'static [&'static str] where Self: Sized{
                return &[ #( #names, )*];
            }

            fn component_collections<'a>(
                &'a self,
            ) -> Box<dyn DoubleEndedIterator<Item = &dyn shura::component::ComponentCollection> + 'a>{
                Box::new([ #( &self.#components_iter as _, )* ].into_iter())
            }

            fn component_collections_mut<'a>(
                &'a mut self,
            ) -> Box<dyn DoubleEndedIterator<Item = &mut dyn shura::component::ComponentCollection> + 'a>{
                Box::new([ #( &mut self.#components_iter2 as _, )* ].into_iter())
            }

            fn component_collection<'a>(&'a self, name: &'static str) -> Option<Box<dyn DoubleEndedIterator<Item = &dyn shura::component::ComponentCollection> + 'a>> {
                match name {
                    #( #component_collection, )*
                    _ => None
                }
            }

            fn component_collection_mut<'a>(&'a mut self, name: &'static str) -> Option<Box<dyn DoubleEndedIterator<Item = &mut dyn shura::component::ComponentCollection> + 'a>> {
                match name {
                    #( #component_collection_mut, )*
                    _ => None
                }
            }
        }
    )
    .into();
}

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };

    let struct_name = ast.ident.clone();
    let (inner_field, inner_type) = inner_component(&data_struct)
        .expect("Cannot find inner component. Define it with #[shura(inner)]");
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    return quote!(
        impl #impl_generics ::shura::component::Component for #struct_name #ty_generics #where_clause {
            type Instance = <#inner_type as ::shura::component::Component>::Instance;
        
            fn instance(&self, world: &shura::physics::World) -> Self::Instance {
                self.#inner_field.instance(world)
            }
        
            fn active(&self) -> bool {
                self.#inner_field.active()
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
        fn android_main(app: ::shura::winit::platform::android::activity::AndroidApp) {
            shura_main(::shura::app::AppConfig::new(app));
        }

        #[cfg(not(target_os = "android"))]
        #[allow(dead_code)]
        fn main() {
            shura_main(::shura::app::AppConfig::new());
        }
    )
    .into()
}
