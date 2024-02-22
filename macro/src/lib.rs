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

const IDENT_NAME: &str = "shura";

fn inner_components(data_struct: &DataStruct) -> Vec<(Ident, TypePath)> {
    let mut inner = vec![];
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("component") {
                                match &field.ty {
                                    Type::Path(type_name) => {
                                        let field_name = field.ident.as_ref().unwrap();
                                        inner.push((field_name.clone(), type_name.clone()));
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
        _ => panic!("Fields must be named!"),
    };
    inner
}

// type RenderGroups = HashSet<(Ident, LitStr, TypePath)>;
// type RenderComponents = Vec<(Ident, Ident)>;
type RenderGroups = HashMap<String, (LitStr, TypePath, Vec<Ident>)>;
type AllComponents = HashSet<Ident>;
type TaggedComponents = HashMap<String, Ident>;
type Handlefields = Vec<Ident>;

fn component_data(
    data_struct: &DataStruct,
) -> (Handlefields, RenderGroups, AllComponents, TaggedComponents) {
    let mut all_components: AllComponents = Default::default();
    let mut tagged_components: TaggedComponents = Default::default();
    let mut render_groups: RenderGroups = Default::default();
    let mut handle_fields: Handlefields = Default::default();
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            let field_name =
                                field.ident.as_ref().unwrap();
                            match &field.ty {
                                Type::Path(type_name) => {
                                    if meta.path.is_ident("handle") {
                                        let field_name =
                                            field.ident.as_ref().unwrap();
                                        handle_fields.push(field_name.clone());
                                    } else if meta.path.is_ident("component") {
                                        all_components.insert(field_name.clone());
                                        if let Ok(value) = meta.value() {
                                            if let Ok(name) = value.parse::<LitStr>() {
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
                                            if let Ok(name) = value.parse::<LitStr>() {
                                                tagged_components.insert(name.value(), field_name.clone());
                                            }
                                        }
                                    }
                                }
                                _ => panic!("Cannot extract the type of the component!"),
                            };
                            Ok(())
                        })
                        .expect(
                            "Define your component like the this: #[shura(component = \"<render_group>\")] or #[shura(component)]",
                        );
                    }
                }
            }
        }
        _ => panic!("Fields must be named!"),
    };
    (
        handle_fields,
        render_groups,
        all_components,
        tagged_components,
    )
}

fn entity_name(ast: &DeriveInput) -> Option<Expr> {
    let attr_name = "name";
    let mut result: Option<Expr> = None;
    for attr in &ast.attrs {
        if attr.path().is_ident(IDENT_NAME) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(attr_name) {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    assert!(result.is_none(), "{attr_name} is already defined!");
                    result = Some(syn::parse_str::<Expr>(&s.value()).unwrap())
                }
                Ok(())
            })
            .unwrap();
        }
    }
    result
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
    let struct_identifier = entity_name(&ast).unwrap_or(parse_quote!(#struct_name_str));
    let struct_identifier_str = struct_identifier.to_token_stream().to_string();
    let mut hashes = USED_COMPONENT_HASHES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    let hash = const_fnv1a_hash::fnv1a_hash_str_32(&struct_identifier_str);
    if hashes.contains(&hash) {
        panic!("Entity with this identifier already exists! Consider giving a custom identifier with #[shura(name=\"<unique_identifier>\")");
    }
    hashes.insert(hash);
    drop(hashes); // Free the mutex lock

    let (handles, render_groups, components, tagged_components) = component_data(data_struct);
    let names = tagged_components.keys();

    let component = tagged_components.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( &self.#field_name as _)
        }
    });

    let component_mut = tagged_components.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( &mut self.#field_name as _)
        }
    });

    let buffer = render_groups
        .iter()
        .map(|(_, (group_name, type_name, field_names))| {
            quote! {
                let buffer = buffers.get_mut::<<#type_name as ::shura::component::Component>::Instance>(#group_name).expect(&format!("Cannot find RenderGroup {}!", #group_name));
                if buffer.needs_update() {
                    for e in entities.clone() {
                        #( e.#field_names.buffer(world, buffer); ) *
                    }
                }
            }
        });

    let components = components.iter().collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote!(
        impl #impl_generics ::shura::entity::EntityIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::entity::EntityId = ::shura::entity::EntityId::new(#hash);

            fn entity_type_id(&self) -> ::shura::entity::EntityId {
                Self::IDENTIFIER
            }
        }

        impl #impl_generics ::shura::entity::Entity for #struct_name #ty_generics #where_clause {
            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.init(handle, world); )*
                #( self.#handles = handle; )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.finish(world); )*
                #( self.#handles = Default::default(); )*
            }

            fn buffer<'a>(
                entities: impl ::shura::entity::RenderEntityIterator<'a, Self>,
                buffers: &mut ::shura::graphics::RenderGroupManager,
                world: &::shura::physics::World,
            ) {
                #( #buffer )*
            }

            fn tags() -> &'static [&'static str] where Self: Sized{
                return &[ #( #names, )*];
            }

            fn components<'a>(
                &'a self,
            ) -> Box<dyn DoubleEndedIterator<Item = &dyn ::shura::component::Component> + 'a>{
                Box::new([ #( &self.#components as _, )* ].into_iter())
            }

            fn components_mut<'a>(
                &'a mut self,
            ) -> Box<dyn DoubleEndedIterator<Item = &mut dyn ::shura::component::Component> + 'a>{
                Box::new([ #( &mut self.#components as _, )* ].into_iter())
            }

            fn component(&self, name: &'static str) -> Option<&dyn ::shura::component::Component> {
                match name {
                    #( #component, )*
                    _ => None
                }
            }

            fn component_mut(&mut self, name: &'static str) -> Option<&mut dyn ::shura::component::Component> {
                match name {
                    #( #component_mut, )*
                    _ => None
                }
            }
        }
    )
    .into()
}

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };

    let struct_name = ast.ident.clone();
    let inner = inner_components(data_struct);
    let field_names = inner.iter().map(|i| &i.0).collect::<Vec<_>>();
    let instance = inner
        .iter()
        .next()
        .map(|(_, ty)| quote!( <#ty as ::shura::component::Component>::Instance ))
        .unwrap_or_else(|| quote!(()));
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote!(
        impl #impl_generics ::shura::component::Component for #struct_name #ty_generics #where_clause {
            type Instance = #instance;
            fn buffer(
                &self,
                world: &::shura::physics::World,
                render_group: &mut ::shura::graphics::RenderGroup<Self::Instance>,
            ) where
                Self: Sized,
            {
                #( self.#field_names.buffer(world, render_group); )*
            }

            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                #( self.#field_names.init(handle, world); )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                #( self.#field_names.finish(world); )*
            }

            fn remove_from_world(&self, world: &mut ::shura::physics::World) {
                #( self.#field_names.remove_from_world(world); )*
            }
        }
    )
    .into()
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
