use graphql_client::GraphQLQuery;

// TODO: Change to be better representation?
type Map = std::collections::HashMap<String, String>;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/queries.graphql",
    response_derives = "Debug, PartialEq",
    normalization = "rust"
)]
pub struct PartsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
pub struct CreateUserMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
pub struct LoginUserMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
pub struct CreatePartMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graph/schema.graphql",
    query_path = "graph/client/mutations.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
pub struct UpdatePartMutation;
