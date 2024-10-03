extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Path};

#[proc_macro_derive(Namespace, attributes(roles))]
pub fn derive_define_namespace(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = if let Data::Enum(namespaces) = input.data {
        namespaces.variants
    } else {
        panic!("Namespace can only be derived for enums");
    };

    let mut to_string_match_arms = vec![];
    //let mut from_string_match_arms = vec![];
    let mut variant_roles = vec![];
    let mut get_roles_match_arms = vec![];

    for variant in variants {
        let variant_name = &variant.ident;
        let attrs = &variant.attrs;

        let roles_path = attrs
            .iter()
            .find(|attr| attr.path().is_ident("roles"))
            .and_then(|attr| parse_attribute(attr));

        if roles_path.is_some() {
            let roles = roles_path.unwrap();
            variant_roles.push((variant_name.clone(), roles.clone()));

            /*
            impl Namespaces {
                pub fn get_roles(&self) -> Option<Box<dyn RoleHierarchy>> {
                    match self {
                        Namespaces::User(_) => None,
                        Namespaces::Post(_) => Some(Box::new(PostRoles::default())),
                        Namespaces::Group(_) => Some(Box::new(GroupRoles::default())),
                    }
                }
            }
            */

            let get_roles_match_arm = quote! {
                #name::#variant_name(_) => Some(Box::new(#roles::default())),
            };

            get_roles_match_arms.push(get_roles_match_arm);
        } else {
            get_roles_match_arms.push(quote! {
                #name::#variant_name(_) => None,
            });
        }

        // impl ToString snippet
        let variant_match = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                quote! {
                    #name::#variant_name(Some(id)) => format!("{}_{}", stringify!(#variant_name), id),
                    #name::#variant_name(None) => stringify!(#variant_name).to_string(),
                }
            }
            _ => panic!("Namespace requires enum variants with exactly one unnamed field"),
        };

        to_string_match_arms.push(variant_match);

        // impl FromStr snippet
        // let variant_from_str = match &variant.fields {
        //     Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
        //         quote! {
        //             s if s.starts_with(stringify!(#variant_name)) => {
        //                 let id = s.split('_').nth(1).unwrap().parse().unwrap();
        //                 Ok(#name::#variant_name(Some(id)))
        //             }
        //             s if s == stringify!(#variant_name) => Ok(#name::#variant_name(None)),
        //         }
        //     }
        //     _ => panic!("Namespace requires enum variants with exactly one unnamed field"),
        // };

        // from_string_match_arms.push(variant_from_str);
    }

    // Generate the ToString implementation
    let to_string_impl = quote! {
        impl NamespaceToString for #name {
            fn to_string(&self) -> String {
                match self {
                    #(#to_string_match_arms)*
                }
            }
        }
    };

    let get_roles_method = quote! {
        impl NamespaceRole for #name {
            fn get_roles(&self) -> Option<Box<dyn RoleHierarchy>> {
                match self {
                    #(#get_roles_match_arms)*
                }
            }
        }
    };

    // let from_string_impl = quote! {
    //     impl FromStr for #name {
    //         type Err = ();

    //         fn from_str(s: &str) -> Result<Self, ()> {
    //             match s {
    //                 #(#from_string_match_arms)*
    //                 _ => Err(()),
    //             }
    //         }
    //     }
    // };

    // let role_to_node_arms = variant_roles.iter().map(|(variant_name, associated_enum)| {
    //     let associated_enum_name = associated_enum.get_ident().unwrap();

    //     // get variants of enum

    //     let variant_roles =

    //     quote! {
    //         #associated_enum_name::#variant_name => Node::new(#name::#variant_name(group_id.clone()).to_string(), self.to_string()),
    //     }
    // });

    // let to_node_impl_for_roles = quote! {
    //     impl ToNode for #name {
    //         fn to_node(&self, group_id: Option<String>) -> Node {
    //             match self {
    //                 #(#role_to_node_arms)*
    //             }
    //         }
    //     }
    // };

    // Generate the get_roles method
    // let get_roles_match_arms = variant_roles.iter().map(|(variant_name, associated_enum)| {
    //     quote! {
    //         #name::#variant_name(_) => AllRoles::#associated_enum(#associated_enum::default()),
    //     }
    // });

    // let get_roles_method = quote! {
    //     impl #name {
    //         pub fn get_roles(&self) -> AllRoles {
    //             match self {
    //                 #(#get_roles_match_arms)*
    //             }
    //         }
    //     }
    // };

    let gen = quote! {
        #to_string_impl
        //#from_string_impl
        #get_roles_method

        impl NamespaceToStringAndRole for #name {}
    };

    gen.into()
}

