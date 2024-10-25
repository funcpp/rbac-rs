use std::{collections::VecDeque, fmt::Debug};

use indradb::{self, QueryExt};

pub trait ACRole: ACRoleClone + Debug {
    fn get_object_namespace_id(&self) -> i32;
    fn get_subject_namespace_id(&self) -> i32;

    fn get_object_namespace(&self) -> Box<dyn ACNamespace>;
    fn get_subject_namespace(&self) -> Box<dyn ACNamespace>;

    fn as_i32(&self) -> i32;

    fn get_id(&self) -> i32 {
        self.get_object_namespace_id() * 1e6 as i32
            + self.get_subject_namespace_id() * 1e3 as i32
            + self.as_i32()
    }

    fn get_super_roles(&self) -> Vec<Box<dyn ACRole>>;

    fn to_string(&self) -> String;
}

pub trait ACRoleClone {
    fn clone_box(&self) -> Box<dyn ACRole>;
}

impl<T> ACRoleClone for T
where
    T: 'static + ACRole + Clone,
{
    fn clone_box(&self) -> Box<dyn ACRole> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ACRole> {
    fn clone(&self) -> Box<dyn ACRole> {
        self.clone_box()
    }
}

pub trait ACNamespace: ACNamespaceClone + Debug {
    fn get_id(&self) -> i32;
    /// roles to object entity (role -> entity)
    fn get_roles(&self) -> Vec<Box<dyn ACRole>>;
    fn to_string(&self) -> String;
}

pub trait ACNamespaceClone {
    fn clone_box(&self) -> Box<dyn ACNamespace>;
}

impl<T> ACNamespaceClone for T
where
    T: 'static + ACNamespace + Clone,
{
    fn clone_box(&self) -> Box<dyn ACNamespace> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ACNamespace> {
    fn clone(&self) -> Box<dyn ACNamespace> {
        self.clone_box()
    }
}

pub struct ACNode {
    t: String, // namespace or role
    id: String,
    tag: String,
}

impl ACNode {
    pub fn new(t: String, id: String, tag: String) -> Self {
        ACNode { t, id, tag }
    }

    pub fn to_string(&self) -> String {
        format!("{}_{}", self.t, self.id)
    }

    pub fn to_identifier(&self) -> indradb::Identifier {
        indradb::Identifier::new(self.to_string()).unwrap()
    }

    pub fn to_vertex(&self) -> indradb::Vertex {
        indradb::Vertex::new(self.to_identifier())
    }
}

#[derive(Clone)]
pub struct ACRoleNode {
    pub role: Box<dyn ACRole>,
    pub object_id: String,
}

impl ACRoleNode {
    pub fn new<S: ACRole + 'static, T: ToString>(role: S, object_id: T) -> Self {
        ACRoleNode {
            role: Box::new(role),
            object_id: object_id.to_string(),
        }
    }
}

impl Into<ACNode> for ACRoleNode {
    fn into(self) -> ACNode {
        let t = format!("{}-{}", self.role.get_object_namespace_id(), self.object_id);
        let id = self.role.get_id().to_string();
        let tag = format!(
            "{}({})_{}{}",
            self.role.get_object_namespace().to_string(),
            self.object_id,
            self.role.get_subject_namespace().to_string(),
            self.role.to_string(),
        );
        ACNode::new(t, id, tag)
    }
}

impl Into<ACNode> for &ACRoleNode {
    fn into(self) -> ACNode {
        let t = format!("{}-{}", self.role.get_object_namespace_id(), self.object_id);
        let id = self.role.get_id().to_string();
        let tag = format!(
            "{}({})_{}{}",
            self.role.get_object_namespace().to_string(),
            self.object_id,
            self.role.get_subject_namespace().to_string(),
            self.role.to_string(),
        );
        ACNode::new(t, id, tag)
    }
}

#[derive(Clone)]
pub struct ACEntityNode {
    pub namespace: Box<dyn ACNamespace>,
    pub id: String,
}

impl ACEntityNode {
    pub fn new<S: ACNamespace + 'static, T: ToString>(namespace: S, id: T) -> Self {
        ACEntityNode {
            namespace: Box::new(namespace),
            id: id.to_string(),
        }
    }
}

impl Into<ACNode> for ACEntityNode {
    fn into(self) -> ACNode {
        let t = self.namespace.get_id().to_string();
        let id = self.id.clone();
        let tag = format!("{}({})", self.namespace.to_string(), id);
        ACNode::new(t, id, tag)
    }
}

impl Into<ACNode> for &ACEntityNode {
    fn into(self) -> ACNode {
        let t = self.namespace.get_id().to_string();
        let id = self.id.clone();
        let tag = format!("{}({})", self.namespace.to_string(), id);
        ACNode::new(t, id, tag)
    }
}

pub struct RBAC {
    pub db: indradb::Database<indradb::RocksdbDatastore>,
}

#[derive(Debug)]
pub enum RBACError {
    IndradbError(indradb::Error),
    VertexNotFound,
    VertexDuplication,
    InvalidInput,
}

impl From<indradb::Error> for RBACError {
    fn from(e: indradb::Error) -> Self {
        RBACError::IndradbError(e)
    }
}

pub struct VertexWithTag {
    pub vertex: indradb::Vertex,
    pub tag: String,
}

