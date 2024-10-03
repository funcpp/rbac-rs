use rbac::{
    NamespaceRole, NamespaceToString, NamespaceToStringAndRole, Node, RoleHierarchy, ToNode,
};
use rbac_macro::{Namespace, Role, ToNode};

// namespaces
#[derive(Namespace)]
pub enum Namespaces {
    User(Option<String>),
    #[roles(PostRoles)]
    Post(Option<String>),
    #[roles(GroupRoles)]
    Group(Option<String>),
}

// roles

#[derive(Role, Default)]
#[namespace(Namespaces::Group)]
pub enum GroupRoles {
    Admin,
    #[child_of(Admin)]
    #[default]
    Member,
}

#[derive(Role, Default)]
#[namespace(Namespaces::Post)]
pub enum PostRoles {
    Writer,
    #[child_of(Writer)]
    #[default]
    Viewer,
}

// entities

#[allow(dead_code)]
#[derive(ToNode)]
#[namespace(Namespaces::User)]
pub struct User {
    pub id: i32,
    pub nickname: &'static str,
}

#[allow(dead_code)]
#[derive(ToNode)]
#[namespace(Namespaces::Post)]
pub struct Post {
    pub id: i32,
    pub author_id: i32,
    pub title: &'static str,
}

#[allow(dead_code)]
#[derive(ToNode)]
#[namespace(Namespaces::Group)]
pub struct Group {
    pub id: i32,
    pub name: &'static str,
}