#[proc_macro_derive(ToNode, attributes(namespace))]
pub fn derive_to_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let namespace_path = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("namespace"))
        .and_then(|attr| parse_attribute(attr))
        .expect("Expected #[namespace] attribute with a path");

    let gen = quote! {
        impl rbac::ToNode for #struct_name {
            fn to_node(&self, parent_id: Option<String>) -> Node {
                Node::new(Box::new(#namespace_path(parent_id)), self.id.to_string())
            }
        }
    };

    gen.into()
}

#[proc_macro_derive(Role, attributes(namespace, child_of))]
pub fn derive_role(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let namespace_path = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("namespace"))
        .and_then(parse_attribute)
        .expect("Expected #[namespace] attribute with a valid path");

    if let Data::Enum(data_enum) = input.data {
        if data_enum.variants.len() == 0 {
            return TokenStream::new();
        }

        let to_string_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let variant_str = variant_name.to_string();
            quote! {
                #name::#variant_name => #variant_str.to_string(),
            }
        });

        let to_node_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            quote! {
                #name::#variant_name => Node::new(Box::new(#namespace_path(group_id.clone())), self.to_string()),
            }
        });

        /*
        #[derive(Role)]
        #[namespace(Namespaces::Group)]
        pub enum GroupRoles {
            Admin,
            #[child_of(Admin)]
            Member,
        }
        impl GroupRoles {
            pub fn iter_hierarchy(mut f: impl FnMut(Self, Self)) {
                f(GroupRoles::Admin, GroupRoles::Member);
            }
        }
        */

        let child_of_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let child_of = variant
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("child_of"));

            if child_of.is_none() {
                return quote! {};
            }

            let child_of = child_of.unwrap();

            let parent = parse_attribute(child_of)
                .expect("Expected #[child_of] attribute with a valid path");

            quote! {
                f(Box::new(#name::#parent), Box::new(#name::#variant_name));
            }
        });

        let iter_al_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;

            quote! {
                f(Box::new(#name::#variant_name));
            }
        });

        let expanded = quote! {
            impl #name {
                pub fn to_string(&self) -> String {
                    match self {
                        #(#to_string_arms)*
                    }
                }
            }

            impl RoleHierarchy for #name {
                fn iter_hierarchy(&self, f: &mut dyn FnMut(Box<dyn RoleHierarchy>, Box<dyn RoleHierarchy>)) {
                    #(#child_of_arms)*
                }

                fn iter_all(&self, f: &mut dyn FnMut(Box<dyn RoleHierarchy>)) {
                    #(#iter_al_arms)*
                }

                // fn as_any(&self) -> &dyn Any {
                //     self
                // }
            }

            impl ToNode for #name {
                fn to_node(&self, group_id: Option<String>) -> Node {
                    match self {
                        #(#to_node_arms)*
                    }
                }
            }
        };

        TokenStream::from(expanded)
    } else {
        panic!("Role derive macro only works with enums");
    }
}

// `#[namespace(AnotherNamespaces::User)]` 형태의 경로를 파싱
fn parse_attribute(attr: &Attribute) -> Option<Path> {
    attr.parse_args::<Path>().ok()
}
