use std::collections::VecDeque;

use indradb::{self, QueryExt};

pub trait NamespaceToString {
    fn to_string(&self) -> String;
}

pub trait NamespaceRole {
    fn get_roles(&self) -> Option<Box<dyn RoleHierarchy>>;
}

pub trait NamespaceToStringAndRole: NamespaceToString + NamespaceRole {}

// Node can be Entity or Role
pub struct Node {
    namespace: Box<dyn NamespaceToStringAndRole>,
    id: String,
}

impl Node {
    pub fn new(namespace: Box<dyn NamespaceToStringAndRole>, id: String) -> Self {
        Node { namespace, id }
    }

    pub fn to_string(&self) -> String {
        format!("{}_{}", self.namespace.to_string(), self.id)
    }

    pub fn to_identifier(&self) -> indradb::Identifier {
        indradb::Identifier::new(self.to_string()).unwrap()
    }

    pub fn to_vertex(&self) -> indradb::Vertex {
        indradb::Vertex::new(self.to_identifier())
    }
}

pub trait ToNode {
    fn to_node(&self, parent_id: Option<String>) -> Node;
}

pub trait RoleHierarchy: ToNode {
    fn iter_hierarchy(&self, f: &mut dyn FnMut(Box<dyn RoleHierarchy>, Box<dyn RoleHierarchy>));
    fn iter_all(&self, f: &mut dyn FnMut(Box<dyn RoleHierarchy>));
    //fn as_any(&self) -> &dyn Any;
}

//

pub struct EntityRelationship {
    subject: Node, // user          user            group
    role: Node,    // is a writer   is a member     is a viewer
    object: Node,  // of post       of group        of post
}

impl EntityRelationship {
    pub fn new(subject: &dyn ToNode, role: &dyn ToNode, object: &dyn ToNode) -> Self {
        let object = object.to_node(None);
        EntityRelationship {
            subject: subject.to_node(None),
            role: role.to_node(Some(object.id.clone())),
            object,
        }
    }

    pub fn new_from_node(subject: Node, role: Node, object: Node) -> Self {
        EntityRelationship {
            subject,
            role,
            object,
        }
    }
}

pub struct RoleRelationship {
    parent: Node,
    child: Node,
}

impl RoleRelationship {
    pub fn new(parent: &dyn ToNode, child: &dyn ToNode) -> Self {
        RoleRelationship {
            parent: parent.to_node(None),
            child: child.to_node(None),
        }
    }

    pub fn new_from_node(parent: Node, child: Node) -> Self {
        RoleRelationship { parent, child }
    }
}

//

pub struct RBAC {
    pub db: indradb::Database<indradb::RocksdbDatastore>,
}

#[derive(Debug)]
pub enum RBACError {
    IndradbError(indradb::Error),
    VertexNotFound,
    VertexDuplication,
}

impl From<indradb::Error> for RBACError {
    fn from(e: indradb::Error) -> Self {
        RBACError::IndradbError(e)
    }
}

impl RBAC {
    pub fn new(db_path: &str) -> Self {
        let db = indradb::RocksdbDatastore::new_db(db_path).unwrap();
        db.index_property(indradb::Identifier::new("entity").unwrap())
            .unwrap();
        RBAC { db }
    }

    pub fn get_all_vertices(&self) -> Result<Vec<indradb::Vertex>, RBACError> {
        let output = self.db.get(indradb::AllVertexQuery)?;
        let vertices = match indradb::util::extract_vertices(output) {
            Some(vs) => vs,
            None => return Ok(Vec::new()),
        };
        Ok(vertices)
    }

    pub fn get_all_edges(&self) -> Result<Vec<indradb::Edge>, RBACError> {
        let output = self.db.get(indradb::AllEdgeQuery)?;
        let edges = match indradb::util::extract_edges(output) {
            Some(es) => es,
            None => return Ok(Vec::new()),
        };
        Ok(edges)
    }

    pub fn clear(&self, really: bool) -> Result<(), RBACError> {
        if really {
            self.db.delete(indradb::AllVertexQuery)?;
            self.db.delete(indradb::AllEdgeQuery)?;
        }
        Ok(())
    }

    fn get_or_create_vertex(&self, node: &Node) -> Result<(indradb::Vertex, bool), RBACError> {
        let entity_identifier = indradb::Identifier::new("entity").unwrap();
        let entity_value = indradb::Json::new(serde_json::Value::String(node.to_string()));

        let q = indradb::VertexWithPropertyValueQuery::new(
            entity_identifier.clone(),
            entity_value.clone(),
        );
        let output = self.db.get(q)?;

        let vertices = match indradb::util::extract_vertices(output) {
            Some(vs) => vs,
            None => return Err(RBACError::VertexNotFound),
        };

        if vertices.is_empty() {
            let v = node.to_vertex();
            self.db.create_vertex(&v)?;
            self.db.set_properties(
                indradb::SpecificVertexQuery::single(v.id.clone()),
                entity_identifier,
                &entity_value,
            )?;

            return Ok((v, false));
        }

        if vertices.len() > 1 {
            return Err(RBACError::VertexDuplication);
        }

        Ok((vertices[0].clone(), true))
    }

