use proc_macro::TokenStream;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
use std::sync::Mutex;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    Data, DataStruct, DeriveInput, Expr, Fields, LitStr, parse_macro_input, parse_quote, Type,
};

type RenderGroups = HashMap<String, (LitStr, Type, Vec<Ident>)>;
type HandleFields = Vec<Ident>;
type AllComponents = HashSet<Ident>;
type TaggedComponents = HashMap<String, Ident>;
type AllBundles = HashMap<Ident, Type>;
type TaggedBundles = HashMap<String, Ident>;

const IDENT_NAME: &str = "shura";
static USED_COMPONENT_HASHES: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

#[derive(Default)]
struct ComponentBundleData {
    render_groups: RenderGroups,
    handle_fields: HandleFields,
    components: AllComponents,
    tagged_components: TaggedComponents,
    bundles: AllBundles,
    tagged_bundles: TaggedBundles,
}

fn bundle_data(data_struct: &DataStruct) -> ComponentBundleData {
    let mut data = ComponentBundleData::default();
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            let field_name = field.ident.as_ref().unwrap();
                            if meta.path.is_ident("handle") {
                                let field_name =
                                    field.ident.as_ref().unwrap();
                                data.handle_fields.push(field_name.clone());
                            } else if meta.path.is_ident("component") {
                                data.components.insert(field_name.clone());
                                if let Ok(value) = meta.value() {
                                    if let Ok(name) = value.parse::<LitStr>() {
                                        data.render_groups
                                            .entry(name.value())
                                            .or_insert_with(|| {
                                                (name, field.ty.clone(), Vec::default())
                                            })
                                            .2
                                            .push(field_name.clone());
                                    }
                                }
                            } else if meta.path.is_ident("bundle") {
                                data.bundles.insert(field_name.clone(), field.ty.clone());
                            } else if meta.path.is_ident("tag") {
                                if let Ok(value) = meta.value() {
                                    if let Ok(name) = value.parse::<LitStr>() {
                                        let value = name.value();
                                        if data.tagged_bundles.contains_key(&value) || data.tagged_components.contains_key(&value) {
                                            panic!("'#[shura(tag=\"{value}\")]' must be unique per Entity!");
                                        }

                                        if data.components.contains(field_name) {
                                            data.tagged_components.insert(value, field_name.clone());
                                        } else if data.bundles.contains_key(field_name) {
                                            data.tagged_bundles.insert(value, field_name.clone());
                                        } else {
                                            panic!("'#[shura(tag=\"{value}\"]' must be after a bundle or component declaration");
                                        }
                                    }
                                }
                            }
                            Ok(())
                        })
                        .expect(
                            "Define your components like the this: '#[shura(component = \"<render_group>\")]' or '#[shura(component)]'",
                        );
                    }
                }
            }
        }
        _ => panic!("Fields must be named!"),
    };
    return data;
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

fn inner_components(data_struct: &DataStruct) -> Vec<(Ident, Type)> {
    let mut inner = vec![];
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("component") {
                                let field_name = field.ident.as_ref().unwrap();
                                inner.push((field_name.clone(), field.ty.clone()));
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

fn component_bundle(ast: &DeriveInput) -> TokenStream2 {
    let data_struct = match ast.data {
        Data::Struct(ref data_struct) => data_struct,
        _ => panic!("Must be a struct!"),
    };
    let struct_name = ast.ident.clone();
    let ComponentBundleData {
        handle_fields,
        render_groups,
        components,
        tagged_components,
        bundles,
        tagged_bundles,
    } = bundle_data(data_struct);
    let names = tagged_components.keys().chain(tagged_bundles.keys());

    let component = tagged_components.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( ::shura::component::ComponentType::Component(&self.#field_name as _))
        }
    });
    let component_mut = tagged_components.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( ::shura::component::ComponentTypeMut::Component(&mut self.#field_name as _))
        }
    });
    let component_buffer = render_groups
        .iter()
        .map(|(_, (group_name, type_name, field_names))| {
            quote! {
                let buffer = render_groups.get_mut::<<#type_name as ::shura::component::Component>::Instance>(#group_name).expect(&format!("Cannot find render group \"{}\"! Try declaring it when setting up the scene.", #group_name));
                if buffer.needs_update() {
                    for e in iter.clone() {
                        #( e.#field_names.buffer(world, buffer); ) *
                    }
                }
            }
        });

    let bundle = tagged_bundles.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( ::shura::component::ComponentType::ComponentBundle(&self.#field_name as _))
        }
    });
    let bundle_mut = tagged_bundles.iter().map(|(name_lit, field_name)| {
        quote! {
            #name_lit => Some( ::shura::component::ComponentTypeMut::ComponentBundle(&mut self.#field_name as _))
        }
    });
    let bundle_buffer = bundles.iter().map(|(field, ty)| {
        quote! {
            #ty::buffer(iter.clone().map(|e| &e.#field), render_groups, world);
        }
    });

    let components = components.iter().collect::<Vec<_>>();
    let bundles = bundles.keys().collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote!(

        impl #impl_generics ::shura::component::ComponentBundle for #struct_name #ty_generics #where_clause {
            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.init(handle, world); )*
                #( self.#bundles.init(handle, world); )*
                #( self.#handle_fields = handle; )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.finish(world); )*
                #( self.#bundles.finish(world); )*
                #( self.#handle_fields = Default::default(); )*
            }

            fn buffer<'a>(
                iter: impl ::shura::component::BufferComponentBundleIterator<'a, Self>,
                render_groups: &mut ::shura::graphics::RenderGroupManager,
                world: &::shura::physics::World,
            ) {
                use ::shura::graphics::RenderGroupCommon;
                use ::shura::component::Component;
                #( #component_buffer )*
                #( #bundle_buffer )*
            }

            fn tags() -> &'static [&'static str] where Self: Sized{
                return &[ #( #names, )* ];
            }

            fn component(&self, name: &'static str) -> Option<::shura::component::ComponentType> {
                match name {
                    #( #component, )*
                    #( #bundle, )*
                    _ => None
                }
            }

            fn component_mut(&mut self, name: &'static str) -> Option<::shura::component::ComponentTypeMut> {
                match name {
                    #( #component_mut, )*
                    #( #bundle_mut, )*
                    _ => None
                }
            }
        }
    )
}

#[proc_macro_derive(ComponentBundle, attributes(shura))]
pub fn derive_component_bundle(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let component_bundle = component_bundle(&ast);

    quote!(
        #component_bundle
    )
    .into()
}

#[proc_macro_derive(Entity, attributes(shura))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
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
        panic!("Entity with this identifier already exists! Consider giving a custom identifier with '#[shura(name=\"<unique_identifier>\")]'");
    }
    hashes.insert(hash);
    drop(hashes); // Free the mutex lock

    let component_bundle = component_bundle(&ast);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote!(
        #component_bundle

        impl #impl_generics ::shura::entity::Entity for #struct_name #ty_generics #where_clause {}

        impl #impl_generics ::shura::entity::EntityIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: ::shura::entity::EntityId = ::shura::entity::EntityId::new(#hash);

            fn entity_type_id(&self) -> ::shura::entity::EntityId {
                Self::IDENTIFIER
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
