use proc_macro::TokenStream;
use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Expr, Fields, LitStr, Type,
};

type AllComponents = HashMap<Ident, Type>;
type TaggedComponents = HashMap<String, Ident>;

const IDENT_NAME: &str = "shura";

#[derive(Default)]
struct ComponentBundleData {
    components: AllComponents,
    tagged_components: TaggedComponents,
}

fn component_data(data_struct: &DataStruct) -> ComponentBundleData {
    let mut data = ComponentBundleData::default();
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident(IDENT_NAME) {
                        attr.parse_nested_meta(|meta| {
                            let field_name = field.ident.as_ref().unwrap();
                            if meta.path.is_ident("component") {
                                data.components.insert(field_name.clone(), field.ty.clone());
                                if let Ok(value) = meta.value() {
                                    if let Ok(name) = value.parse::<LitStr>() {
                                        let value = name.value();
                                        if data.tagged_components.contains_key(&value) {
                                            panic!("'#[shura(component=\"{value}\")]' must be unique per Entity!");
                                        }

                                        data.tagged_components.insert(value, field_name.clone());
                                    }
                                }
                            }
                            Ok(())
                        })
                        .expect(
                            "Define your components like the this: '#[shura(component)]'",
                        );
                    }
                }
            }
        }
        _ => panic!("Fields must be named!"),
    };
    data
}

fn entity_name(ast: &DeriveInput) -> Option<Expr> {
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

fn component_assets(ast: &DeriveInput) -> Vec<(String, Type, Expr)> {
    let mut result = vec![];
    for attr in &ast.attrs {
        if attr.path().is_ident(IDENT_NAME) {
            let mut asset_name = None;
            let mut asset_type = None;
            let mut asset_action = None;

            let _ = attr.parse_nested_meta(|meta| {
                let value = meta.value()?;
                if meta.path.is_ident("asset") {
                    if let Ok(lit_str) = value.parse::<LitStr>() {
                        asset_name = Some(lit_str.value());
                    }
                } else if meta.path.is_ident("ty") {
                    if let Ok(ty) = value.parse::<Type>() {
                        asset_type = Some(ty);
                    }
                } else if meta.path.is_ident("action") {
                    if let Ok(expr) = value.parse::<Expr>() {
                        asset_action = Some(expr);
                    }
                }

                Ok(())
            });

            if asset_name.is_some() || asset_type.is_some() || asset_action.is_some() {
                let asset_name =
                    asset_name.expect("Expected 'asset' argument in 'shura' attribute");
                let asset_type = asset_type.expect("Expected 'ty' argument in 'shura' attribute");
                let asset_action =
                    asset_action.expect("Expected 'action' argument in 'shura' attribute");

                result.push((asset_name, asset_type, asset_action))
            }
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
    let ComponentBundleData {
        components,
        tagged_components,
    } = component_data(data_struct);
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
    let buffer_children = components.iter().map(|(field_name, ty)| {
        quote! {
            #ty::buffer(iter.clone().map(|e| &e.#field_name), ctx);
        }
    });

    let components = components.keys().collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let assets: Vec<(String, Type, Expr)> = component_assets(ast);
    let assets = assets.iter().map(|(asset_name, asset_type, action)| {
        quote! {
            let mut asset = ctx.assets.get_dyn_mut(#asset_name);
            if asset.needs_update() {
                let asset = asset.downcast_mut::<#asset_type>().unwrap();
                for obj in iter.clone() {
                    fn exec(obj: &#struct_name #ty_generics, asset: &mut #asset_type, ctx: &::shura::context::Context, mut f: impl FnMut(&#struct_name #ty_generics, &mut #asset_type, &::shura::context::Context)) {
                        (f)(obj, asset, ctx)
                    }
                    exec(obj, asset, ctx, #action)
                }
            }
        }
    });

    quote!(
        impl #impl_generics ::shura::component::Component for #struct_name #ty_generics #where_clause {
            fn buffer<'a>(
                iter: impl ::shura::component::BufferComponentIterator<'a, Self>,
                ctx: &::shura::context::Context
            ) where
                Self: Sized,
            {
                #( #assets )*
                #( #buffer_children )*
            }

            fn init(&mut self, handle: ::shura::entity::EntityHandle, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.init(handle, world); )*
            }

            fn finish(&mut self, world: &mut ::shura::physics::World) {
                use ::shura::component::Component;
                #( self.#components.finish(world); )*
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

            fn remove_from_world(&self, world: &mut ::shura::physics::World) {
                #( self.#components.remove_from_world(world); )*
            }

            fn tags() -> &'static [&'static str] where Self: Sized{
                return &[ #( #names, )* ];
            }
        }
    )
}

#[proc_macro_derive(Component, attributes(shura))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let component = component(&ast);

    quote!(
        #component
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
    let hash = const_fnv1a_hash::fnv1a_hash_str_32(&struct_identifier_str);

    let component = component(&ast);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote!(
        #component

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
