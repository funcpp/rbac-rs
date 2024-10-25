extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Path};

#[proc_macro_derive(Namespace, attributes(namespace_role))]
pub fn derive_define_namespace(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = if let Data::Enum(namespaces) = input.data {
        namespaces.variants
    } else {
        panic!("Namespace can only be derived for enums");
    };

    /*
    pub enum Namespace {
        #[namespace_role(UserToPost::Writer, UserToPost::Viewer)]
        #[namespace_role(UserToGroup::Admin, UserToGroup::Member)]
        User,
        Post,
        #[namespace_role(GroupToPost::Writer, GroupToPost::Viewer)]
        Group,
    }

    impl ACNamespace for Namespace {
        fn get_id(&self) -> i32 {
            match self {
                Namespace::User => 1,
                Namespace::Club => 2,
                Namespace::ClubTeam => 3,
            }
        }

        fn to_string(&self) -> String {
            match self {
                Namespace::User => "User".to_string(),
                Namespace::Club => "Club".to_string(),
                Namespace::ClubTeam => "ClubTeam".to_string(),
            }
        }

        fn get_roles(&self) -> Vec<Box<dyn ACRole>> {
            match self {
                Namespace::User => vec![
                    Box::new(UserToPost::Writer),
                    Box::new(UserToPost::Viewer),
                    Box::new(UserToGroup::Admin),
                    Box::new(UserToGroup::Member),
                ],
                Namespace::Post => vec![],
                Namespace::Group => vec![],
            }
        }
    }
    */

    let get_id_arms = variants.iter().enumerate().map(|(i, variant)| {
        let variant_name = &variant.ident;
        quote! {
            #name::#variant_name => #i as i32,
        }
    });

    let to_string_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();
        quote! {
            #name::#variant_name => #variant_str.to_string(),
        }
    });

    let get_roles_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let role_attrs = find_all_attrs(&variant.attrs, "namespace_role");
        let role_attrs = role_attrs.iter().map(|role| {
            quote! {
                Box::new(#role),
            }
        });

        quote! {
            #name::#variant_name => vec![#(#role_attrs)*],
        }
    });

    let gen = quote! {
        impl ACNamespace for #name {
            fn get_id(&self) -> i32 {
                match self {
                    #(#get_id_arms)*
                }
            }

            fn to_string(&self) -> String {
                match self {
                    #(#to_string_arms)*
                }
            }

            fn get_roles(&self) -> Vec<Box<dyn ACRole>> {
                match self {
                    #(#get_roles_arms)*
                }
            }
        }
    };

    gen.into()
}

#[proc_macro_derive(Role, attributes(role_subject, role_object, subrole_of))]
pub fn derive_role(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    /*
    impl ACRole for UserToClub {
        fn get_object_namespace_id() -> i32 {
            Namespace::User.get_id()
        }
        fn get_subject_namespace_id() -> i32 {
            Namespace::Club.get_id()
        }

        fn get_super_roles(&self) -> Vec<Box<dyn ACRole>> {
            match self {
                UserToClub::Master => vec![],
                UserToClub::SemiMaster => vec![Box::new(UserToClub::Master), Box::new(UserToClub::SuperMaster)],
                UserToClub::Member => vec![Box::new(UserToClub::SemiMaster)],
            }
        }

        fn as_i32(&self) -> i32 {
            match self {
                UserToClub::Master => 0,
                UserToClub::SemiMaster => 1,
                UserToClub::Member => 2,
            }
        }

        fn to_string(&self) -> String {
            match self {
                UserToClub::Master => "Master".to_string(),
                UserToClub::SemiMaster => "SemiMaster".to_string(),
                UserToClub::Member => "Member".to_string(),
            }
        }
    }
    */

    let subject_namespace_path = find_single_attr(&input.attrs, "role_subject")
        .expect("Expected #[role_subject] attribute with a valid path");

    let object_namespace_path = find_single_attr(&input.attrs, "role_object")
        .expect("Expected #[role_object] attribute with a valid path");

    if let Data::Enum(data_enum) = input.data {
        if data_enum.variants.len() == 0 {
            return TokenStream::new();
        }

        let get_subset_ofs_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let subset_ofs = find_all_attrs(&variant.attrs, "subrole_of");

            if subset_ofs.len() == 0 {
                return quote! {
                    #name::#variant_name => vec![],
                };
            }

            let subset_ofs = subset_ofs.iter().map(|subset_of| {
                quote! {
                    Box::new(#name::#subset_of),
                }
            });

            quote! {
                #name::#variant_name => vec![#(#subset_ofs)*],
            }
        });

        let to_string_arms = data_enum.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let variant_str = variant_name.to_string();
            quote! {
                #name::#variant_name => #variant_str.to_string(),
            }
        });

        let as_i32_arms = data_enum.variants.iter().enumerate().map(|(i, variant)| {
            let variant_name = &variant.ident;
            quote! {
                #name::#variant_name => #i as i32,
            }
        });

        let expanded = quote! {
            impl ACRole for #name {
                fn to_string(&self) -> String {
                    match self {
                        #(#to_string_arms)*
                    }
                }

                fn get_subject_namespace_id(&self) -> i32 {
                    #subject_namespace_path.get_id()
                }

                fn get_object_namespace_id(&self) -> i32 {
                    #object_namespace_path.get_id()
                }

                fn get_subject_namespace(&self) -> Box::<dyn ACNamespace> {
                    Box::new(#subject_namespace_path)
                }

                fn get_object_namespace(&self) -> Box::<dyn ACNamespace> {
                    Box::new(#object_namespace_path)
                }

                fn get_super_roles(&self) -> Vec<Box<dyn ACRole>> {
                    match self {
                        #(#get_subset_ofs_arms)*
                    }
                }

                fn as_i32(&self) -> i32 {
                    match self {
                        #(#as_i32_arms)*
                    }
                }
            }
        };

        TokenStream::from(expanded)
    } else {
        panic!("Role derive macro only works with enums");
    }
}

fn find_single_attr(attrs: &Vec<Attribute>, ident: &str) -> Option<Path> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident(ident))
        .and_then(parse_attribute)
}

fn find_all_attrs(attrs: &Vec<Attribute>, ident: &str) -> Vec<Path> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident(ident))
        .filter_map(parse_attribute)
        .collect()
}

fn parse_attribute(attr: &Attribute) -> Option<Path> {
    attr.parse_args::<Path>().ok()
}
