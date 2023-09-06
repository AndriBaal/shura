use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
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

#[proc_macro_derive(Component, attributes(position, name, buffer, non_parallel))]
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
        quote!(shura::#position_field_name)
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

    let non_parallel = is_path_set(&ast, "non_parallel");
    let buffer_method = if cfg!(feature = "rayon") && !non_parallel {
        quote!(par_buffer)
    } else {
        quote!(buffer)
    };

    let buffer_impl = if buffer_fields.is_empty() {
        quote!(
            const INSTANCE_SIZE: u64 = std::mem::size_of::<shura::InstancePosition>() as u64;
            fn buffer_with(
                mut helper: BufferHelper<Self>,
                each: impl Fn(&mut Self) + Send + Sync,
            ) {
                helper.#buffer_method(|c: &mut Self| {
                    each(c);
                    c.position().instance(helper.world)
                })
            }
            fn buffer(mut helper: BufferHelper<Self>) {
                helper.#buffer_method(|component| component.position().instance(helper.world))
            }
        )
    } else {
        quote!(
            const INSTANCE_SIZE: u64 = (std::mem::size_of::<shura::InstancePosition>() + #(std::mem::size_of::<#buffer_types>())+*) as u64;
            fn buffer_with(
                mut helper: shura::BufferHelper<Self>,
                each: impl Fn(&mut Self) + Send + Sync
            ) {
                #[repr(C)]
                #[derive(Clone, Copy)]
                struct Instance {
                    #position_field_name: shura::InstancePosition,
                    #(#struct_fields),*
                }
                unsafe impl bytemuck::Pod for Instance {}
                unsafe impl bytemuck::Zeroable for Instance {}

                helper.#buffer_method(|component| {
                    (each)(component);
                    Instance {
                        #position_field_name: component.position().instance(helper.world),
                        #(#buffer_fields: component.#buffer_fields),*
                    }
                });
            }

            fn buffer(
                mut helper: shura::BufferHelper<Self>
            ) {
                #[repr(C)]
                #[derive(Clone, Copy)]
                struct Instance {
                    #position_field_name: shura::InstancePosition,
                    #(#struct_fields),*
                }
                unsafe impl bytemuck::Pod for Instance {}
                unsafe impl bytemuck::Zeroable for Instance {}

                helper.#buffer_method(|component| {
                    Instance {
                        #position_field_name: component.position().instance(helper.world),
                        #(#buffer_fields: component.#buffer_fields),*
                    }
                });
            }
        )
    };

    let init_finish = if has_position {
        quote!(
            fn init(&mut self, handle: shura::ComponentHandle, world: &mut shura::World) {
                #var_position_field_name.init(handle, world)
            }

            fn finish(&mut self, world: &mut shura::World) {
                #var_position_field_name.finish(world)
            }
        )
    } else {
        quote!(
            fn init(&mut self, handle: shura::ComponentHandle, world: &mut shura::World) {}
            fn finish(&mut self, world: &mut shura::World) {}
        )
    };

    return quote!(
        impl #impl_generics shura::ComponentIdentifier for #struct_name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #struct_identifier;
            const IDENTIFIER: shura::ComponentTypeId = shura::ComponentTypeId::new(#hash);
        }

        impl #impl_generics shura::Component for #struct_name #ty_generics #where_clause {
            fn position(&self) -> &dyn shura::Position {
                &#var_position_field_name
            }

            #init_finish

            #buffer_impl

            fn component_type_id(&self) -> shura::ComponentTypeId {
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
        fn android_main(app: shura::AndroidApp) {
            shura_main(shura::ShuraConfig::default(app));
        }

        #[cfg(not(target_os = "android"))]
        #[allow(dead_code)]
        fn main() {
            shura_main(shura::ShuraConfig::default());
        }
    )
    .into()
}

#[proc_macro_derive(Resource, attributes(global))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let component: TokenStream2 = component(&ast).into();
    let struct_name = ast.ident.clone();
    let global = is_path_set(&ast, "global");
    let config = if global {
        quote!(GLOBAL_RESOURCE)
    } else {
        quote!(RESOURCE)
    };

    let resource = quote!(
        #component

        impl shura::ComponentController for #struct_name {
            const CONFIG: shura::ComponentConfig = shura::ComponentConfig::#config;
        }
    );

    return resource.into();
}
