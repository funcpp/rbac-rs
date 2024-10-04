mod examples;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    mod community {
        use rbac::EntityRelationship;

        use crate::examples::community::*;

        const UESR_ALICE: User = User {
            id: 1,
            nickname: "Alice",
        };

        const USER_BOB: User = User {
            id: 2,
            nickname: "Bob",
        };

        const USER_CHARLIE: User = User {
            id: 3,
            nickname: "Charlie",
        };

        const POST_BY_ALICE: Post = Post {
            id: 1,
            author_id: UESR_ALICE.id,
            title: "Hello, World!",
        };

        const POST_BY_CHARLIE: Post = Post {
            id: 2,
            author_id: USER_CHARLIE.id,
            title: "Hello, World!",
        };

        const GROUP_FOO: Group = Group { id: 1, name: "Foo" };

        const POST_BY_FOO: Post = Post {
            id: 3,
            author_id: GROUP_FOO.id,
            title: "Hello, World!",
        };

        #[test]
        fn community_basic() {
            let server = rbac::RBAC::new("./rocksdb/test1");
            server.clear(true).unwrap();

            let r = EntityRelationship::new(&UESR_ALICE, &PostRoles::Writer, &POST_BY_ALICE);

            server.add_relationship(&r).unwrap();
            let allowed = server.allowed(&r).unwrap();

            assert_eq!(allowed, true);
        }

        #[test]
        fn community_role_inherits() {
            let server = rbac::RBAC::new("./rocksdb/test2");
            server.clear(true).unwrap();

            // alice is a writer
            let r = EntityRelationship::new(&UESR_ALICE, &PostRoles::Writer, &POST_BY_ALICE);
            server.add_relationship(&r).unwrap();

            // role inherits automatically
            let test = EntityRelationship::new(&UESR_ALICE, &PostRoles::Viewer, &POST_BY_ALICE);
            let result = server.allowed(&test);
            // alice is a writer and it means also a viewer.
            assert_eq!(result.unwrap(), true);

            let r = EntityRelationship::new(&USER_BOB, &PostRoles::Viewer, &POST_BY_ALICE);
            server.add_relationship(&r).unwrap();

            let test = EntityRelationship::new(&USER_CHARLIE, &PostRoles::Viewer, &POST_BY_ALICE);
            let result = server.allowed(&test);
            // charlie is not a node yet... so it should return an error
            assert_eq!(result.is_err(), true);

            // charlie write a post
            let r = EntityRelationship::new(&USER_CHARLIE, &PostRoles::Writer, &POST_BY_CHARLIE);
            server.add_relationship(&r).unwrap();

            // now charlie is a node
            let result = server.allowed(&test);
            assert_eq!(result.is_ok(), true);
            // but can't view alice's post
            assert_eq!(result.unwrap(), false);

            // charlie and bob joins the group foo
            let r = EntityRelationship::new(&USER_BOB, &GroupRoles::Member, &GROUP_FOO);
            server.add_relationship(&r).unwrap();

            let r = EntityRelationship::new(&USER_CHARLIE, &GroupRoles::Member, &GROUP_FOO);
            server.add_relationship(&r).unwrap();

            // group foo writes a post
            let r = EntityRelationship::new(&GROUP_FOO, &PostRoles::Writer, &POST_BY_FOO);
            server.add_relationship(&r).unwrap();

            // charlie can view the post by foo
            let test = EntityRelationship::new(&USER_CHARLIE, &PostRoles::Viewer, &POST_BY_FOO);
            let result = server.allowed(&test);

            assert_eq!(result.unwrap(), true);
        }
    }
}
