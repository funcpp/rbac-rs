mod examples;

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    mod community {
        use rbac::ACNamespace;

        use crate::examples::community::*;

        #[test]
        fn schema() {
            let ns = Namespace::Post;
            let roles = ns.get_roles();
            assert_eq!(roles.len(), 4);
        }

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

            let result = server
                .add_relationship(UESR_ALICE, UserToPost::Writer, POST_BY_ALICE)
                .unwrap();
            assert_eq!(result, true);
            let allowed = server
                .allowed(UESR_ALICE, UserToPost::Writer, POST_BY_ALICE)
                .unwrap();

            assert_eq!(allowed, true);
        }

        #[test]
        fn community_role_inherits() {
            let server = rbac::RBAC::new("./rocksdb/test2");
            server.clear(true).unwrap();

            // alice is a writer
            server
                .add_relationship(UESR_ALICE, UserToPost::Writer, POST_BY_ALICE)
                .unwrap();

            // role inherits automatically
            let result = server.allowed(UESR_ALICE, UserToPost::Viewer, POST_BY_ALICE);
            // alice is a writer and it means also a viewer.
            assert_eq!(result.unwrap(), true);

            server
                .add_relationship(USER_BOB, UserToPost::Viewer, POST_BY_ALICE)
                .unwrap();

            let result = server.allowed(USER_CHARLIE, UserToPost::Viewer, POST_BY_ALICE);
            // charlie is not a node yet... so it should return an error
            assert_eq!(result.is_err(), true);

            // charlie write a post
            server
                .add_relationship(USER_CHARLIE, UserToPost::Writer, POST_BY_CHARLIE)
                .unwrap();

            // now charlie is a node
            let result = server.allowed(USER_CHARLIE, UserToPost::Viewer, POST_BY_ALICE);
            assert_eq!(result.is_ok(), true);
            // but can't view alice's post
            assert_eq!(result.unwrap(), false);

            // charlie and bob joins the group foo
            server
                .add_relationship(USER_BOB, UserToGroup::Member, GROUP_FOO)
                .unwrap();

            server
                .add_relationship(USER_CHARLIE, UserToGroup::Member, GROUP_FOO)
                .unwrap();

            // group foo writes a post
            server
                .add_relationship(GROUP_FOO, GroupToPost::Writer, POST_BY_FOO)
                .unwrap();

            // charlie can view the post by foo
            // let result = server.allowed(USER_CHARLIE, UserToPost::Viewer, POST_BY_FOO);

            // assert_eq!(result.unwrap(), true);
        }
    }
}
