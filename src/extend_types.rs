use crate::state::{GraphQL, GraphQLType};

use graphql_parser::schema;

/// Convert Text to String.
/// See https://github.com/graphql-rust/graphql-parser/blob/master/src/common.rs#L12-L28
fn convert_text_to_string<'a, T>(text: &T::Value) -> String
where
    T: schema::Text<'a>,
{
    String::from(text.as_ref())
}

/// Extend id for type extensions.
/// Only used internally to distinguish between a type and its extension.
fn get_extended_id(id: String) -> String { format!("{}__", id) }

/// Extract dependencies from any entity's directives.
fn get_dependencies_from_directives<'a, T>(directives: &[schema::Directive<'a, T>]) -> Vec<String>
where
    T: schema::Text<'a>,
{
    directives
        .iter()
        .map(|directive| convert_text_to_string::<T>(&directive.name))
        .collect::<Vec<String>>()
}

/// Recursively walk a field to get the dependencies.
fn walk_field<'a, T>(field: &schema::Field<'a, T>) -> Vec<String>
where
    T: schema::Text<'a>,
{
    field
        // Inject arguments.
        .arguments
        .iter()
        .map(|argument| walk_field_type(&argument.value_type))
        // Inject directives.
        .chain(get_dependencies_from_directives(&field.directives))
        // Inject field type.
        .chain(vec![walk_field_type(&field.field_type)])
        .collect::<Vec<String>>()
}

/// Recursively walk a field type to get the inner String value.
fn walk_field_type<'a, T>(field_type: &schema::Type<'a, T>) -> String
where
    T: schema::Text<'a>,
{
    match field_type {
        schema::Type::NamedType(name) => convert_text_to_string::<T>(name),
        schema::Type::ListType(field_type) => {
            // Field type is boxed, need to unbox.
            walk_field_type(field_type.as_ref())
        }
        schema::Type::NonNullType(field_type) => {
            // Same here.
            walk_field_type(field_type.as_ref())
        }
    }
}

/// Recursively walk an input to get the dependencies.
fn walk_input_value<'a, T>(input_value: &schema::InputValue<'a, T>) -> Vec<String>
where
    T: schema::Text<'a>,
{
    get_dependencies_from_directives(&input_value.directives)
        .into_iter()
        .chain(vec![walk_field_type(&input_value.value_type)])
        .collect::<Vec<String>>()
}

pub trait ExtendType {
    fn get_dependencies(&self) -> Vec<String>;
    fn get_id_and_name(&self) -> (Option<String>, String);
    fn get_mapped_type(&self) -> GraphQL;
    fn get_raw(&self) -> String;
}

impl<'a, T> ExtendType for schema::EnumType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get root directives.
        get_dependencies_from_directives(&self.directives)
            .into_iter()
            // Get values' directives.
            .chain(
                self.values
                    .iter()
                    .map(|enum_value| get_dependencies_from_directives(&enum_value.directives))
                    .flatten(),
            )
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::Enum) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::EnumTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get root directives.
        get_dependencies_from_directives(&self.directives)
            .into_iter()
            // Get values' directives.
            .chain(
                self.values
                    .iter()
                    .map(|enum_value| get_dependencies_from_directives(&enum_value.directives))
                    .flatten(),
            )
            // Add extension's source.
            .chain(vec![convert_text_to_string::<T>(&self.name)])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::Enum) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::InputObjectType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|input_value| walk_input_value(input_value))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::InputObject) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::InputObjectTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|input_value| walk_input_value(input_value))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            // Add extension's source.
            .chain(vec![convert_text_to_string::<T>(&self.name)])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::InputObject) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::InterfaceType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|field| walk_field(field))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::Interface) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::InterfaceTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|field| walk_field(field))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            // Add extension's source.
            .chain(vec![convert_text_to_string::<T>(&self.name)])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::Interface) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::ObjectType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|field| walk_field(field))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            // Get interfaces as dependencies.
            .chain(
                self.implements_interfaces
                    .iter()
                    .map(|directive| convert_text_to_string::<T>(&directive)),
            )
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::Object) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::ObjectTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get fields' dependencies.
        self.fields
            .iter()
            .map(|field| walk_field(field))
            .flatten()
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            // Get interfaces as dependencies.
            .chain(
                self.implements_interfaces
                    .iter()
                    .map(|directive| convert_text_to_string::<T>(&directive)),
            )
            // Add extension's source.
            .chain(vec![String::from(self.name.as_ref())])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::Object) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::ScalarType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get root directives.
        get_dependencies_from_directives(&self.directives)
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::Scalar) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::ScalarTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get root directives.
        get_dependencies_from_directives(&self.directives)
            .into_iter()
            // Add extension's source.
            .chain(vec![convert_text_to_string::<T>(&self.name)])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::Scalar) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::UnionType<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get types as dependencies.
        self.types
            .iter()
            .map(|inner_type| convert_text_to_string::<T>(&inner_type))
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeDefinition(GraphQLType::Union) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::UnionTypeExtension<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // Get types as dependencies.
        self.types
            .iter()
            .map(|inner_type| convert_text_to_string::<T>(&inner_type))
            // Get root directives.
            .chain(get_dependencies_from_directives(&self.directives))
            // Add extension's source.
            .chain(vec![convert_text_to_string::<T>(&self.name)])
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (Some(get_extended_id(name.clone())), name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::TypeExtension(GraphQLType::Union) }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::SchemaDefinition<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        // A schema can only have a query, a mutation and a subscription.
        vec![&self.query, &self.mutation, &self.subscription]
            .into_iter()
            .filter_map(|field| match field {
                Some(field) => Some(convert_text_to_string::<T>(&field)),
                None => None,
            })
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        // A Schema has no name, use a default one.
        (None, String::from("schema"))
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::Schema }
    fn get_raw(&self) -> String { self.to_string() }
}