impl RBAC {
    pub fn new(db_path: &str) -> Self {
        let db = indradb::RocksdbDatastore::new_db(db_path).unwrap();
        db.index_property(indradb::Identifier::new("node").unwrap())
            .unwrap();
        db.index_property(indradb::Identifier::new("tag").unwrap())
            .unwrap();
        RBAC { db }
    }

    pub fn get_all_vertices(&self) -> Result<Vec<VertexWithTag>, RBACError> {
        let tag_ident = indradb::Identifier::new("tag").unwrap();
        let output = self.db.get(indradb::AllVertexQuery.properties().unwrap())?;
        let vertices = match indradb::util::extract_vertex_properties(output) {
            Some(vs) => vs,
            None => return Ok(Vec::new()),
        };
        let vertices = vertices
            .iter()
            .map(|v| VertexWithTag {
                vertex: v.vertex.clone(),
                tag: v
                    .props
                    .iter()
                    .find(|p| p.name == tag_ident)
                    .unwrap()
                    .value
                    .as_str()
                    .unwrap()
                    .to_string(),
            })
            .collect();
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

    fn get_or_create_vertex(&self, node: ACNode) -> Result<(indradb::Vertex, bool), RBACError> {
        let entity_identifier = indradb::Identifier::new("node").unwrap();
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
            let q = indradb::SpecificVertexQuery::single(v.id.clone());
            self.db
                .set_properties(q.clone(), entity_identifier, &entity_value)?;

            let tag_ident = indradb::Identifier::new("tag").unwrap();
            let tag_value = indradb::Json::new(serde_json::Value::String(node.tag));
            self.db.set_properties(q, tag_ident, &tag_value).unwrap();

            return Ok((v, false));
        }

        if vertices.len() > 1 {
            return Err(RBACError::VertexDuplication);
        }

        Ok((vertices[0].clone(), true))
    }

    fn get_vertex(&self, node: ACNode) -> Result<indradb::Vertex, RBACError> {
        let q = indradb::VertexWithPropertyValueQuery::new(
            indradb::Identifier::new("node").unwrap(),
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
        parent: &ACRoleNode,
        child: &ACRoleNode,
    ) -> Result<bool, RBACError> {
        let (parent_v, _) = self.get_or_create_vertex(parent.into())?;
        let (child_v, _) = self.get_or_create_vertex(child.into())?;

        let e = indradb::Edge::new(
            parent_v.id,
            indradb::Identifier::new("inherits").unwrap(),
            child_v.id,
        );

        self.db.create_edge(&e)?;

        Ok(true)
    }

    pub fn add_relationship(
        &self,
        subject: impl Into<ACEntityNode>,
        role: impl ACRole + 'static,
        object: impl Into<ACEntityNode>,
    ) -> Result<bool, RBACError> {
        let object: ACEntityNode = object.into();
        let subject: ACEntityNode = subject.into();

        if role.get_object_namespace().get_id() != object.namespace.get_id()
            || role.get_subject_namespace().get_id() != subject.namespace.get_id()
        {
            return Err(RBACError::InvalidInput);
        }

        let role = ACRoleNode::new(role, object.id.clone());

        let (subject_v, _) = self.get_or_create_vertex(subject.clone().into())?;
        let (role_v, _) = self.get_or_create_vertex(role.into())?;
        let (object_v, was_object_exist) = self.get_or_create_vertex(object.clone().into())?;

        // newly created object vertex
        if !was_object_exist {
            // handle role's hierarchy
            let roles = object.namespace.get_roles();
            for role in roles {
                if role.get_subject_namespace().get_id() != subject.namespace.get_id() {
                    continue;
                }

                let role_node = ACRoleNode {
                    role: role.clone(),
                    object_id: object.id.clone(),
                };
                let (role_v, _) = self.get_or_create_vertex((&role_node).into())?;
                let role_e = indradb::Edge::new(
                    role_v.id,
                    indradb::Identifier::new("role_to_entity").unwrap(),
                    object_v.id,
                );

                self.db.create_edge(&role_e)?;

                for super_role in role.get_super_roles() {
                    let super_role_node = ACRoleNode {
                        role: super_role,
                        object_id: object.id.clone(),
                    };
                    self.add_role_relationship(&super_role_node.into(), (&role_node).into())?;
                }
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

    // subject -- ... -> role -> object
    pub fn allowed<R: ACRole + 'static>(
        &self,
        subject: impl Into<ACEntityNode>,
        role: R,
        object: impl Into<ACEntityNode>,
    ) -> Result<bool, RBACError> {
        let subject: ACEntityNode = subject.into();
        let object: ACEntityNode = object.into();

        let role = ACRoleNode::new(role, object.id.clone());

        let subject_v = self.get_vertex(subject.into())?;
        let role_v = self.get_vertex(role.into())?;
        let object_v = self.get_vertex(object.into())?;

        let mut queue = VecDeque::new();
        queue.push_back(subject_v.id.clone());

        let mut visited = Vec::new();
        visited.push(subject_v.id.clone());

        let mut role_checked = false;

        while !queue.is_empty() {
            let v = queue.pop_front().unwrap();

            let q = indradb::SpecificVertexQuery::single(v).outbound().unwrap();
            let output = self.db.get(q)?;

            // println!("cur: {:?}", v);
            // for o in output.clone() {
            //     println!("outbound: {:?}", o);
            // }

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
                    if edge.inbound_id == role_v.id {
                        role_checked = true;
                    }

                    if edge.inbound_id == object_v.id {
                        if role_checked {
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