    fn get_vertex(&self, node: &Node) -> Result<indradb::Vertex, RBACError> {
        let q = indradb::VertexWithPropertyValueQuery::new(
            indradb::Identifier::new("entity").unwrap(),
            indradb::Json::new(serde_json::Value::String(node.to_string())),
        );
        let output = self.db.get(q)?;

        let vertices = match indradb::util::extract_vertices(output) {
            Some(vs) => vs,
            None => return Err(RBACError::VertexNotFound),
        };

        if vertices.is_empty() {
            return Err(RBACError::VertexNotFound);
        }

        if vertices.len() > 1 {
            return Err(RBACError::VertexDuplication);
        }

        Ok(vertices[0].clone())
    }

    pub fn add_role_relationship(
        &self,
        relationship: &RoleRelationship,
    ) -> Result<bool, RBACError> {
        let (parent_v, _) = self.get_or_create_vertex(&relationship.parent)?;
        let (child_v, _) = self.get_or_create_vertex(&relationship.child)?;

        let e = indradb::Edge::new(
            parent_v.id,
            indradb::Identifier::new("inherits").unwrap(),
            child_v.id,
        );

        self.db.create_edge(&e)?;

        Ok(true)
    }

    pub fn add_relationship(&self, relationship: &EntityRelationship) -> Result<bool, RBACError> {
        let (subject_v, _) = self.get_or_create_vertex(&relationship.subject)?;
        let (role_v, _) = self.get_or_create_vertex(&relationship.role)?;
        let (object_v, was_object_exist) = self.get_or_create_vertex(&relationship.object)?;

        // newly created object vertex
        if !was_object_exist {
            // handle role's hierarchy
            let roles = relationship.object.namespace.get_roles();
            if let Some(roles) = roles {
                roles.iter_hierarchy(&mut |parent, child| {
                    let parent_node = parent.to_node(Some(relationship.object.id.clone()));
                    let child_node = child.to_node(Some(relationship.object.id.clone()));
                    self.add_role_relationship(&RoleRelationship::new_from_node(
                        parent_node,
                        child_node,
                    ))
                    .unwrap();
                });

                roles.iter_all(&mut |role| {
                    let role_node = role.to_node(Some(relationship.object.id.clone()));
                    let (role_v, _) = self.get_or_create_vertex(&role_node).unwrap();
                    let role_e = indradb::Edge::new(
                        role_v.id,
                        indradb::Identifier::new("role_to_entity").unwrap(),
                        object_v.id,
                    );
                    self.db.create_edge(&role_e).unwrap();
                });
            }
        }

        let subject_e = indradb::Edge::new(
            subject_v.id,
            indradb::Identifier::new("entity_to_role").unwrap(),
            role_v.id,
        );

        self.db.create_edge(&subject_e)?;

        Ok(true)
    }

    // pub fn vertex_count(&self) -> Result<usize, RBACError> {
    //     let output = self.db.get(indradb::AllVertexQuery)?;
    //     // let vertices = match indradb::util::extract_vertices(output) {
    //     //     Some(vs) => vs,
    //     //     None => return Ok(0),
    //     // };
    //     Ok(output.len())
    // }

    pub fn allowed(&self, target: &EntityRelationship) -> Result<bool, RBACError> {
        let subject_v = self.get_vertex(&target.subject)?;
        let role_v = self.get_vertex(&target.role)?;
        let object_v = self.get_vertex(&target.object)?;

        let mut queue = VecDeque::new();
        queue.push_back(subject_v.id.clone());

        let mut visited = Vec::new();
        visited.push(subject_v.id.clone());

        while !queue.is_empty() {
            let v = queue.pop_front().unwrap();

            let q = indradb::SpecificVertexQuery::single(v).outbound().unwrap();
            let output = self.db.get(q)?;

            println!("cur: {:?}", v);
            for o in output.clone() {
                println!("outbound: {:?}", o);
            }

            let vertices = indradb::util::extract_vertices(output.clone());

            if let Some(vertices) = vertices {
                for vertex in vertices {
                    if vertex.id == object_v.id {
                        //return Ok(true);
                    } else {
                        if !visited.contains(&vertex.id) {
                            visited.push(vertex.id.clone());
                            queue.push_back(vertex.id.clone());
                        }
                    }
                }
            }

            let edges = indradb::util::extract_edges(output);

            if let Some(edges) = edges {
                for edge in edges {
                    if edge.inbound_id == object_v.id {
                        if edge.outbound_id == role_v.id {
                            return Ok(true);
                        }
                    } else {
                        if !visited.contains(&edge.inbound_id) {
                            visited.push(edge.inbound_id.clone());
                            queue.push_back(edge.inbound_id.clone());
                        }
                    }
                }
            }
        }

        Ok(false)
    }
}