impl<'a, T> ExtendType for schema::DirectiveDefinition<'a, T>
where
    T: schema::Text<'a>,
{
    fn get_dependencies(&self) -> Vec<String> {
        self.arguments
            .iter()
            .map(|input_value| walk_input_value(input_value))
            .flatten()
            .collect::<Vec<String>>()
    }
    fn get_id_and_name(&self) -> (Option<String>, String) {
        let name = convert_text_to_string::<T>(&self.name);
        (None, name)
    }
    fn get_mapped_type(&self) -> GraphQL { GraphQL::Directive }
    fn get_raw(&self) -> String { self.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    use graphql_parser::parse_schema;

    fn match_and_assert(
        contents: &str,
        dependencies: Vec<&str>,
        id_and_name: (Option<String>, String),
        mapped_type: GraphQL,
    ) {
        fn assert(
            schema_type: impl ExtendType,
            dependencies: Vec<&str>,
            id_and_name: (Option<String>, String),
            mapped_type: GraphQL,
            raw: String,
        ) {
            assert_eq!(schema_type.get_dependencies(), dependencies);
            assert_eq!(schema_type.get_id_and_name(), id_and_name);
            assert_eq!(schema_type.get_mapped_type(), mapped_type);
            assert_eq!(schema_type.get_raw(), raw);
        };

        let document = parse_schema::<String>(contents).unwrap().to_owned();

        match document.definitions.get(0).unwrap().to_owned() {
            schema::Definition::TypeDefinition(type_definition) => match type_definition {
                schema::TypeDefinition::Enum(enum_type) => {
                    assert(
                        enum_type,
                        dependencies,
                        id_and_name,
                        mapped_type,
                        document.to_string(),
                    );
                }
                schema::TypeDefinition::Scalar(_) => {}
                schema::TypeDefinition::Object(_) => {}
                schema::TypeDefinition::Interface(_) => {}
                schema::TypeDefinition::Union(_) => {}
                schema::TypeDefinition::InputObject(_) => {}
            },

            schema::Definition::SchemaDefinition(_) => {}
            schema::Definition::TypeExtension(type_extension) => match type_extension {
                schema::TypeExtension::Enum(enum_type_extension) => assert(
                    enum_type_extension,
                    dependencies,
                    id_and_name,
                    mapped_type,
                    document.to_string(),
                ),
                schema::TypeExtension::Scalar(_) => {}
                schema::TypeExtension::Object(_) => {}
                schema::TypeExtension::Interface(_) => {}
                schema::TypeExtension::Union(_) => {}
                schema::TypeExtension::InputObject(_) => {}
            },
            schema::Definition::DirectiveDefinition(_) => {}
        };
    }

    #[test]
    fn test_enum() {
        match_and_assert(
            "enum Foo @foo { A @bar B C}",
            vec!["foo", "bar"],
            (None, String::from("Foo")),
            GraphQL::TypeDefinition(GraphQLType::Enum),
        );
    }

    #[test]
    fn test_extend_enum() {
        match_and_assert(
            "extend enum Foo @foo { D @bar }",
            vec!["foo", "bar", "Foo"],
            (Some(String::from("Foo__")), String::from("Foo")),
            GraphQL::TypeExtension(GraphQLType::Enum),
        );
    }
}
