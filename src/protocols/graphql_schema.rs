use crate::error::AppError;
use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_null_default_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

pub const INTROSPECTION_QUERY: &str = r"
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      ...FullType
    }
    directives {
      name
      description
      locations
      args {
        ...InputValue
      }
    }
  }
}

fragment FullType on __Type {
  kind
  name
  description
  fields(includeDeprecated: true) {
    name
    description
    args {
      ...InputValue
    }
    type {
      ...TypeRef
    }
    isDeprecated
    deprecationReason
  }
  inputFields {
    ...InputValue
  }
  interfaces {
    ...TypeRef
  }
  enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
  }
  possibleTypes {
    ...TypeRef
  }
}

fragment InputValue on __InputValue {
  name
  description
  type {
    ...TypeRef
  }
  defaultValue
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
            }
          }
        }
      }
    }
  }
}
";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionResponse {
    pub data: Option<IntrospectionData>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_null_default_vec")]
    pub errors: Vec<IntrospectionError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionData {
    #[serde(rename = "__schema")]
    pub schema: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "queryType")]
    pub query_type: Option<TypeRef>,
    #[serde(rename = "mutationType")]
    pub mutation_type: Option<TypeRef>,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<TypeRef>,
    pub types: Vec<FullType>,
    pub directives: Vec<Directive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRef {
    pub kind: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "ofType")]
    pub of_type: Option<Box<TypeRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullType {
    pub kind: String,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Option<Vec<Field>>,
    #[serde(rename = "inputFields", default)]
    pub input_fields: Option<Vec<InputValue>>,
    #[serde(default)]
    pub interfaces: Option<Vec<TypeRef>>,
    #[serde(rename = "enumValues", default)]
    pub enum_values: Option<Vec<EnumValue>>,
    #[serde(rename = "possibleTypes", default)]
    pub possible_types: Option<Vec<TypeRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub args: Vec<InputValue>,
    #[serde(rename = "type")]
    pub field_type: TypeRef,
    #[serde(rename = "isDeprecated", default)]
    pub is_deprecated: bool,
    #[serde(rename = "deprecationReason", default)]
    pub deprecation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValue {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub input_type: TypeRef,
    #[serde(rename = "defaultValue", default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "isDeprecated", default)]
    pub is_deprecated: bool,
    #[serde(rename = "deprecationReason", default)]
    pub deprecation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directive {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<String>,
    #[serde(default)]
    pub args: Vec<InputValue>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLSchema {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: Vec<SchemaType>,
}

#[derive(Debug, Clone)]
pub struct SchemaType {
    pub kind: TypeKind,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<SchemaField>,
    pub input_fields: Vec<SchemaInputField>,
    pub enum_values: Vec<SchemaEnumValue>,
    pub interfaces: Vec<String>,
    #[allow(dead_code)]
    pub possible_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
    NonNull,
    List,
}

impl TypeKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "SCALAR" => Self::Scalar,
            "OBJECT" => Self::Object,
            "INTERFACE" => Self::Interface,
            "UNION" => Self::Union,
            "ENUM" => Self::Enum,
            "INPUT_OBJECT" => Self::InputObject,
            "NON_NULL" => Self::NonNull,
            "LIST" => Self::List,
            _ => Self::Scalar,
        }
    }
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar => write!(f, "Scalar"),
            Self::Object => write!(f, "Object"),
            Self::Interface => write!(f, "Interface"),
            Self::Union => write!(f, "Union"),
            Self::Enum => write!(f, "Enum"),
            Self::InputObject => write!(f, "Input Object"),
            Self::NonNull => write!(f, "NonNull"),
            Self::List => write!(f, "List"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaField {
    pub name: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub args: Vec<SchemaArg>,
    pub return_type: String,
    pub is_deprecated: bool,
    #[allow(dead_code)]
    pub deprecation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaArg {
    pub name: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub arg_type: String,
    #[allow(dead_code)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaInputField {
    pub name: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub field_type: String,
    #[allow(dead_code)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaEnumValue {
    pub name: String,
    pub description: Option<String>,
    pub is_deprecated: bool,
}

pub fn parse_introspection_response(
    response: &IntrospectionResponse,
) -> Result<GraphQLSchema, AppError> {
    if !response.errors.is_empty() {
        let msgs: Vec<&str> = response.errors.iter().map(|e| e.message.as_str()).collect();
        return Err(AppError::Validation(format!(
            "Introspection errors: {}",
            msgs.join(", ")
        )));
    }

    let data = response
        .data
        .as_ref()
        .ok_or_else(|| AppError::Validation("No data in introspection response".to_string()))?;

    let mut schema = GraphQLSchema {
        query_type: data.schema.query_type.as_ref().and_then(|t| t.name.clone()),
        mutation_type: data
            .schema
            .mutation_type
            .as_ref()
            .and_then(|t| t.name.clone()),
        subscription_type: data
            .schema
            .subscription_type
            .as_ref()
            .and_then(|t| t.name.clone()),
        types: Vec::new(),
    };

    for full_type in &data.schema.types {
        let name = match &full_type.name {
            Some(n) => n.clone(),
            None => continue,
        };

        if name.starts_with("__") {
            continue;
        }

        let kind = TypeKind::from_str(&full_type.kind);

        let fields = full_type
            .fields
            .as_ref()
            .map(|fs| {
                fs.iter()
                    .map(|f| SchemaField {
                        name: f.name.clone(),
                        description: f.description.clone(),
                        args: f
                            .args
                            .iter()
                            .map(|a| SchemaArg {
                                name: a.name.clone(),
                                description: a.description.clone(),
                                arg_type: render_type_ref(&a.input_type),
                                default_value: a.default_value.clone(),
                            })
                            .collect(),
                        return_type: render_type_ref(&f.field_type),
                        is_deprecated: f.is_deprecated,
                        deprecation_reason: f.deprecation_reason.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let input_fields = full_type
            .input_fields
            .as_ref()
            .map(|ifs| {
                ifs.iter()
                    .map(|i| SchemaInputField {
                        name: i.name.clone(),
                        description: i.description.clone(),
                        field_type: render_type_ref(&i.input_type),
                        default_value: i.default_value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let enum_values = full_type
            .enum_values
            .as_ref()
            .map(|evs| {
                evs.iter()
                    .map(|e| SchemaEnumValue {
                        name: e.name.clone(),
                        description: e.description.clone(),
                        is_deprecated: e.is_deprecated,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let interfaces = full_type
            .interfaces
            .as_ref()
            .map(|is| is.iter().filter_map(|i| i.name.clone()).collect())
            .unwrap_or_default();

        let possible_types = full_type
            .possible_types
            .as_ref()
            .map(|pts| pts.iter().filter_map(|pt| pt.name.clone()).collect())
            .unwrap_or_default();

        schema.types.push(SchemaType {
            kind,
            name,
            description: full_type.description.clone(),
            fields,
            input_fields,
            enum_values,
            interfaces,
            possible_types,
        });
    }

    schema.types.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(schema)
}

fn render_type_ref(type_ref: &TypeRef) -> String {
    match &type_ref.name {
        Some(name) => {
            // Check if this is a wrapped type (NonNull, List)
            if let Some(of_type) = &type_ref.of_type {
                let inner = render_type_ref(of_type);
                match type_ref.kind.as_deref() {
                    Some("NON_NULL") => format!("{inner}!"),
                    Some("LIST") => format!("[{inner}]"),
                    _ => inner,
                }
            } else {
                name.clone()
            }
        }
        None => {
            // No name means it's a wrapper type, recurse into ofType
            if let Some(of_type) = &type_ref.of_type {
                let inner = render_type_ref(of_type);
                match type_ref.kind.as_deref() {
                    Some("NON_NULL") => format!("{inner}!"),
                    Some("LIST") => format!("[{inner}]"),
                    _ => inner,
                }
            } else {
                "Unknown".to_string()
            }
        }
    }
}

#[allow(dead_code)]
pub fn get_type_by_name<'a>(schema: &'a GraphQLSchema, name: &str) -> Option<&'a SchemaType> {
    schema.types.iter().find(|t| t.name == name)
}

#[allow(dead_code)]
pub fn get_query_type(schema: &GraphQLSchema) -> Option<&SchemaType> {
    schema
        .query_type
        .as_ref()
        .and_then(|name| get_type_by_name(schema, name))
}

#[allow(dead_code)]
pub fn get_mutation_type(schema: &GraphQLSchema) -> Option<&SchemaType> {
    schema
        .mutation_type
        .as_ref()
        .and_then(|name| get_type_by_name(schema, name))
}

#[allow(dead_code)]
pub fn get_subscription_type(schema: &GraphQLSchema) -> Option<&SchemaType> {
    schema
        .subscription_type
        .as_ref()
        .and_then(|name| get_type_by_name(schema, name))
}

#[allow(dead_code)]
pub fn get_autocomplete_suggestions(
    schema: &GraphQLSchema,
    query: &str,
    cursor_offset: usize,
) -> Vec<String> {
    let mut suggestions = Vec::new();
    // Find a valid char boundary at or before cursor_offset
    let safe_offset = if cursor_offset <= query.len() {
        let mut offset = cursor_offset;
        while offset > 0 && !query.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    } else {
        query.len()
    };
    let before_cursor = &query[..safe_offset];

    let last_word = before_cursor
        .rsplit_once(|c: char| c.is_whitespace() || c == '{' || c == '(' || c == ':')
        .map_or(before_cursor, |(_, w)| w);

    if let Some(query_type) = get_query_type(schema) {
        for field in &query_type.fields {
            if field.name.starts_with(last_word) && !suggestions.contains(&field.name) {
                suggestions.push(field.name.clone());
            }
        }
    }

    if let Some(mutation_type) = get_mutation_type(schema) {
        for field in &mutation_type.fields {
            if field.name.starts_with(last_word) && !suggestions.contains(&field.name) {
                suggestions.push(field.name.clone());
            }
        }
    }

    for schema_type in &schema.types {
        if schema_type.name.starts_with(last_word) && !suggestions.contains(&schema_type.name) {
            suggestions.push(schema_type.name.clone());
        }
        for field in &schema_type.fields {
            if field.name.starts_with(last_word) && !suggestions.contains(&field.name) {
                suggestions.push(field.name.clone());
            }
        }
    }

    suggestions.sort();
    suggestions.truncate(20);
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_kind_from_str() {
        assert_eq!(TypeKind::from_str("SCALAR"), TypeKind::Scalar);
        assert_eq!(TypeKind::from_str("OBJECT"), TypeKind::Object);
        assert_eq!(TypeKind::from_str("INTERFACE"), TypeKind::Interface);
        assert_eq!(TypeKind::from_str("UNION"), TypeKind::Union);
        assert_eq!(TypeKind::from_str("ENUM"), TypeKind::Enum);
        assert_eq!(TypeKind::from_str("INPUT_OBJECT"), TypeKind::InputObject);
        assert_eq!(TypeKind::from_str("UNKNOWN"), TypeKind::Scalar);
    }

    #[test]
    fn parse_introspection_minimal() {
        let json = r#"{
            "data": {
                "__schema": {
                    "queryType": {"name": "Query"},
                    "mutationType": {"name": "Mutation"},
                    "subscriptionType": null,
                    "types": [],
                    "directives": []
                }
            }
        }"#;
        let response: IntrospectionResponse = serde_json::from_str(json).unwrap();
        let schema = parse_introspection_response(&response).unwrap();
        assert_eq!(schema.query_type.as_deref(), Some("Query"));
        assert_eq!(schema.mutation_type.as_deref(), Some("Mutation"));
        assert!(schema.subscription_type.is_none());
        assert!(schema.types.is_empty());
    }

    #[test]
    fn parse_introspection_with_types() {
        let json = r#"{
            "data": {
                "__schema": {
                    "queryType": {"name": "Query"},
                    "types": [
                        {
                            "kind": "OBJECT",
                            "name": "Query",
                            "description": "Root query type",
                            "fields": [
                                {
                                    "name": "users",
                                    "description": "Get all users",
                                    "args": [
                                        {
                                            "name": "limit",
                                            "description": "Max results",
                                            "type": {"name": "Int"},
                                            "defaultValue": "10"
                                        }
                                    ],
                                    "type": {"name": "User"},
                                    "isDeprecated": false,
                                    "deprecationReason": null
                                }
                            ],
                            "inputFields": null,
                            "interfaces": [],
                            "enumValues": null,
                            "possibleTypes": null
                        },
                        {
                            "kind": "SCALAR",
                            "name": "String",
                            "description": null,
                            "fields": null,
                            "inputFields": null,
                            "interfaces": null,
                            "enumValues": null,
                            "possibleTypes": null
                        }
                    ],
                    "directives": []
                }
            }
        }"#;
        let response: IntrospectionResponse = serde_json::from_str(json).unwrap();
        let schema = parse_introspection_response(&response).unwrap();
        assert_eq!(schema.types.len(), 2);

        let query_type = &schema.types[0];
        assert_eq!(query_type.name, "Query");
        assert_eq!(query_type.kind, TypeKind::Object);
        assert_eq!(query_type.fields.len(), 1);
        assert_eq!(query_type.fields[0].name, "users");
        assert_eq!(query_type.fields[0].args.len(), 1);
        assert_eq!(query_type.fields[0].args[0].name, "limit");
    }

    #[test]
    fn parse_introspection_errors() {
        let json = r#"{
            "data": null,
            "errors": [{"message": "Cannot query field 'invalid'"}]
        }"#;
        let response: IntrospectionResponse = serde_json::from_str(json).unwrap();
        let result = parse_introspection_response(&response);
        assert!(result.is_err());
    }

    #[test]
    fn get_type_by_name_works() {
        let mut schema = GraphQLSchema::default();
        schema.types.push(SchemaType {
            kind: TypeKind::Object,
            name: "User".to_string(),
            description: None,
            fields: vec![],
            input_fields: vec![],
            enum_values: vec![],
            interfaces: vec![],
            possible_types: vec![],
        });

        assert!(get_type_by_name(&schema, "User").is_some());
        assert!(get_type_by_name(&schema, "Post").is_none());
    }

    #[test]
    fn autocomplete_suggestions() {
        let mut schema = GraphQLSchema {
            query_type: Some("Query".to_string()),
            ..Default::default()
        };
        schema.types.push(SchemaType {
            kind: TypeKind::Object,
            name: "Query".to_string(),
            description: None,
            fields: vec![
                SchemaField {
                    name: "users".to_string(),
                    description: None,
                    args: vec![],
                    return_type: "[User]".to_string(),
                    is_deprecated: false,
                    deprecation_reason: None,
                },
                SchemaField {
                    name: "posts".to_string(),
                    description: None,
                    args: vec![],
                    return_type: "[Post]".to_string(),
                    is_deprecated: false,
                    deprecation_reason: None,
                },
            ],
            input_fields: vec![],
            enum_values: vec![],
            interfaces: vec![],
            possible_types: vec![],
        });

        let suggestions = get_autocomplete_suggestions(&schema, "{ u", 3);
        assert!(suggestions.contains(&"users".to_string()));

        let suggestions = get_autocomplete_suggestions(&schema, "{ p", 3);
        assert!(suggestions.contains(&"posts".to_string()));
    }
}
